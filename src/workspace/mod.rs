pub mod pane;
pub mod split_tree;
pub mod tab_drag;

use gpui::{
    App, Bounds, Context, Entity, EntityId, IntoElement, MouseButton, ParentElement, Pixels,
    Point, Render, ScrollHandle, Styled, Subscription, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};

use crate::app::window_root::WindowRoot;
use crate::cli::Startup;
use crate::explorer::Explorer;
use crate::explorer::drag::{ScrollDragHost, ScrollbarDrag, ScrollbarId, WidthResizeDrag};
use crate::explorer::shared_state::SharedState;
use crate::keys;
use crate::settings::{DiskUsageSettings, GitSettings, SearchSettings, SidebarItem};
use crate::theme;
use crate::ui::sidebar;

use pane::{Pane, PaneEvent};
use split_tree::{Axis, MIN_PANE_FRACTION, SplitNode};
use tab_drag::{DragHover, SplitZone, TabDragPayload};

pub const MIN_SIDEBAR_WIDTH: f32 = 140.0;
pub const MAX_SIDEBAR_WIDTH: f32 = 480.0;
pub const DEFAULT_SIDEBAR_WIDTH: f32 = 300.0;
pub const FORCE_TAB_STRIP: bool = true;

#[derive(Clone, Debug)]
pub struct SplitResizeState {
    pub path: Vec<usize>,
    pub left_index: usize,
    pub anchor_pos: f32,
    pub start_fractions: (f32, f32),
}

