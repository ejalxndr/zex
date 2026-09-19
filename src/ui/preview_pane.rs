use gpui::{
    AnyElement, Context, DragMoveEvent, MouseButton, MouseUpEvent, ObjectFit, Render, StyledImage,
    Window, div, img, prelude::*, px,
};

use crate::explorer::Explorer;
use crate::filesystem::entry::format_size;
use crate::theme;
use crate::ui::thumbnail::ThumbnailCache;

pub const DEFAULT_WIDTH: f32 = 360.0;
pub const MIN_WIDTH: f32 = 240.0;
pub const MAX_WIDTH: f32 = 820.0;

const HEADER_HEIGHT: f32 = 40.0;
const FOOTER_HEIGHT: f32 = 34.0;
const RESIZE_HIT_WIDTH: f32 = 14.0;

struct PreviewResizeGhost;

impl Render for PreviewResizeGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub fn render(explorer: &mut Explorer, cx: &Context<Explorer>) -> Option<AnyElement> {
    if !cx.global::<ThumbnailCache>().enabled() {
        return None;
    }

    let path = explorer.preview_target()?;
    let width = explorer.preview_width;
    let resizing = explorer.preview_resize_drag.is_some();

    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size_text = explorer
        .entries
        .iter()
        .find(|entry| entry.path == path)
        .map(|entry| format_size(entry.size))
        .unwrap_or_default();

    Some(
        div()
            .relative()
            .flex()
            .flex_col()
            .h_full()
            .w(px(width))
            .flex_shrink_0()
            .border_l_1()
            .border_color(theme::border())
            .bg(theme::bg_panel())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .h(px(HEADER_HEIGHT))
                    .px_5()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(theme::border())
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(theme::text_primary())
                            .child(name),
                    )
                    .child(
                        div()
                            .id("close-preview")
                            .flex_shrink_0()
                            .cursor_pointer()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .text_color(theme::text_muted())
                            .hover(|style| {
                                style
                                    .bg(theme::bg_hover())
                                    .text_color(theme::text_primary())
                            })
                            .on_click(cx.listener(|explorer, _event, _window, cx| {
                                explorer.dismiss_preview(cx);
                            }))
                            .child("×"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .p_6()
                    .overflow_hidden()
                    .child(
                        img(path)
                            .size_full()
                            .object_fit(ObjectFit::Contain)
                            .with_fallback(|| {
                                div()
                                    .text_color(theme::text_muted())
                                    .text_size(px(12.0))
                                    .child("Preview unavailable")
                                    .into_any_element()
                            }),
                    ),
            )
            .when(!size_text.is_empty(), |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .h(px(FOOTER_HEIGHT))
                        .px_5()
                        .flex()
                        .items_center()
                        .border_t_1()
                        .border_color(theme::border())
                        .text_color(theme::text_muted())
                        .text_size(px(11.0))
                        .child(size_text),
                )
            })
            .child(resize_handle(resizing, cx))
            .into_any_element(),
    )
}

fn resize_handle(active: bool, cx: &Context<Explorer>) -> impl IntoElement {
    let entity = cx.entity();

    div()
        .id("preview-resize-handle")
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(-RESIZE_HIT_WIDTH / 2.0))
        .w(px(RESIZE_HIT_WIDTH))
        .occlude()
        .flex()
        .justify_center()
        .cursor_col_resize()
        .child(
            div()
                .w(px(1.0))
                .h_full()
                .when(active, |bar| bar.bg(theme::bg_selected())),
        )
        .on_drag((), move |_, _point, window, cx| {
            let anchor_x = f32::from(window.mouse_position().x);
            entity.update(cx, |explorer, cx| {
                explorer.begin_preview_resize(anchor_x, cx);
            });
            cx.new(|_| PreviewResizeGhost)
        })
        .on_drag_move::<()>(
            cx.listener(move |explorer, event: &DragMoveEvent<()>, _window, cx| {
                explorer.update_preview_resize(f32::from(event.event.position.x), cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                explorer.end_preview_resize(cx);
            }),
        )
}
