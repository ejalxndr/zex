use std::ops::Range;
use std::path::PathBuf;

use gpui::{
    ClickEvent, Context, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder as _, px, svg, uniform_list,
};

use crate::explorer::Explorer;
use crate::explorer::drag::ScrollbarId;
use crate::explorer::search::{SearchResults, SearchState, SearchStatus};
use crate::filesystem::entry::FsEntry;
use crate::search::SearchScope;
use crate::settings::CaseSensitivity;
use crate::theme;
use crate::theme::icon_theme;
use crate::ui::context_menu;
use crate::ui::path_bar::{SuggestionDown, SuggestionUp};
use crate::ui::popup_menu::ContextMenuExt;
use crate::ui::scrollbar::Scrollbar;
use crate::ui::text_input::{Escape, SubmitEnter, TextInput as Input};
use crate::ui::tooltip::Tooltip;
use crate::ui::{bulk_progress, open_with_dialog, warning_dialog};

enum SearchRow {
    FileHeader { path: PathBuf, count: usize },
    Match(usize),
    Name(usize),
}

pub fn render(explorer: &Explorer, window: &mut Window, cx: &Context<Explorer>) -> impl IntoElement {
    let state = explorer.search.as_ref().unwrap();
    let entity = cx.entity();

    div()
        .id("search-root")
        .size_full()
        .flex()
        .flex_col()
        .bg(theme::bg_root())
        .text_color(theme::text_primary())
        .child(render_header(explorer, state, cx))
        .children(explorer.op_error.as_ref().map(|message| {
            div()
                .relative()
                .w_full()
                .flex_shrink_0()
                .py_2()
                .pl_3()
                .pr_8()
                .bg(theme::bg_on_error())
                .border_1()
                .border_color(theme::text_error())
                .text_color(theme::text_on_error())
                .child(div().whitespace_normal().child(message.clone()))
                .child(
                    div()
                        .id("dismiss-search-op-error")
                        .absolute()
                        .top_1()
                        .right_1()
                        .cursor_pointer()
                        .px_2()
                        .on_click(cx.listener(|explorer, _, _, cx| explorer.dismiss_op_error(cx)))
                        .child("×"),
                )
        }))
        .child(render_body(explorer, state, entity, window, cx))
        .children(warning_dialog::render(explorer, cx))
        .children(bulk_progress::render(explorer, cx))
        .children(open_with_dialog::render(explorer, cx))
}

fn render_header(_explorer: &Explorer, state: &SearchState, cx: &Context<Explorer>) -> impl IntoElement {
    let total = state.results.len();
    let truncated_note = state.truncated.then(|| {
        div()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .pb_1()
            .text_size(px(11.0))
            .text_color(theme::text_faint())
            .child(format!("Showing first {total} — refine your search for more."))
    });

    div()
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(theme::border())
        .bg(theme::bg_root())
        .child(
            div()
                .w_full()
                .flex_shrink_0()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py_2()
                .child(scope_segment(state, cx))
                .child(input_pill(state, cx))
                .child(nav_controls(state, cx)),
        )
        .children(truncated_note)
}

fn scope_segment(state: &SearchState, cx: &Context<Explorer>) -> impl IntoElement {
    let name_active = state.scope == SearchScope::Names;
    let contents_active = state.scope == SearchScope::Contents;

    let name_icon = tinted_generic_file_icon(px(14.0), name_active, cx);
    let contents_icon = svg()
        .flex_shrink_0()
        .path("icons/text.svg")
        .size(px(14.0))
        .text_color(if contents_active { theme::box_select_border() } else { theme::text_faint() })
        .into_any_element();

    div()
        .flex_shrink_0()
        .flex()
        .flex_row()
        .gap_1()
        .child(scope_icon_button(
            "search-scope-name",
            SearchScope::Names,
            "File name",
            name_icon,
            cx,
        ))
        .child(scope_icon_button(
            "search-scope-text",
            SearchScope::Contents,
            "File content",
            contents_icon,
            cx,
        ))
}

fn scope_icon_button(
    id: impl Into<ElementId>,
    scope: SearchScope,
    tooltip_text: &'static str,
    icon: gpui::AnyElement,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    div()
        .id(id.into())
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .size(px(24.0))
        .rounded_md()
        .hover(|style| style.bg(theme::bg_hover()))
        .on_click(cx.listener(move |explorer, _, _, cx| explorer.set_search_scope(scope, cx)))
        .tooltip(Tooltip::build(tooltip_text))
        .child(icon)
}

