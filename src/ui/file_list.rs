use std::ops::Range;
use std::time::{Duration, SystemTime};

use crate::explorer::Explorer;
use crate::explorer::clipboard_ops::ClipboardOp;
use crate::explorer::columns::{Column, ColumnVisibility, ColumnWidths, SortColumn, SortDirection};
use crate::explorer::drag::ScrollbarId;
use crate::explorer::git_ops::git_letter_for;
use crate::filesystem::entry::{self, format_modified, format_size, type_label};
use crate::filesystem::operations::new_entry::NewEntryKind;
use crate::git::GitFileStatus;
use crate::settings::GitBadgeStyle;
use crate::theme;
use crate::theme::icon_theme;
use crate::ui::column_header;
use crate::ui::context_menu;
use crate::ui::popup_menu::ContextMenuExt;
use crate::ui::scrollbar::Scrollbar;
use crate::ui::text_input::{Escape, TextInput as Input};
use crate::ui::thumbnail;
use gpui::{
    AnyElement, Context, DevicePixels, Div, DragMoveEvent, Entity, IntoElement, MouseButton,
    MouseDownEvent, MouseUpEvent, Render, Stateful, Window, div, prelude::*, px, uniform_list,
};

const NAME_MIN_WIDTH: f32 = 80.0;
const HEADER_HEIGHT: f32 = 28.0;
const DRAG_ICON_LOGICAL_SIZE: f32 = 24.0;
const TRASH_LOCATION_WIDTH: f32 = 260.0;
const TRASH_DELETED_WIDTH: f32 = 170.0;

struct BoxSelectGhost;

impl Render for BoxSelectGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn resize_handle(
    column: Column,
    explorer: Entity<Explorer>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let drag_explorer = explorer.clone();

    column_header::resize_handle(
        ("column-resize-handle", column as u8 as u64),
        column,
        move |_column, _point, window, cx| {
            let anchor_x = f32::from(window.mouse_position().x);
            drag_explorer.update(cx, |explorer, cx| {
                explorer.begin_column_resize(column, anchor_x, cx);
            });
            cx.new(|_| column_header::ResizeGhost)
        },
        cx.listener(
            move |explorer, event: &DragMoveEvent<Column>, _window, cx| {
                explorer.update_column_resize(f32::from(event.event.position.x), cx);
            },
        ),
        cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
            explorer.end_column_resize(cx);
        }),
    )
}

fn sort_header_cell(
    width: Option<f32>,
    label: &'static str,
    sort_column: SortColumn,
    explorer: &Explorer,
    cx: &Context<Explorer>,
) -> Stateful<Div> {
    let active = explorer.sort_column == sort_column;
    let direction = if active {
        explorer.sort_direction
    } else {
        SortDirection::Ascending
    };

    column_header::sort_cell(
        ("sort-header", sort_column as u8 as u64),
        format!("sort-header-{sort_column:?}"),
        label,
        width,
        NAME_MIN_WIDTH,
        active,
        direction,
        cx.listener(move |explorer, _event: &gpui::ClickEvent, _window, cx| {
            explorer.set_sort(sort_column, cx);
        }),
    )
}

fn column_cell(width: f32, text: impl IntoElement) -> Div {
    div()
        .w(px(width))
        .h_full()
        .flex_shrink_0()
        .flex()
        .items_center()
        .pl_2()
        .child(text)
}

fn column_divider_overlay(widths: ColumnWidths, visibility: ColumnVisibility) -> impl IntoElement {
    let mut divider_widths = Vec::new();
    if visibility.get(Column::Type) {
        divider_widths.push(widths.get(Column::Type));
    }
    if visibility.get(Column::Size) {
        divider_widths.push(widths.get(Column::Size));
    }
    if visibility.get(Column::Modified) {
        divider_widths.push(widths.get(Column::Modified));
    }

    column_header::divider_overlay(px(HEADER_HEIGHT), NAME_MIN_WIDTH, divider_widths)
}

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> impl IntoElement {
    if explorer.is_trash() {
        render_trash(explorer, cx)
    } else {
        render_dir(explorer, cx)
    }
}

