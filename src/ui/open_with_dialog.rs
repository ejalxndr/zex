use gpui::{
    AnyElement, Context, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, StatefulInteractiveElement, Styled, div, img, px, svg,
};

use crate::explorer::Explorer;
use crate::filesystem::open_with::AppIcon;
use crate::theme;

fn app_icon(icon: &Option<AppIcon>) -> AnyElement {
    match icon {
        Some(AppIcon::Raster(path)) => img(path.clone()).size(px(16.0)).flex_shrink_0().into_any_element(),
        Some(AppIcon::Svg(path)) => svg()
            .path(path.to_string_lossy().into_owned())
            .size(px(16.0))
            .flex_shrink_0()
            .text_color(theme::text_muted())
            .into_any_element(),
        None => div().size(px(16.0)).flex_shrink_0().into_any_element(),
    }
}

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> Option<impl IntoElement> {
    let state = explorer.open_with.as_ref()?;
    let name = state
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| state.path.display().to_string());

    let body: gpui::AnyElement = if state.loading {
        div()
            .py_2()
            .text_color(theme::text_muted())
            .child("Looking for applications…")
            .into_any_element()
    } else if state.apps.is_empty() {
        div()
            .py_2()
            .text_color(theme::text_muted())
            .child("No applications found for this file type.")
            .into_any_element()
    } else {
        div()
            .id("open-with-app-list")
            .flex()
            .flex_col()
            .max_h(px(280.0))
            .overflow_scroll()
            .children(state.apps.iter().enumerate().map(|(ix, app)| {
                let target_app = app.clone();
                div()
                    .id(("open-with-app", ix))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(theme::text_primary())
                    .hover(|style| style.bg(theme::bg_hover()))
                    .on_click(cx.listener(move |explorer, _, window, cx| {
                        explorer.launch_with_app(target_app.clone(), window, cx);
                    }))
                    .child(app_icon(&app.icon))
                    .child(app.name.clone())
            }))
            .into_any_element()
    };

    Some(
        div()
            .id("open-with-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(Hsla { h: 0., s: 0., l: 0., a: 0.5 })
            .child(
                div()
                    .id("open-with-panel")
                    .on_mouse_down_out(cx.listener(|explorer, _, window, cx| {
                        explorer.cancel_open_with(window, cx);
                    }))
                    .on_mouse_down(MouseButton::Left, |_, _window, cx| cx.stop_propagation())
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(320.0))
                    .p_4()
                    .bg(theme::bg_elevated())
                    .border_1()
                    .border_color(theme::border())
                    .shadow(theme::elevated_shadow())
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme::text_primary())
                            .child(format!("Open \u{201c}{name}\u{201d} With")),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .child(
                                div()
                                    .id("open-with-cancel")
                                    .cursor_pointer()
                                    .px_3()
                                    .py_1()
                                    .border_1()
                                    .border_color(theme::border())
                                    .text_color(theme::text_primary())
                                    .hover(|style| style.bg(theme::bg_hover()))
                                    .on_click(cx.listener(|explorer, _, window, cx| {
                                        explorer.cancel_open_with(window, cx);
                                    }))
                                    .child("Cancel"),
                            ),
                    ),
            ),
    )
}