fn tinted_generic_file_icon(size: gpui::Pixels, active: bool, cx: &gpui::App) -> gpui::AnyElement {
    let icon = cx.global::<icon_theme::IconThemeState>().generic_file_icon();
    let color = if active { theme::box_select_border() } else { theme::text_faint() };
    match icon.mode {
        icon_theme::IconRenderMode::Tinted => svg()
            .path(icon.path.to_string_lossy().into_owned())
            .size(size)
            .flex_shrink_0()
            .text_color(color)
            .into_any_element(),
        icon_theme::IconRenderMode::FullColor => {
            gpui::img(icon.path).size(size).flex_shrink_0().into_any_element()
        }
    }
}

fn input_pill(state: &SearchState, cx: &Context<Explorer>) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .border_1()
        .border_color(theme::border())
        .rounded_md()
        .bg(theme::bg_root())
        .pl_2()
        .pr_1()
        .py(px(6.0))
        .on_action(cx.listener(|explorer, _: &Escape, window, cx| explorer.cancel_search(window, cx)))
        .on_action(cx.listener(|explorer, _: &SubmitEnter, window, cx| {
            explorer.reveal_selected_result(window, cx)
        }))
        .on_action(cx.listener(|explorer, _: &SuggestionDown, _window, cx| {
            explorer.select_next_result(cx)
        }))
        .on_action(cx.listener(|explorer, _: &SuggestionUp, _window, cx| explorer.select_prev_result(cx)))
        .child(div().flex_1().min_w(px(0.0)).child(Input::new(&state.input)))
        .child(icon_button(
            "search-opt-case",
            "Aa",
            case_tooltip(state.case),
            state.case == CaseSensitivity::Sensitive,
            |explorer, _, _, cx| explorer.cycle_search_case(cx),
            cx,
        ))
        .child(icon_button(
            "search-opt-word",
            "wd",
            "Whole word",
            state.whole_word,
            |explorer, _, _, cx| explorer.toggle_search_whole_word(cx),
            cx,
        ))
        .child(icon_button(
            "search-opt-regex",
            ".*",
            "Use regular expression",
            state.regex,
            |explorer, _, _, cx| explorer.toggle_search_regex(cx),
            cx,
        ))
        .child(vdivider())
        .child(icon_button(
            "search-opt-hidden",
            "h.",
            "Include hidden files",
            state.include_hidden,
            |explorer, _, _, cx| explorer.toggle_search_hidden(cx),
            cx,
        ))
        .child(icon_button(
            "search-opt-gitignore",
            "gi",
            "Respect .gitignore",
            state.respect_gitignore,
            |explorer, _, _, cx| explorer.toggle_search_gitignore(cx),
            cx,
        ))
}

fn case_tooltip(case: CaseSensitivity) -> &'static str {
    match case {
        CaseSensitivity::Sensitive => "Case: sensitive (click to cycle)",
        CaseSensitivity::Insensitive => "Case: insensitive (click to cycle)",
        CaseSensitivity::Smart => "Case: smart (click to cycle)",
    }
}

fn nav_controls(state: &SearchState, cx: &Context<Explorer>) -> impl IntoElement {
    let total = state.results.len();
    let current = state.selected_index.map(|ix| ix + 1).unwrap_or(0);
    let can_navigate = total > 0;
    let count_text = if state.status == SearchStatus::Searching {
        "…".to_string()
    } else {
        format!("{current}/{total}")
    };

    div()
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .child(nav_chevron(
            "search-prev",
            "‹",
            can_navigate,
            |explorer, _, _, cx| explorer.select_prev_result(cx),
            cx,
        ))
        .child(
            div()
                .min_w(px(36.0))
                .text_size(px(11.0))
                .text_center()
                .text_color(theme::text_muted())
                .child(count_text),
        )
        .child(nav_chevron(
            "search-next",
            "›",
            can_navigate,
            |explorer, _, _, cx| explorer.select_next_result(cx),
            cx,
        ))
}

fn nav_chevron<F>(
    id: impl Into<ElementId>,
    glyph: &'static str,
    enabled: bool,
    handler: F,
    cx: &Context<Explorer>,
) -> impl IntoElement
where
    F: Fn(&mut Explorer, &ClickEvent, &mut Window, &mut Context<Explorer>) + 'static,
{
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_center()
        .size(px(20.0))
        .rounded_md()
        .text_color(theme::text_muted())
        .when(enabled, |el| {
            el.cursor_pointer()
                .hover(|style| style.bg(theme::bg_hover()).text_color(theme::text_primary()))
                .on_click(cx.listener(handler))
        })
        .child(glyph)
}

