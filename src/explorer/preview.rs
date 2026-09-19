use std::path::PathBuf;

use gpui::Context;

use crate::ui::preview_pane::{MAX_WIDTH, MIN_WIDTH};
use crate::ui::thumbnail;

use super::Explorer;
use super::drag::WidthResizeDrag;

impl Explorer {
    fn single_selection(&self) -> Option<PathBuf> {
        if self.selected.len() == 1 {
            self.selected.iter().next().cloned()
        } else {
            None
        }
    }

    pub fn preview_target(&mut self) -> Option<PathBuf> {
        let single = self.single_selection();

        if self.preview_dismissed.is_some() && self.preview_dismissed != single {
            self.preview_dismissed = None;
        }

        let path = single?;
        if self.preview_dismissed.as_ref() == Some(&path) {
            return None;
        }

        let entry = self.entries.iter().find(|entry| entry.path == path)?;
        if entry.is_dir || entry.is_broken_symlink {
            return None;
        }

        thumbnail::is_supported_image(&path).then_some(path)
    }

    pub fn dismiss_preview(&mut self, cx: &mut Context<Self>) {
        self.preview_dismissed = self.single_selection();
        cx.notify();
    }

    pub fn begin_preview_resize(&mut self, anchor_x: f32, cx: &mut Context<Self>) {
        self.preview_resize_drag = Some(WidthResizeDrag {
            anchor_x,
            start_width: self.preview_width,
        });
        cx.notify();
    }

    pub fn update_preview_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let Some(drag) = self.preview_resize_drag else {
            return;
        };
        let new_width = drag.start_width + (drag.anchor_x - current_x);
        self.preview_width = new_width.clamp(MIN_WIDTH, MAX_WIDTH);
        cx.notify();
    }

    pub fn end_preview_resize(&mut self, cx: &mut Context<Self>) {
        if self.preview_resize_drag.take().is_some() {
            cx.notify();
        }
    }
}
