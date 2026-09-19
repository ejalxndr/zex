use std::collections::{HashMap, VecDeque};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use gpui::{
    AnyElement, Context, Global, ObjectFit, Pixels, RenderImage, StyledImage, img, prelude::*,
};
use image::ImageDecoder;
use image::metadata::Orientation;

use crate::explorer::Explorer;
use crate::filesystem::entry::FsEntry;

const THUMBNAIL_MAX_EDGE: u32 = 64;
const MAX_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DECODE_ALLOC: u64 = 256 * 1024 * 1024;
const CACHE_CAPACITY: usize = 512;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "jfif", "gif", "bmp", "webp", "ico", "tiff", "tif", "qoi", "tga",
];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .is_some_and(|ext| SUPPORTED_EXTENSIONS.contains(&ext.as_str()))
}

enum State {
    Pending,
    Ready(Arc<RenderImage>),
    Failed,
}

struct Record {
    modified: Option<SystemTime>,
    state: State,
}

pub struct ThumbnailCache {
    enabled: bool,
    records: HashMap<PathBuf, Record>,
    order: VecDeque<PathBuf>,
}

impl Global for ThumbnailCache {}

impl ThumbnailCache {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            records: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    fn needs_fetch(&self, path: &Path, modified: Option<SystemTime>) -> bool {
        match self.records.get(path) {
            Some(record) => record.modified != modified,
            None => true,
        }
    }

    fn mark_pending(&mut self, path: PathBuf, modified: Option<SystemTime>) {
        if self
            .records
            .insert(
                path.clone(),
                Record {
                    modified,
                    state: State::Pending,
                },
            )
            .is_none()
        {
            self.order.push_back(path);
        }
        self.evict_if_needed();
    }

    fn store(
        &mut self,
        path: PathBuf,
        modified: Option<SystemTime>,
        image: Option<Arc<RenderImage>>,
    ) {
        let state = match image {
            Some(image) => State::Ready(image),
            None => State::Failed,
        };
        if self
            .records
            .insert(path.clone(), Record { modified, state })
            .is_none()
        {
            self.order.push_back(path);
        }
        self.evict_if_needed();
    }

    fn evict_if_needed(&mut self) {
        let mut budget = self.order.len();
        while self.records.len() > CACHE_CAPACITY && budget > 0 {
            budget -= 1;
            let Some(candidate) = self.order.pop_front() else {
                break;
            };
            if matches!(
                self.records.get(&candidate).map(|record| &record.state),
                Some(State::Pending)
            ) {
                self.order.push_back(candidate);
                continue;
            }
            self.records.remove(&candidate);
        }
    }
}

enum Lookup {
    Ready(Arc<RenderImage>),
    Pending,
    Failed,
    Missing,
}

fn lookup(cache: &ThumbnailCache, path: &Path, modified: Option<SystemTime>) -> Lookup {
    match cache.records.get(path) {
        Some(record) if record.modified == modified => match &record.state {
            State::Ready(image) => Lookup::Ready(image.clone()),
            State::Pending => Lookup::Pending,
            State::Failed => Lookup::Failed,
        },
        _ => Lookup::Missing,
    }
}

pub fn icon_element(
    entry: &FsEntry,
    size: Pixels,
    cx: &mut Context<Explorer>,
) -> Option<AnyElement> {
    if entry.is_dir || entry.is_symlink {
        return None;
    }
    if !cx.global::<ThumbnailCache>().enabled || !is_supported_image(&entry.path) {
        return None;
    }

    match lookup(cx.global::<ThumbnailCache>(), &entry.path, entry.modified) {
        Lookup::Ready(image) => Some(
            img(image)
                .size(size)
                .flex_shrink_0()
                .rounded_sm()
                .overflow_hidden()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
        ),
        Lookup::Pending | Lookup::Failed => None,
        Lookup::Missing => {
            request(entry.path.clone(), entry.modified, cx);
            None
        }
    }
}

fn request(path: PathBuf, modified: Option<SystemTime>, cx: &mut Context<Explorer>) {
    if !cx.global::<ThumbnailCache>().needs_fetch(&path, modified) {
        return;
    }
    cx.global_mut::<ThumbnailCache>()
        .mark_pending(path.clone(), modified);

    let decode_path = path.clone();
    cx.spawn(async move |explorer, cx| {
        let image = cx
            .background_spawn(async move { decode(&decode_path) })
            .await;
        let stored = cx
            .update_global::<ThumbnailCache, _>(|cache, _| {
                cache.store(path, modified, image);
            })
            .is_ok();
        if stored {
            explorer.update(cx, |_, cx| cx.notify()).ok();
        }
    })
    .detach();
}

fn decode(path: &Path) -> Option<Arc<RenderImage>> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_SOURCE_BYTES {
        return None;
    }

    let mut reader = image::ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);

    let mut decoder = reader.into_decoder().ok()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut source = image::DynamicImage::from_decoder(decoder).ok()?;
    source.apply_orientation(orientation);

    let mut thumbnail = source
        .thumbnail(THUMBNAIL_MAX_EDGE, THUMBNAIL_MAX_EDGE)
        .into_rgba8();
    if thumbnail.width() == 0 || thumbnail.height() == 0 {
        return None;
    }
    for pixel in thumbnail.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }

    Some(Arc::new(RenderImage::new(vec![image::Frame::new(
        thumbnail,
    )])))
}