pub struct Workspace {
    pub sidebar_visible: bool,
    pub sidebar_entries: Vec<SidebarItem>,
    pub sidebar_width: f32,
    pub sidebar_resize_drag: Option<WidthResizeDrag>,
    pub sidebar_scroll_handle: ScrollHandle,
    scrollbar_drag: Option<ScrollbarDrag>,
    root: SplitNode,
    drag_hover: Option<DragHover>,
    dragging_tab: Option<TabDragPayload>,
    split_resize: Option<SplitResizeState>,
    git_settings: GitSettings,
    disk_usage_settings: DiskUsageSettings,
    search_settings: SearchSettings,
    shared: Entity<SharedState>,
    pane_subscriptions: Vec<Subscription>,
}

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        show_hidden: bool,
        sidebar_visible: bool,
        sidebar_entries: Vec<SidebarItem>,
        git_settings: GitSettings,
        disk_usage_settings: DiskUsageSettings,
        search_settings: SearchSettings,
        shared: Entity<SharedState>,
        startup: Startup,
    ) -> Self {
        let explorer = cx.new(|cx| {
            Explorer::new(
                window,
                cx,
                show_hidden,
                git_settings.clone(),
                disk_usage_settings.clone(),
                search_settings.clone(),
                shared.clone(),
                startup,
            )
        });
        let pane = cx.new(|cx| {
            Pane::new(
                explorer,
                git_settings.clone(),
                disk_usage_settings.clone(),
                search_settings.clone(),
                shared.clone(),
                cx,
            )
        });

        Self::from_pane(
            pane,
            sidebar_visible,
            sidebar_entries,
            git_settings,
            disk_usage_settings,
            search_settings,
            shared,
            cx,
        )
    }

    /// Builds a workspace around an already-existing pane — used when tearing a tab off into
    /// its own OS window, where the pane (and its tab) already exist and just need a new home.
    #[allow(clippy::too_many_arguments)]
    pub fn from_pane(
        pane: Entity<Pane>,
        sidebar_visible: bool,
        sidebar_entries: Vec<SidebarItem>,
        git_settings: GitSettings,
        disk_usage_settings: DiskUsageSettings,
        search_settings: SearchSettings,
        shared: Entity<SharedState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self {
            sidebar_visible,
            sidebar_entries,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_resize_drag: None,
            sidebar_scroll_handle: ScrollHandle::new(),
            scrollbar_drag: None,
            root: SplitNode::Leaf(pane.clone()),
            drag_hover: None,
            dragging_tab: None,
            split_resize: None,
            git_settings,
            disk_usage_settings,
            search_settings,
            shared,
            pane_subscriptions: Vec::new(),
        };
        this.subscribe_pane(&pane, cx);
        this
    }

    fn subscribe_pane(&mut self, pane: &Entity<Pane>, cx: &mut Context<Self>) {
        let subscription = cx.subscribe(pane, |workspace, pane, event, cx| match event {
            PaneEvent::Empty => workspace.remove_pane(pane.entity_id(), cx),
            PaneEvent::TabDropped => {
                workspace.drag_hover = None;
                cx.notify();
            }
        });
        self.pane_subscriptions.push(subscription);
    }

    fn remove_pane(&mut self, id: EntityId, cx: &mut Context<Self>) {
        if self.root.is_leaf(id) {
            // The only pane left in the workspace — nothing to fall back to, so veto the close.
            cx.notify();
            return;
        }
        self.root.remove_leaf(id);
        cx.notify();
    }

    /// The pane whose active tab currently holds keyboard focus, falling back to the first
    /// leaf (in tree order) if nothing is focused yet.
    fn focused_pane(&self, window: &Window, cx: &App) -> Entity<Pane> {
        let leaves = self.root.leaves();
        for pane in &leaves {
            let focus_handle = pane.read(cx).active_tab().read(cx).focus_handle.clone();
            if focus_handle.is_focused(window) {
                return pane.clone();
            }
        }
        leaves.into_iter().next().expect("workspace always has at least one pane")
    }

    pub fn handle_tab_drop(&mut self, dragged: TabDragPayload, target_pane: Entity<Pane>, cx: &mut Context<Self>) {
        let target_id = target_pane.entity_id();
        let zone = self
            .drag_hover
            .take()
            .filter(|hover| hover.pane_id == target_id)
            .map(|hover| hover.zone)
            .unwrap_or(SplitZone::Center);

        let same_pane = dragged.source_pane.entity_id() == target_id;
        let source_tab_count = dragged.source_pane.read(cx).tabs.len();
        if same_pane && (zone == SplitZone::Center || source_tab_count <= 1) {
            // Either dropped back into the middle of the pane it came from, or it's the only
            // tab that pane has — there's nothing on the other side of the pane to split against.
            cx.notify();
            return;
        }

        let Some(explorer) = dragged
            .source_pane
            .update(cx, |pane, cx| pane.take_tab(dragged.tab_ix, cx))
        else {
            cx.notify();
            return;
        };

        match zone {
            SplitZone::Center => {
                target_pane.update(cx, |pane, cx| {
                    let index = pane.tabs.len();
                    pane.insert_tab(index, explorer, cx);
                });
            }
            SplitZone::Left | SplitZone::Right | SplitZone::Top | SplitZone::Bottom => {
                let axis = match zone {
                    SplitZone::Left | SplitZone::Right => Axis::Horizontal,
                    _ => Axis::Vertical,
                };
                let new_first = matches!(zone, SplitZone::Left | SplitZone::Top);
                let new_pane = cx.new(|cx| {
                    Pane::new(
                        explorer,
                        self.git_settings.clone(),
                        self.disk_usage_settings.clone(),
                        self.search_settings.clone(),
                        self.shared.clone(),
                        cx,
                    )
                });
                self.subscribe_pane(&new_pane, cx);
                self.root.split_leaf(target_id, axis, new_pane, new_first);
            }
        }

        cx.notify();
    }

    /// Called continuously while a tab drag is in progress, so that a drop outside the window
    /// (which no `on_drop` target ever sees) still knows which tab was being dragged.
    pub fn track_dragging_tab(&mut self, dragged: TabDragPayload) {
        self.dragging_tab = Some(dragged);
    }

    /// Tears the currently-dragged tab out into a brand new OS window, positioned at roughly
    /// where it was dropped. No-op if it's the only tab in the only pane of this workspace,
    /// since there would be nothing left behind.
    pub fn tear_off_dragging_tab(&mut self, screen_position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(dragged) = self.dragging_tab.take() else {
            return;
        };

        let is_only_pane = self.root.is_leaf(dragged.source_pane.entity_id());
        let source_tab_count = dragged.source_pane.read(cx).tabs.len();
        if is_only_pane && source_tab_count <= 1 {
            return;
        }

        let Some(explorer) = dragged
            .source_pane
            .update(cx, |pane, cx| pane.take_tab(dragged.tab_ix, cx))
        else {
            return;
        };

        let sidebar_visible = self.sidebar_visible;
        let sidebar_entries = self.sidebar_entries.clone();
        let git_settings = self.git_settings.clone();
        let disk_usage_settings = self.disk_usage_settings.clone();
        let search_settings = self.search_settings.clone();
        let shared = self.shared.clone();

        let bounds = Bounds::new(screen_position, size(px(1000.0), px(650.0)));
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                let focus_handle = explorer.read(cx).focus_handle.clone();
                window.focus(&focus_handle);
                let pane = cx.new(|cx| {
                    Pane::new(
                        explorer,
                        git_settings.clone(),
                        disk_usage_settings.clone(),
                        search_settings.clone(),
                        shared.clone(),
                        cx,
                    )
                });
                let workspace = cx.new(|cx| {
                    Workspace::from_pane(
                        pane,
                        sidebar_visible,
                        sidebar_entries,
                        git_settings,
                        disk_usage_settings,
                        search_settings,
                        shared,
                        cx,
                    )
                });
                cx.new(|_| WindowRoot { content: workspace })
            },
        )
        .ok();

        cx.notify();
    }

    pub fn begin_split_resize(&mut self, path: Vec<usize>, left_index: usize, anchor_pos: f32, cx: &mut Context<Self>) {
        let Some(SplitNode::Split { children, .. }) = self.root.at_path_mut(&path) else {
            return;
        };
        let Some(fraction_a) = children.get(left_index).map(|(_, fraction)| *fraction) else {
            return;
        };
        let Some(fraction_b) = children.get(left_index + 1).map(|(_, fraction)| *fraction) else {
            return;
        };
        self.split_resize = Some(SplitResizeState {
            path,
            left_index,
            anchor_pos,
            start_fractions: (fraction_a, fraction_b),
        });
        cx.notify();
    }

    pub fn update_split_resize(&mut self, left_index: usize, mouse: f32, total_pixels: f32, cx: &mut Context<Self>) {
        let Some(state) = self.split_resize.clone() else {
            return;
        };
        if state.left_index != left_index || total_pixels <= 0.0 {
            return;
        }
        let (start_a, start_b) = state.start_fractions;
        let sum = start_a + start_b;
        if sum <= MIN_PANE_FRACTION * 2.0 {
            return;
        }
        let delta = (mouse - state.anchor_pos) / total_pixels;
        let new_a = (start_a + delta).clamp(MIN_PANE_FRACTION, sum - MIN_PANE_FRACTION);
        let new_b = sum - new_a;

        if let Some(SplitNode::Split { children, .. }) = self.root.at_path_mut(&state.path) {
            if let Some(entry) = children.get_mut(left_index) {
                entry.1 = new_a;
            }
            if let Some(entry) = children.get_mut(left_index + 1) {
                entry.1 = new_b;
            }
        }
        cx.notify();
    }

    pub fn end_split_resize(&mut self, cx: &mut Context<Self>) {
        if self.split_resize.take().is_some() {
            cx.notify();
        }
    }

    pub fn sidebar_should_render(&self) -> bool {
        self.sidebar_visible && !self.sidebar_entries.is_empty()
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    pub fn begin_sidebar_resize(&mut self, anchor_x: f32, cx: &mut Context<Self>) {
        self.sidebar_resize_drag = Some(WidthResizeDrag {
            anchor_x,
            start_width: self.sidebar_width,
        });
        cx.notify();
    }

    pub fn update_sidebar_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let Some(drag) = self.sidebar_resize_drag else {
            return;
        };
        let new_width = drag.start_width + (current_x - drag.anchor_x);
        self.sidebar_width = new_width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
        cx.notify();
    }

    pub fn end_sidebar_resize(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_resize_drag.take().is_some() {
            cx.notify();
        }
    }
}

