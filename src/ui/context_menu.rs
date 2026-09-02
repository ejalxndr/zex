use std::path::PathBuf;

use gpui::{Context, Entity, Window};

use crate::ui::popup_menu::{PopupMenu, PopupMenuItem};

use crate::explorer::Explorer;
use crate::explorer::columns::Column;
use crate::keys;
use crate::theme::icon_theme;

const EXPLORER_CONTEXT: &str = "Explorer";

pub fn build(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let target = explorer.update(cx, |explorer, _cx| explorer.context_menu_target.take());

    match target {
        Some(path) => file_row_menu(explorer, path, menu, window, cx),
        None => empty_space_menu(explorer, menu, window, cx),
    }
}

pub fn build_trash(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let target = explorer.update(cx, |explorer, _cx| explorer.context_menu_target.take());

    match target {
        Some(id_path) => trash_row_menu(explorer, id_path, menu, window, cx),
        None => trash_empty_space_menu(explorer, menu, window, cx),
    }
}

fn trash_row_menu(
    explorer: Entity<Explorer>,
    _id_path: PathBuf,
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let restore_explorer = explorer.clone();
    let purge_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new("Restore").on_click(move |_, _window, cx| {
            restore_explorer.update(cx, |explorer, cx| explorer.restore_selection(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Delete Permanently").on_click(move |_, _window, cx| {
            purge_explorer.update(cx, |explorer, cx| explorer.purge_selection(cx));
        }),
    )
}

fn trash_empty_space_menu(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let show_hidden = explorer.read(cx).show_hidden;
    let undo_label = explorer.read(cx).undo_label(cx);
    let redo_label = explorer.read(cx).redo_label(cx);

    let undo_shortcut = keys::shortcut_label(window, &keys::Undo, EXPLORER_CONTEXT);
    let redo_shortcut = keys::shortcut_label(window, &keys::Redo, EXPLORER_CONTEXT);
    let toggle_hidden_shortcut = keys::shortcut_label(window, &keys::ToggleHidden, EXPLORER_CONTEXT);

    let toggle_explorer = explorer.clone();
    let undo_explorer = explorer.clone();
    let redo_explorer = explorer.clone();
    let empty_trash_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new(match undo_label {
            Some(label) => format!("Undo {label}"),
            None => "Undo".to_string(),
        })
        .disabled(undo_label.is_none())
        .shortcut_opt(undo_shortcut)
        .on_click(move |_, _window, cx| {
            undo_explorer.update(cx, |explorer, cx| explorer.undo(cx));
        }),
    )
    .item(
        PopupMenuItem::new(match redo_label {
            Some(label) => format!("Redo {label}"),
            None => "Redo".to_string(),
        })
        .disabled(redo_label.is_none())
        .shortcut_opt(redo_shortcut)
        .on_click(move |_, _window, cx| {
            redo_explorer.update(cx, |explorer, cx| explorer.redo(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Empty Trash").on_click(move |_, _window, cx| {
            empty_trash_explorer.update(cx, |explorer, cx| explorer.empty_trash(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Show Hidden Files")
            .checked(show_hidden)
            .shortcut_opt(toggle_hidden_shortcut)
            .on_click(move |_, _window, cx| {
                toggle_explorer.update(cx, |explorer, cx| explorer.toggle_hidden(cx));
            }),
    )
}

pub(crate) fn file_row_menu(
    explorer: Entity<Explorer>,
    path: PathBuf,
    mut menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let is_multi = explorer.read(cx).selected.len() > 1;
    let is_dir = path.is_dir();

    if !is_multi {
        let open_explorer = explorer.clone();
        let open_path = path.clone();
        menu = menu.item(
            PopupMenuItem::new("Open").on_click(move |_, _window, cx| {
                open_explorer.update(cx, |explorer, cx| {
                    explorer.open_entry(&open_path, cx);
                });
            }),
        );

        if !is_dir {
            let open_with_explorer = explorer.clone();
            let open_with_path = path.clone();
            menu = menu.item(
                PopupMenuItem::new("Open With\u{2026}").on_click(move |_, window, cx| {
                    open_with_explorer.update(cx, |explorer, cx| {
                        explorer.begin_open_with(open_with_path.clone(), window, cx);
                    });
                }),
            );
        }
        menu = menu.separator();
    }

    if !is_multi && is_dir {
        let open_tab_explorer = explorer.clone();
        let open_tab_path = path.clone();
        menu = menu.item(
            PopupMenuItem::new("Open in New Tab").on_click(move |_, window, cx| {
                let explorer = open_tab_explorer.read(cx);
                let pane = explorer.pane.clone().expect("a rendered tab always has an owning pane");
                let show_hidden = explorer.show_hidden;
                pane.update(cx, |pane, cx| {
                    pane.spawn_new_tab_at(open_tab_path.clone(), show_hidden, window, cx);
                });
            }),
        );
    }

    if !is_multi {
        let rename_shortcut = keys::shortcut_label(window, &keys::Rename, EXPLORER_CONTEXT);
        let rename_explorer = explorer.clone();
        let rename_path = path.clone();
        menu = menu.item(
            PopupMenuItem::new("Rename")
                .shortcut_opt(rename_shortcut)
                .on_click(move |_, window, cx| {
                    rename_explorer.update(cx, |explorer, cx| {
                        explorer.begin_rename(rename_path.clone(), window, cx);
                    });
                }),
        );
    }

    let cut_shortcut = keys::shortcut_label(window, &keys::Cut, EXPLORER_CONTEXT);
    let copy_shortcut = keys::shortcut_label(window, &keys::Copy, EXPLORER_CONTEXT);
    let delete_shortcut = keys::shortcut_label(window, &keys::Delete, EXPLORER_CONTEXT);

    let cut_explorer = explorer.clone();
    let copy_explorer = explorer.clone();
    let copy_path_explorer = explorer.clone();
    let copy_path_target = path.clone();
    let delete_explorer = explorer.clone();
    let properties_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new("Cut")
            .shortcut_opt(cut_shortcut)
            .on_click(move |_, _window, cx| {
                cut_explorer.update(cx, |explorer, cx| explorer.cut_selection(cx));
            }),
    )
    .item(
        PopupMenuItem::new("Copy")
            .shortcut_opt(copy_shortcut)
            .on_click(move |_, _window, cx| {
                copy_explorer.update(cx, |explorer, cx| explorer.copy_selection(cx));
            }),
    )
    .item(
        PopupMenuItem::new("Copy Path(s)").on_click(move |_, _window, cx| {
            copy_path_explorer.update(cx, |explorer, cx| {
                let paths: Vec<PathBuf> = if explorer.selected.len() > 1 {
                    explorer.selected.iter().cloned().collect()
                } else {
                    vec![copy_path_target.clone()]
                };
                explorer.copy_paths_to_clipboard(&paths, cx);
            });
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Delete")
            .shortcut_opt(delete_shortcut)
            .on_click(move |_, _window, cx| {
                delete_explorer.update(cx, |explorer, cx| explorer.delete_selection(cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Properties").on_click(move |_, window, cx| {
            properties_explorer.update(cx, |explorer, cx| {
                let paths: Vec<PathBuf> = explorer.selected.iter().cloned().collect();
                explorer.open_properties(paths, window, cx);
            });
        }),
    )
}

pub fn search_result_menu(
    explorer: Entity<Explorer>,
    path: PathBuf,
    reveal_index: Option<usize>,
    menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    explorer.update(cx, |explorer, cx| {
        explorer.selected = std::iter::once(path.clone()).collect();
        cx.notify();
    });

    let open_explorer = explorer.clone();
    let open_path = path.clone();
    let open_with_explorer = explorer.clone();
    let open_with_path = path.clone();
    let reveal_explorer = explorer.clone();
    let reveal_path_target = path.clone();
    let copy_explorer = explorer.clone();
    let cut_explorer = explorer.clone();
    let copy_path_explorer = explorer.clone();
    let copy_path_target = path.clone();
    let delete_explorer = explorer.clone();
    let properties_explorer = explorer.clone();
    let properties_path = path;

    menu.item(
        PopupMenuItem::new("Open").on_click(move |_, _window, cx| {
            open_explorer.update(cx, |explorer, cx| {
                if let Err(err) = open::that_detached(&open_path) {
                    explorer.op_error = Some(format!("Couldn't open {}: {err}", open_path.display()));
                    cx.notify();
                }
            });
        }),
    )
    .item(
        PopupMenuItem::new("Open With\u{2026}").on_click(move |_, window, cx| {
            open_with_explorer.update(cx, |explorer, cx| {
                explorer.begin_open_with(open_with_path.clone(), window, cx);
            });
        }),
    )
    .item(
        PopupMenuItem::new("Reveal in Explorer").on_click(move |_, window, cx| {
            reveal_explorer.update(cx, |explorer, cx| match reveal_index {
                Some(ix) => explorer.reveal_result(ix, window, cx),
                None => explorer.reveal_path(reveal_path_target.clone(), window, cx),
            });
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Copy").on_click(move |_, _window, cx| {
            copy_explorer.update(cx, |explorer, cx| explorer.copy_selection(cx));
        }),
    )
    .item(
        PopupMenuItem::new("Cut").on_click(move |_, _window, cx| {
            cut_explorer.update(cx, |explorer, cx| explorer.cut_selection(cx));
        }),
    )
    .item(
        PopupMenuItem::new("Copy Path").on_click(move |_, _window, cx| {
            copy_path_explorer.update(cx, |explorer, cx| {
                explorer.copy_paths_to_clipboard(std::slice::from_ref(&copy_path_target), cx);
            });
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Delete").on_click(move |_, _window, cx| {
            delete_explorer.update(cx, |explorer, cx| explorer.delete_selection(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Properties").on_click(move |_, window, cx| {
            properties_explorer.update(cx, |explorer, cx| {
                explorer.open_properties(vec![properties_path.clone()], window, cx);
            });
        }),
    )
}

pub fn disk_usage_row_menu(
    explorer: Entity<Explorer>,
    path: PathBuf,
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let open_explorer = explorer.clone();
    let open_path = path.clone();
    let copy_explorer = explorer.clone();
    let copy_path = path.clone();
    let trash_explorer = explorer;

    menu.item(PopupMenuItem::new("Open").on_click(move |_, _window, cx| {
        open_explorer.update(cx, |explorer, cx| {
            explorer.open_disk_usage_entry(open_path.clone(), cx);
        });
    }))
    .item(
        PopupMenuItem::new("Copy Path").on_click(move |_, _window, cx| {
            copy_explorer.update(cx, |explorer, cx| {
                explorer.copy_paths_to_clipboard(std::slice::from_ref(&copy_path), cx);
            });
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Move to Trash").on_click(move |_, _window, cx| {
            trash_explorer.update(cx, |explorer, cx| {
                explorer.trash_disk_usage_entry(path.clone(), cx);
            });
        }),
    )
}

pub fn history_menu(
    explorer: Entity<Explorer>,
    entries: Vec<(usize, PathBuf)>,
    mut menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    if entries.is_empty() {
        return menu.item(PopupMenuItem::new("No History").disabled(true));
    }

    for (index, path) in entries {
        let label = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let jump_explorer = explorer.clone();

        menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _window, cx| {
            jump_explorer.update(cx, |explorer, cx| {
                explorer.go_to_history_entry(index, cx);
            });
        }));
    }

    menu
}

pub fn column_menu(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let visibility = explorer.read(cx).column_visibility;

    let type_explorer = explorer.clone();
    let size_explorer = explorer.clone();
    let modified_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new("Type")
            .checked(visibility.get(Column::Type))
            .on_click(move |_, _window, cx| {
                type_explorer.update(cx, |explorer, cx| {
                    explorer.toggle_column(Column::Type, cx);
                });
            }),
    )
    .item(
        PopupMenuItem::new("Size")
            .checked(visibility.get(Column::Size))
            .on_click(move |_, _window, cx| {
                size_explorer.update(cx, |explorer, cx| {
                    explorer.toggle_column(Column::Size, cx);
                });
            }),
    )
    .item(
        PopupMenuItem::new("Date Modified")
            .checked(visibility.get(Column::Modified))
            .on_click(move |_, _window, cx| {
                modified_explorer.update(cx, |explorer, cx| {
                    explorer.toggle_column(Column::Modified, cx);
                });
            }),
    )
}

fn empty_space_menu(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let show_hidden = explorer.read(cx).show_hidden;
    let has_clipboard = explorer.read(cx).shared.read(cx).clipboard.is_some()
        || cx
            .read_from_clipboard()
            .is_some_and(|item| item.paths().is_some());
    let undo_label = explorer.read(cx).undo_label(cx);
    let redo_label = explorer.read(cx).redo_label(cx);

    let undo_shortcut = keys::shortcut_label(window, &keys::Undo, EXPLORER_CONTEXT);
    let redo_shortcut = keys::shortcut_label(window, &keys::Redo, EXPLORER_CONTEXT);
    let paste_shortcut = keys::shortcut_label(window, &keys::Paste, EXPLORER_CONTEXT);
    let toggle_hidden_shortcut = keys::shortcut_label(window, &keys::ToggleHidden, EXPLORER_CONTEXT);

    let new_folder_explorer = explorer.clone();
    let new_file_explorer = explorer.clone();
    let paste_explorer = explorer.clone();
    let toggle_explorer = explorer.clone();
    let properties_explorer = explorer.clone();
    let undo_explorer = explorer.clone();
    let redo_explorer = explorer.clone();

    let menu = menu
        .item(
            PopupMenuItem::new(match undo_label {
                Some(label) => format!("Undo {label}"),
                None => "Undo".to_string(),
            })
            .disabled(undo_label.is_none())
            .shortcut_opt(undo_shortcut)
            .on_click(move |_, _window, cx| {
                undo_explorer.update(cx, |explorer, cx| explorer.undo(cx));
            }),
        )
        .item(
            PopupMenuItem::new(match redo_label {
                Some(label) => format!("Redo {label}"),
                None => "Redo".to_string(),
            })
            .disabled(redo_label.is_none())
            .shortcut_opt(redo_shortcut)
            .on_click(move |_, _window, cx| {
                redo_explorer.update(cx, |explorer, cx| explorer.redo(cx));
            }),
        )
        .separator();

    menu.item(
        PopupMenuItem::new("New Folder")
            .icon(icon_theme::directory_menu_icon(cx))
            .on_click(move |_, window, cx| {
                new_folder_explorer
                    .update(cx, |explorer, cx| explorer.begin_new_folder(window, cx));
            }),
    )
    .item(
        PopupMenuItem::new("New File")
            .icon(icon_theme::generic_file_menu_icon(cx))
            .on_click(move |_, window, cx| {
                new_file_explorer.update(cx, |explorer, cx| explorer.begin_new_file(window, cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Paste")
            .disabled(!has_clipboard)
            .shortcut_opt(paste_shortcut)
            .on_click(move |_, _window, cx| {
                paste_explorer.update(cx, |explorer, cx| explorer.paste(cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Show Hidden Files")
            .checked(show_hidden)
            .shortcut_opt(toggle_hidden_shortcut)
            .on_click(move |_, _window, cx| {
                toggle_explorer.update(cx, |explorer, cx| explorer.toggle_hidden(cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Properties").on_click(move |_, window, cx| {
            properties_explorer.update(cx, |explorer, cx| {
                let dir = explorer.current_dir().to_path_buf();
                explorer.open_properties(vec![dir], window, cx);
            });
        }),
    )
}