fn render_dir(explorer: &Explorer, cx: &Context<Explorer>) -> AnyElement {
    if let Some(error) = &explorer.error {
        return div()
            .flex_1()
            .w_full()
            .p_3()
            .text_color(theme::text_error())
            .child(error.clone())
            .into_any_element();
    }

    let row_count = explorer.entries.len();
    let list_entity = cx.entity();
    let header_entity = cx.entity();
    let entity = cx.entity();
    let scroll_handle = explorer.scroll_handle.clone();
    let visibility = explorer.column_visibility;
    let widths = explorer.column_widths;
    let current_dir = explorer.current_dir().to_path_buf();

    let box_select_overlay = explorer.box_select.as_ref().map(|drag| {
        let viewport = scroll_handle.0.borrow().base_handle.bounds();
        let top = (drag.anchor.y.min(drag.current.y) - viewport.origin.y).max(px(0.0));
        let bottom =
            (drag.anchor.y.max(drag.current.y) - viewport.origin.y).min(viewport.size.height);
        let height = (bottom - top).max(px(0.0));

        let left = (drag.anchor.x.min(drag.current.x) - viewport.origin.x).max(px(0.0));
        let right =
            (drag.anchor.x.max(drag.current.x) - viewport.origin.x).min(viewport.size.width);
        let width = (right - left).max(px(0.0));

        div()
            .absolute()
            .top(top)
            .left(left)
            .w(width)
            .h(height)
            .bg(theme::box_select_fill())
            .border_1()
            .border_color(theme::box_select_border())
    });

    let new_entry_row = explorer.new_entry.as_ref().map(|new_entry| {
        let icon = match new_entry.kind {
            NewEntryKind::Folder => icon_theme::directory_svg_icon(cx).into_any_element(),
            NewEntryKind::File => icon_theme::generic_file_svg_icon(cx).into_any_element(),
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_0p5()
            .child(icon)
            .child(
                div()
                    .flex_1()
                    .on_action(cx.listener(|explorer, _: &Escape, _window, cx| {
                        explorer.cancel_new_entry(cx)
                    }))
                    .child(Input::new(&new_entry.input)),
            )
    });

    div()
        .relative()
        .flex_1()
        .flex()
        .flex_col()
        .w_full()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(HEADER_HEIGHT))
                .px_3()
                .bg(theme::bg_header())
                .text_color(theme::text_muted())
                .border_b_1()
                .border_color(theme::border())
                .context_menu(move |menu, window, cx| {
                    context_menu::column_menu(header_entity.clone(), menu, window, cx)
                })
                .child(sort_header_cell(None, "Name", SortColumn::Name, explorer, cx))
                .when(visibility.get(Column::Type), |row| {
                    row.child(
                        sort_header_cell(
                            Some(widths.get(Column::Type)),
                            "Type",
                            SortColumn::Type,
                            explorer,
                            cx,
                        )
                        .relative()
                        .child(resize_handle(Column::Type, entity.clone(), cx)),
                    )
                })
                .when(visibility.get(Column::Size), |row| {
                    row.child(
                        sort_header_cell(
                            Some(widths.get(Column::Size)),
                            "Size",
                            SortColumn::Size,
                            explorer,
                            cx,
                        )
                        .relative()
                        .child(resize_handle(Column::Size, entity.clone(), cx)),
                    )
                })
                .when(visibility.get(Column::Modified), |row| {
                    row.child(
                        sort_header_cell(
                            Some(widths.get(Column::Modified)),
                            "Date Modified",
                            SortColumn::Modified,
                            explorer,
                            cx,
                        )
                        .relative()
                        .child(resize_handle(Column::Modified, entity.clone(), cx)),
                    )
                }),
        )
        .child(
            div()
                .id("file-list-body")
                .flex_1()
                .flex()
                .flex_col()
                .w_full()
                .context_menu(move |menu, window, cx| {
                    context_menu::build(list_entity.clone(), menu, window, cx)
                })
                .can_drop({
                    let current_dir = current_dir.clone();
                    move |paths, _window, _cx| {
                        let Some(paths) = paths.downcast_ref::<gpui::ExternalPaths>() else {
                            return false;
                        };
                        paths
                            .paths()
                            .iter()
                            .any(|path| path.parent() != Some(current_dir.as_path()))
                    }
                })
                .drag_over::<gpui::ExternalPaths>(|style, _paths, _window, _cx| {
                    style
                        .bg(theme::drop_target_fill())
                        .border_1()
                        .border_color(theme::drop_target_border())
                })
                .on_drop::<gpui::ExternalPaths>(cx.listener({
                    let current_dir = current_dir.clone();
                    move |explorer, paths: &gpui::ExternalPaths, _window, cx| {
                        explorer.move_paths_into(paths.paths().to_vec(), current_dir.clone(), cx);
                    }
                }))
                .on_drag((), {
                    let box_select_entity = entity.clone();
                    move |_, _point, window, cx| {
                        let origin = window.mouse_position();
                        let modifiers = window.modifiers();
                        box_select_entity.update(cx, |explorer, cx| {
                            explorer.begin_box_select(origin, modifiers, cx);
                        });
                        cx.new(|_| BoxSelectGhost)
                    }
                })
                .on_drag_move::<()>(cx.listener(
                    move |explorer, event: &DragMoveEvent<()>, _window, cx| {
                        explorer.update_box_select(event.event.position, cx);
                    },
                ))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                        explorer.end_box_select(cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                        explorer.end_box_select(cx);
                    }),
                )
                .on_click(cx.listener(|explorer, event: &gpui::ClickEvent, _window, cx| {
                    explorer.click_empty_space(event.position(), cx);
                }))
                .children(new_entry_row)
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(
                            uniform_list(
                                "file-list",
                                row_count,
                                cx.processor(move |explorer: &mut Explorer, range: Range<usize>, _window: &mut Window, cx| {
                                    let mut rows = Vec::new();

                                    for ix in range {
                                        let entry = explorer.entries[ix].clone();
                                        let is_selected = explorer.selected.contains(&entry.path);
                                        let is_cut = explorer
                                            .shared
                                            .read(cx)
                                            .clipboard
                                            .as_ref()
                                            .map(|clip| clip.op == ClipboardOp::Cut && clip.paths.contains(&entry.path))
                                            .unwrap_or(false);
                                        let type_text = type_label(&entry);
                                        let size_text = if entry.is_dir {
                                            String::new()
                                        } else {
                                            format_size(entry.size)
                                        };
                                        let modified_text = format_modified(entry.modified);
                                        let icon = thumbnail::icon_element(&entry, px(16.0), cx)
                                            .unwrap_or_else(|| icon_theme::svg_icon_for(&entry, cx));
                                        let path = entry.path.clone();
                                        let right_click_path = path.clone();
                                        let widths = explorer.column_widths;

                                        let drag_entry = entry.clone();
                                        let drag_paths = if is_selected && explorer.selected.len() > 1 {
                                            explorer.selected.iter().cloned().collect()
                                        } else {
                                            vec![entry.path.clone()]
                                        };

                                        let git_status = explorer.git_status_for(&entry);
                                        let badge_style = explorer.git_settings.status.badge_style;
                                        let git_color = git_status.map(|status| explorer.git_color_for(status));
                                        let is_dimmed_ignored = git_status == Some(GitFileStatus::Ignored)
                                            && explorer.git_settings.status.dim_ignored;
                                        let show_text_color = matches!(
                                            badge_style,
                                            GitBadgeStyle::TextColor | GitBadgeStyle::Both
                                        );
                                        let git_badge = git_status
                                            .filter(|_| matches!(badge_style, GitBadgeStyle::Icon | GitBadgeStyle::Both))
                                            .map(|status| {
                                                div()
                                                    .flex_shrink_0()
                                                    .w(px(14.0))
                                                    .h(px(14.0))
                                                    .rounded_full()
                                                    .bg(explorer.git_color_for(status))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_size(px(9.0))
                                                    .text_color(theme::bg_root())
                                                    .child(git_letter_for(status))
                                            });

                                        let name_child: AnyElement = match &explorer.renaming {
                                            Some(renaming) if renaming.path == entry.path => div()
                                                .flex_1()
                                                .min_w(px(0.0))
                                                .on_action(cx.listener(|explorer, _: &Escape, _window, cx| {
                                                    explorer.cancel_rename(cx)
                                                }))
                                                .child(Input::new(&renaming.input))
                                                .into_any_element(),
                                            _ => {
                                                let mut name_div = div()
                                                    .flex_1()
                                                    .min_w(px(0.0))
                                                    .truncate()
                                                    .child(entry.name.clone());
                                                if entry.is_broken_symlink {
                                                    name_div = name_div.text_color(theme::text_error());
                                                } else if show_text_color
                                                    && let Some(color) = git_color
                                                {
                                                    name_div = name_div.text_color(color);
                                                }
                                                name_div.into_any_element()
                                            }
                                        };

                                        rows.push(
                                            div()
                                                .id(ix)
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .w_full()
                                                .px_3()
                                                .py_0p5()
                                                .when(is_cut, |this| this.opacity(0.5))
                                                .when(!is_cut && is_dimmed_ignored, |this| this.opacity(0.55))
                                                .when(is_selected, |this| this.bg(theme::bg_selected()))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener({
                                                        let path = path.clone();
                                                        move |explorer, event: &MouseDownEvent, _window, cx| {
                                                            explorer.mouse_down_select(
                                                                path.clone(),
                                                                event.modifiers,
                                                                event.click_count,
                                                                cx,
                                                            );
                                                        }
                                                    }),
                                                )
                                                .on_mouse_up(
                                                    MouseButton::Left,
                                                    cx.listener(move |explorer, event: &MouseUpEvent, _window, cx| {
                                                        explorer.mouse_up_select(
                                                            path.clone(),
                                                            event.modifiers,
                                                            event.click_count,
                                                            cx,
                                                        );
                                                    }),
                                                )
                                                .on_mouse_down(
                                                    MouseButton::Right,
                                                    cx.listener(move |explorer, _event, _window, _cx| {
                                                        explorer.context_menu_target = if is_selected {
                                                            Some(right_click_path.clone())
                                                        } else {
                                                            None
                                                        };
                                                    }),
                                                )
                                                .on_drag_out(move |window, cx| {
                                                    eprintln!("[drag-out] starting drag for {:?}", drag_paths);
                                                    let resolved_icon = cx
                                                        .global::<icon_theme::IconThemeState>()
                                                        .icon_for(&drag_entry);
                                                    let physical_size = DevicePixels(
                                                        (DRAG_ICON_LOGICAL_SIZE * window.scale_factor())
                                                            .round() as i32,
                                                    );
                                                    let icon = icon_theme::rasterize_drag_icon(
                                                        &resolved_icon,
                                                        physical_size,
                                                        cx,
                                                    );
                                                    eprintln!("[drag-out] icon computed: {}", icon.is_some());
                                                    Some((drag_paths.clone(), icon))
                                                })
                                                .when(entry.is_dir, |row| {
                                                    let drop_path = entry.path.clone();
                                                    row.drag_over::<gpui::ExternalPaths>(|style, _paths, _window, _cx| {
                                                        style
                                                            .bg(theme::drop_target_fill())
                                                            .border_1()
                                                            .border_color(theme::drop_target_border())
                                                    })
                                                    .on_drop::<gpui::ExternalPaths>(cx.listener(
                                                        move |explorer, paths: &gpui::ExternalPaths, _window, cx| {
                                                            explorer.move_paths_into(
                                                                paths.paths().to_vec(),
                                                                drop_path.clone(),
                                                                cx,
                                                            );
                                                        },
                                                    ))
                                                })
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_2()
                                                        .flex_1()
                                                        .min_w(px(0.0))
                                                        .child(icon)
                                                        .child(name_child)
                                                        .children(git_badge),
                                                )
                                                .when(explorer.column_visibility.get(Column::Type), |row| {
                                                    row.child(
                                                        column_cell(
                                                            widths.get(Column::Type),
                                                            div()
                                                                .text_color(theme::text_muted())
                                                                .child(type_text.clone()),
                                                        ),
                                                    )
                                                })
                                                .when(explorer.column_visibility.get(Column::Size), |row| {
                                                    row.child(
                                                        column_cell(
                                                            widths.get(Column::Size),
                                                            div()
                                                                .text_color(theme::text_muted())
                                                                .child(size_text.clone()),
                                                        ),
                                                    )
                                                })
                                                .when(explorer.column_visibility.get(Column::Modified), |row| {
                                                    row.child(
                                                        column_cell(
                                                            widths.get(Column::Modified),
                                                            div()
                                                                .text_color(theme::text_muted())
                                                                .child(modified_text.clone()),
                                                        ),
                                                    )
                                                }),
                                        );
                                    }

                                    rows
                                }),
                            )
                            .flex_1()
                            .track_scroll(scroll_handle.clone()),
                        )
                        .child(Scrollbar::vertical_for_uniform_list(
                            &scroll_handle,
                            entity.clone(),
                            ScrollbarId::FileList,
                        ))
                        .children(box_select_overlay),
                ),
        )
        .child(column_divider_overlay(widths, visibility))
        .into_any_element()
}

