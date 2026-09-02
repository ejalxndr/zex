use std::path::PathBuf;

use gpui::{Context, Task, Window};

use crate::filesystem::open_with::{self, DesktopApp};

use super::Explorer;

const DISCOVER_TIMEOUT_MS: u64 = 3000;

pub struct OpenWithState {
    pub path: PathBuf,
    pub apps: Vec<DesktopApp>,
    pub loading: bool,
    task: Option<Task<()>>,
}

impl Explorer {
    pub fn begin_open_with(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_with = Some(OpenWithState {
            path: path.clone(),
            apps: Vec::new(),
            loading: true,
            task: None,
        });

        let task = cx.spawn(async move |weak, cx| {
            let apps = cx
                .background_executor()
                .spawn(async move { open_with::discover_apps_for(&path, DISCOVER_TIMEOUT_MS) })
                .await;

            let _ = weak.update(cx, |explorer, cx| {
                if let Some(state) = &mut explorer.open_with {
                    state.apps = apps;
                    state.loading = false;
                    cx.notify();
                }
            });
        });

        if let Some(state) = &mut self.open_with {
            state.task = Some(task);
        }
        cx.notify();
    }

    pub fn launch_with_app(&mut self, app: DesktopApp, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = self.open_with.take() else { return };
        window.focus(&self.focus_handle);
        if let Err(err) = open_with::launch(&state.path, &app) {
            self.op_error = Some(format!("Couldn't open {}: {err}", state.path.display()));
        }
        cx.notify();
    }

    pub fn cancel_open_with(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_with.take().is_some() {
            window.focus(&self.focus_handle);
            cx.notify();
        }
    }
}