impl ScrollDragHost for Workspace {
    fn begin_scrollbar_drag(&mut self, id: ScrollbarId, grab_offset: Pixels, cx: &mut Context<Self>) {
        self.scrollbar_drag = Some(ScrollbarDrag { id, grab_offset });
        cx.notify();
    }

    fn scrollbar_grab_offset(&self, id: ScrollbarId) -> Option<Pixels> {
        self.scrollbar_drag
            .filter(|drag| drag.id == id)
            .map(|drag| drag.grab_offset)
    }

    fn end_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.scrollbar_drag.take().is_some() {
            cx.notify();
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.focused_pane(window, cx).read(cx).active_tab();
        let entity = cx.entity();

        let leaves = self.root.leaves();
        let total_tabs: usize = leaves.iter().map(|pane| pane.read(cx).tabs.len()).sum();
        let show_tab_strip = FORCE_TAB_STRIP || total_tabs > 1;
        for pane in &leaves {
            pane.update(cx, |pane, cx| {
                if pane.show_tab_strip != show_tab_strip {
                    pane.show_tab_strip = show_tab_strip;
                    cx.notify();
                }
            });
        }

        let tree = split_tree::render(self, &self.root, Vec::new(), &entity);

        div()
            .id("workspace-root")
            .key_context("Workspace")
            .on_action(cx.listener(|workspace, _: &keys::ToggleSidebar, _window, cx| {
                workspace.toggle_sidebar(cx);
            }))
            .on_action(cx.listener(|workspace, _: &keys::NewTab, window, cx| {
                let pane = workspace.focused_pane(window, cx);
                pane.update(cx, |pane, cx| pane.spawn_new_tab(window, cx));
            }))
            .on_action(cx.listener(|workspace, _: &keys::CloseTab, window, cx| {
                let pane = workspace.focused_pane(window, cx);
                pane.update(cx, |pane, cx| pane.close_active_tab(cx));
            }))
            .on_action(cx.listener(|workspace, _: &keys::NextTab, window, cx| {
                let pane = workspace.focused_pane(window, cx);
                pane.update(cx, |pane, cx| pane.next_tab(cx));
            }))
            .on_action(cx.listener(|workspace, _: &keys::PrevTab, window, cx| {
                let pane = workspace.focused_pane(window, cx);
                pane.update(cx, |pane, cx| pane.prev_tab(cx));
            }))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|workspace, _event, window, cx| {
                    let screen_position = window.bounds().origin + window.mouse_position();
                    workspace.tear_off_dragging_tab(screen_position, cx);
                }),
            )
            .size_full()
            .flex()
            .flex_row()
            .bg(theme::bg_window())
            .text_color(theme::text_primary())
            .when(self.sidebar_should_render(), |this| {
                this.child(sidebar::render(self, active, window, cx))
            })
            .child(div().flex().flex_col().flex_1().size_full().child(tree))
            .when(self.sidebar_should_render(), |this| {
                this.child(sidebar::resize_handle(
                    cx.entity(),
                    self.sidebar_width,
                    self.sidebar_resize_drag.is_some(),
                    cx,
                ))
            })
    }
}