fn trash_column_divider_overlay(size_width: f32) -> impl IntoElement {
    column_header::divider_overlay(
        px(HEADER_HEIGHT),
        NAME_MIN_WIDTH,
        [TRASH_LOCATION_WIDTH, size_width, TRASH_DELETED_WIDTH],
    )
}

fn render_trash(explorer: &Explorer, cx: &Context<Explorer>) -> AnyElement {
    if let Some(error) = &explorer.error {
        return div()
            .flex_1()
            .w_full()
            .p_3()
            .text_color(theme::text_error())
            .child(error.clone())
            .into_any_element();
    }

    let row_count = explorer.trash_entries.len();
    let list_entity = cx.entity();
    let entity = cx.entity();
    let scroll_handle = explorer.scroll_handle.clone();
    let size_width = explorer.column_widths.get(Column::Size);

    let box_select_overlay = explorer.box_select.as_ref().map(|drag| {
        let viewport = scroll_handle.0.borrow().base_handle.bounds();
        let top = (drag.anchor.y.min(drag.current.y) - viewport.origin.y).max(px(0.0));
        let bottom =
            (drag.anchor.y.max(drag.current.y) - viewport.origin.y).min(viewport.size.height);
        let height = (bottom - top).max(px(0.0));

        let left = (drag.anchor.x.min(drag.current.x) - viewport.origin.x).max(px(0.0));
        let right =
            (drag.anchor.x.max(drag.current.x) - viewport.origin.x).min(viewport.size.width);
        let width = (right - left).max(px(0.0));

        div()
            .absolute()
            .top(top)
            .left(left)
            .w(width)
            .h(height)
            .bg(theme::box_select_fill())
            .border_1()
            .border_color(theme::box_select_border())
    });

    div()
        .relative()
        .flex_1()
        .flex()
        .flex_col()
        .w_full()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(HEADER_HEIGHT))
                .px_3()
                .bg(theme::bg_header())
                .text_color(theme::text_muted())
                .border_b_1()
                .border_color(theme::border())
                .child(div().flex_1().min_w(px(NAME_MIN_WIDTH)).child("Name"))
                .child(column_cell(TRASH_LOCATION_WIDTH, "Original Location"))
                .child(column_cell(size_width, "Size"))
                .child(column_cell(TRASH_DELETED_WIDTH, "Trashed On")),
        )
        .child(
            div()
                .id("trash-list-body")
                .flex_1()
                .flex()
                .flex_col()
                .w_full()
                .context_menu(move |menu, window, cx| {
                    context_menu::build_trash(list_entity.clone(), menu, window, cx)
                })
                .on_drag((), {
                    let box_select_entity = entity.clone();
                    move |_, _point, window, cx| {
                        let origin = window.mouse_position();
                        let modifiers = window.modifiers();
                        box_select_entity.update(cx, |explorer, cx| {
                            explorer.begin_box_select(origin, modifiers, cx);
                        });
                        cx.new(|_| BoxSelectGhost)
                    }
                })
                .on_drag_move::<()>(cx.listener(
                    move |explorer, event: &DragMoveEvent<()>, _window, cx| {
                        explorer.update_box_select(event.event.position, cx);
                    },
                ))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                        explorer.end_box_select(cx);
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                        explorer.end_box_select(cx);
                    }),
                )
                .on_click(cx.listener(|explorer, event: &gpui::ClickEvent, _window, cx| {
                    explorer.click_empty_space(event.position(), cx);
                }))
                .child(
                    div()
                        .relative()
                        .flex()
                        .flex_1()
                        .w_full()
                        .overflow_hidden()
                        .child(
                            uniform_list(
                                "trash-list",
                                row_count,
                                cx.processor(move |explorer: &mut Explorer, range: Range<usize>, _window: &mut Window, cx| {
                                    let mut rows = Vec::new();

                                    for ix in range {
                                        let entry = explorer.trash_entries[ix].clone();
                                        let is_selected = explorer.selected.contains(&entry.id_path);
                                        let size_text = if entry.is_dir {
                                            String::new()
                                        } else {
                                            format_size(entry.size)
                                        };
                                        let deleted_text = format_modified(Some(
                                            SystemTime::UNIX_EPOCH
                                                + Duration::from_secs(entry.deleted_at.max(0) as u64),
                                        ));
                                        let original_location = entry
                                            .original_path
                                            .parent()
                                            .map(|parent| parent.to_string_lossy().into_owned())
                                            .unwrap_or_default();
                                        let fake_entry = entry::FsEntry {
                                            name: entry.name.clone(),
                                            path: entry.original_path.clone(),
                                            is_dir: entry.is_dir,
                                            size: entry.size,
                                            modified: None,
                                            ..Default::default()
                                        };
                                        let icon = icon_theme::svg_icon_for(&fake_entry, cx);
                                        let id_path = entry.id_path.clone();
                                        let right_click_path = id_path.clone();

                                        rows.push(
                                            div()
                                                .id(ix)
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .w_full()
                                                .px_3()
                                                .py_0p5()
                                                .when(is_selected, |this| this.bg(theme::bg_selected()))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener({
                                                        let path = id_path.clone();
                                                        move |explorer, event: &MouseDownEvent, _window, cx| {
                                                            explorer.mouse_down_select(
                                                                path.clone(),
                                                                event.modifiers,
                                                                event.click_count,
                                                                cx,
                                                            );
                                                        }
                                                    }),
                                                )
                                                .on_mouse_up(
                                                    MouseButton::Left,
                                                    cx.listener(move |explorer, event: &MouseUpEvent, _window, cx| {
                                                        explorer.mouse_up_select(
                                                            id_path.clone(),
                                                            event.modifiers,
                                                            event.click_count,
                                                            cx,
                                                        );
                                                    }),
                                                )
                                                .on_mouse_down(
                                                    MouseButton::Right,
                                                    cx.listener(move |explorer, _event, _window, _cx| {
                                                        explorer.context_menu_target = if is_selected {
                                                            Some(right_click_path.clone())
                                                        } else {
                                                            None
                                                        };
                                                    }),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .flex_row()
                                                        .items_center()
                                                        .gap_2()
                                                        .flex_1()
                                                        .min_w(px(0.0))
                                                        .child(icon)
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w(px(0.0))
                                                                .truncate()
                                                                .child(entry.name.clone()),
                                                        ),
                                                )
                                                .child(column_cell(
                                                    TRASH_LOCATION_WIDTH,
                                                    div()
                                                        .truncate()
                                                        .text_color(theme::text_muted())
                                                        .child(original_location),
                                                ))
                                                .child(column_cell(
                                                    size_width,
                                                    div()
                                                        .text_color(theme::text_muted())
                                                        .child(size_text),
                                                ))
                                                .child(column_cell(
                                                    TRASH_DELETED_WIDTH,
                                                    div()
                                                        .text_color(theme::text_muted())
                                                        .child(deleted_text),
                                                )),
                                        );
                                    }

                                    rows
                                }),
                            )
                            .flex_1()
                            .track_scroll(scroll_handle.clone()),
                        )
                        .child(Scrollbar::vertical_for_uniform_list(
                            &scroll_handle,
                            entity.clone(),
                            ScrollbarId::FileList,
                        ))
                        .children(box_select_overlay),
                ),
        )
        .child(trash_column_divider_overlay(size_width))
        .into_any_element()
}