fn vdivider() -> impl IntoElement {
    div().flex_shrink_0().w(px(1.0)).h(px(14.0)).bg(theme::border())
}

fn icon_button<F>(
    id: impl Into<ElementId>,
    glyph: impl Into<SharedString>,
    tooltip: &'static str,
    active: bool,
    handler: F,
    cx: &Context<Explorer>,
) -> impl IntoElement
where
    F: Fn(&mut Explorer, &ClickEvent, &mut Window, &mut Context<Explorer>) + 'static,
{
    div()
        .id(id.into())
        .flex_shrink_0()
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .h(px(20.0))
        .min_w(px(20.0))
        .px_1()
        .rounded(px(4.0))
        .text_size(px(11.0))
        .when(active, |el| el.text_color(theme::box_select_border()))
        .when(!active, |el| el.text_color(theme::text_faint()))
        .hover(|style| style.bg(theme::bg_hover()).text_color(theme::text_primary()))
        .on_click(cx.listener(handler))
        .tooltip(Tooltip::build(tooltip))
        .child(glyph.into())
}

fn build_rows(state: &SearchState) -> Vec<SearchRow> {
    match &state.results {
        SearchResults::Contents(items) => {
            let mut rows = Vec::new();
            let mut ix = 0;
            while ix < items.len() {
                let path = items[ix].path.clone();
                let start = ix;
                while ix < items.len() && items[ix].path == path {
                    ix += 1;
                }
                rows.push(SearchRow::FileHeader { path, count: ix - start });
                rows.extend((start..ix).map(SearchRow::Match));
            }
            rows
        }
        SearchResults::Names(items) => (0..items.len()).map(SearchRow::Name).collect(),
    }
}

fn render_body(
    _explorer: &Explorer,
    state: &SearchState,
    entity: gpui::Entity<Explorer>,
    _window: &mut Window,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let rows = build_rows(state);
    let row_count = rows.len();
    let scroll_handle = state.scroll_handle.clone();

    let empty_message = match state.status {
        SearchStatus::Idle => Some("Type to search."),
        SearchStatus::Searching => None,
        SearchStatus::Done if row_count == 0 => Some("No results."),
        SearchStatus::Done => None,
        SearchStatus::Error => None,
    };

    div()
        .relative()
        .flex()
        .flex_1()
        .flex_col()
        .min_h(px(0.0))
        .when_some(state.error.clone(), |el, error| {
            el.child(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .bg(theme::bg_error())
                    .text_color(theme::text_error())
                    .child(error),
            )
        })
        .child(
            div()
                .relative()
                .flex()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .when_some(empty_message, |el, message| {
                    el.child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme::text_muted())
                            .child(message),
                    )
                })
                .when(empty_message.is_none(), |el| {
                    el.child(
                        uniform_list(
                            "search-results-list",
                            row_count,
                            cx.processor({
                                let entity = entity.clone();
                                move |explorer: &mut Explorer, range: Range<usize>, _window, cx| {
                                    let Some(state) = &explorer.search else { return Vec::new() };
                                    range
                                        .map(|ix| {
                                            render_row(explorer, state, &rows[ix], ix, entity.clone(), cx)
                                                .into_any_element()
                                        })
                                        .collect::<Vec<_>>()
                                }
                            }),
                        )
                        .flex_1()
                        .track_scroll(scroll_handle.clone()),
                    )
                    .child(Scrollbar::vertical_for_uniform_list(
                        &scroll_handle,
                        entity.clone(),
                        ScrollbarId::SearchResults,
                    ))
                }),
        )
}

