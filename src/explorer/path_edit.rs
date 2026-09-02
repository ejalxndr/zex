use std::path::{Path, PathBuf};

use gpui::{AppContext, Context, Entity, Subscription, Window};

use crate::ui::text_input::{TextInputEvent as InputEvent, TextInputState as InputState};

use super::{Explorer, TRASH_VIRTUAL_PATH, trash_virtual_path};

pub struct PathEditState {
    pub input: Entity<InputState>,
    pub suggestions: Vec<PathBuf>,
    pub selected_suggestion: Option<usize>,
    _input_subscription: Subscription,
    _focus_out_subscription: Subscription,
}

const MAX_PATH_SUGGESTIONS: usize = 8;

fn path_completions(text: &str) -> Vec<PathBuf> {
    let path = Path::new(text);
    let (dir, prefix) = if text.is_empty() || text.ends_with('/') {
        (path.to_path_buf(), String::new())
    } else {
        match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => {
                let parent = if parent.as_os_str().is_empty() {
                    PathBuf::from("/")
                } else {
                    parent.to_path_buf()
                };
                (parent, name.to_string_lossy().into_owned())
            }
            _ => (PathBuf::from("/"), String::new()),
        }
    };

    let Ok(read_dir) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let prefix_lower = prefix.to_lowercase();
    let mut matches: Vec<PathBuf> = read_dir
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|entry| {
            prefix.is_empty()
                || entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with(&prefix_lower)
        })
        .map(|entry| entry.path())
        .collect();

    matches.sort_by(|a, b| {
        let name = |p: &Path| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        };
        let (an, bn) = (name(a), name(b));
        an.eq(&prefix_lower)
            .cmp(&bn.eq(&prefix_lower))
            .reverse()
            .then(an.cmp(&bn))
    });
    matches.truncate(MAX_PATH_SUGGESTIONS);
    matches
}

impl Explorer {
    pub fn begin_edit_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let default_value = match &self.disk_usage {
            Some(state) => state.current_root.to_string_lossy().into_owned(),
            None => self.current_dir().to_string_lossy().into_owned(),
        };

        let input = cx.new(|cx| InputState::new(window, cx).default_value(default_value));
        input.update(cx, |input, cx| input.focus(window, cx));
        let focus_handle = input.read(cx).focus_handle();

        let input_subscription =
            cx.subscribe(
                &input,
                |explorer: &mut Self, _input, event, cx| match event {
                    InputEvent::PressEnter { .. } => explorer.commit_edit_path(cx),
                    InputEvent::Changed { .. } => explorer.update_path_suggestions(cx),
                },
            );

        let this = cx.entity();
        let focus_out_subscription =
            window.on_focus_out(&focus_handle, cx, move |_event, _window, cx| {
                this.update(cx, |explorer, cx| explorer.cancel_edit_path(cx));
            });

        self.editing_path = Some(PathEditState {
            input,
            suggestions: Vec::new(),
            selected_suggestion: None,
            _input_subscription: input_subscription,
            _focus_out_subscription: focus_out_subscription,
        });
        cx.notify();
    }

    pub fn commit_edit_path(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = self.editing_path.take() else {
            return;
        };

        let text = editing.input.read(cx).value().to_string();
        let trimmed = text.trim();

        if trimmed == TRASH_VIRTUAL_PATH && self.disk_usage.is_none() {
            self.op_error = None;
            self.navigate_to(trash_virtual_path(), cx);
            cx.notify();
            return;
        }

        let path = PathBuf::from(trimmed);

        if path.is_dir() {
            self.op_error = None;
            self.navigate_or_drill(path, cx);
        } else {
            self.op_error = Some(format!("Not a directory: {}", path.display()));
        }
        cx.notify();
    }

    fn navigate_or_drill(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(state) = &self.disk_usage else {
            self.navigate_to(path, cx);
            return;
        };

        if !path.starts_with(&state.mount_point) {
            self.op_error = Some(format!("{} is not on this disk", path.display()));
            cx.notify();
            return;
        }

        self.drill_into(path, cx);
    }

    pub fn select_next_suggestion(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = &mut self.editing_path else {
            return;
        };
        if editing.suggestions.is_empty() {
            return;
        }
        let len = editing.suggestions.len();
        editing.selected_suggestion = Some(match editing.selected_suggestion {
            Some(ix) => (ix + 1) % len,
            None => 0,
        });
        cx.notify();
    }

    pub fn select_prev_suggestion(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = &mut self.editing_path else {
            return;
        };
        if editing.suggestions.is_empty() {
            return;
        }
        let len = editing.suggestions.len();
        editing.selected_suggestion = Some(match editing.selected_suggestion {
            Some(0) => len - 1,
            Some(ix) => ix - 1,
            None => len - 1,
        });
        cx.notify();
    }

    pub fn cancel_edit_path(&mut self, cx: &mut Context<Self>) {
        if self.editing_path.take().is_some() {
            cx.notify();
        }
    }

    pub fn accept_path_suggestion(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.editing_path = None;
        self.op_error = None;
        self.navigate_or_drill(path, cx);
    }

    pub fn complete_path_suggestion(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = &self.editing_path else {
            return;
        };
        let chosen = editing
            .selected_suggestion
            .or_else(|| (!editing.suggestions.is_empty()).then_some(0))
            .and_then(|ix| editing.suggestions.get(ix))
            .cloned();
        let Some(chosen) = chosen else {
            return;
        };
        let current = editing.input.read(cx).value().to_string();
        let mut text = chosen.to_string_lossy().into_owned();
        text.push('/');
        if text == current {
            return;
        }
        editing
            .input
            .clone()
            .update(cx, |input, cx| input.set_content(text, cx));
    }

    pub fn update_path_suggestions(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = &self.editing_path else {
            return;
        };
        let text = editing.input.read(cx).value().to_string();
        let suggestions = path_completions(&text);
        if let Some(editing) = &mut self.editing_path {
            editing.selected_suggestion = if suggestions.is_empty() {
                None
            } else {
                Some(0)
            };
            editing.suggestions = suggestions;
        }
        cx.notify();
    }
}