fn render_row(
    _explorer: &Explorer,
    state: &SearchState,
    row: &SearchRow,
    ix: usize,
    entity: gpui::Entity<Explorer>,
    cx: &Context<Explorer>,
) -> gpui::AnyElement {
    match row {
        SearchRow::FileHeader { path, count } => {
            let relative = path.strip_prefix(&state.root).unwrap_or(path);
            file_header_row(relative, *count, ix, path.clone(), entity, cx).into_any_element()
        }
        SearchRow::Match(match_ix) => {
            let SearchResults::Contents(items) = &state.results else {
                return div().into_any_element();
            };
            let Some(item) = items.get(*match_ix) else {
                return div().into_any_element();
            };
            let is_selected = state.selected_index == Some(*match_ix);
            let match_ix = *match_ix;
            let menu_path = item.path.clone();
            div()
                .id(("search-match-row", ix))
                .flex()
                .flex_row()
                .items_baseline()
                .gap_2()
                .w_full()
                .pl_6()
                .pr_3()
                .py_1()
                .cursor_pointer()
                .when(is_selected, |el| el.bg(theme::bg_selected()))
                .hover(|style| style.bg(theme::bg_hover()))
                .on_click(cx.listener(move |explorer, _, window, cx| {
                    explorer.reveal_result(match_ix, window, cx);
                }))
                .context_menu(move |menu, window, cx| {
                    context_menu::search_result_menu(entity.clone(), menu_path.clone(), Some(match_ix), menu, window, cx)
                })
                .child(
                    div()
                        .flex_shrink_0()
                        .w(px(36.0))
                        .text_color(theme::text_faint())
                        .child(item.line_number.to_string()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .child(highlighted_line(&item.line_text, &item.match_ranges)),
                )
                .into_any_element()
        }
        SearchRow::Name(name_ix) => {
            let SearchResults::Names(items) = &state.results else {
                return div().into_any_element();
            };
            let Some(item) = items.get(*name_ix) else {
                return div().into_any_element();
            };
            let is_selected = state.selected_index == Some(*name_ix);
            let name_ix = *name_ix;
            let path = item.path.clone();
            let relative = item
                .path
                .strip_prefix(&state.root)
                .unwrap_or(&item.path)
                .display()
                .to_string();
            let match_ranges = item.match_ranges.clone();
            let fake = FsEntry {
                path: item.path.clone(),
                is_dir: false,
                ..Default::default()
            };
            let icon = icon_theme::svg_icon_for_size(&fake, px(14.0), cx);
            let menu_path = path.clone();
            div()
                .id(("search-name-row", ix))
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .w_full()
                .px_3()
                .py_1()
                .cursor_pointer()
                .when(is_selected, |el| el.bg(theme::bg_selected()))
                .hover(|style| style.bg(theme::bg_hover()))
                .on_click(cx.listener(move |explorer, _, window, cx| {
                    explorer.reveal_result(name_ix, window, cx);
                }))
                .context_menu(move |menu, window, cx| {
                    context_menu::search_result_menu(entity.clone(), menu_path.clone(), Some(name_ix), menu, window, cx)
                })
                .child(icon)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .child(highlighted_line(&relative, &match_ranges)),
                )
                .into_any_element()
        }
    }
}

fn highlighted_line(text: &str, ranges: &[std::ops::Range<usize>]) -> gpui::AnyElement {
    let mut children: Vec<gpui::AnyElement> = Vec::new();
    let mut cursor = 0usize;

    for range in ranges {
        let (Some(plain), Some(matched)) = (text.get(cursor..range.start), text.get(range.start..range.end)) else {
            return div().child(text.to_string()).into_any_element();
        };
        if !plain.is_empty() {
            children.push(div().child(plain.to_string()).into_any_element());
        }
        children.push(
            div()
                .text_color(theme::text_primary())
                .bg(theme::text_selection_fill())
                .child(matched.to_string())
                .into_any_element(),
        );
        cursor = range.end;
    }

    if let Some(rest) = text.get(cursor..)
        && !rest.is_empty()
    {
        children.push(div().child(rest.to_string()).into_any_element());
    }

    div().flex().flex_row().children(children).into_any_element()
}

fn file_header_row(
    path: &std::path::Path,
    count: usize,
    ix: usize,
    full_path: PathBuf,
    entity: gpui::Entity<Explorer>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let click_path = full_path.clone();
    div()
        .id(("search-file-header", ix))
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .w_full()
        .px_3()
        .py_1()
        .cursor_pointer()
        .bg(theme::bg_header())
        .hover(|style| style.bg(theme::bg_hover()))
        .on_click(cx.listener(move |explorer, _, window, cx| {
            explorer.reveal_path(click_path.clone(), window, cx);
        }))
        .context_menu(move |menu, window, cx| {
            context_menu::search_result_menu(entity.clone(), full_path.clone(), None, menu, window, cx)
        })
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .child(path.display().to_string()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_color(theme::text_muted())
                .child(count.to_string()),
        )
}

