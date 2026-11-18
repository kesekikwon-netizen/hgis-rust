//! Canvas keys after popup owners, target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391.
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Key};
use strata_canvas2d::map_tools::MapToolKind;

enum EscapeAction {
    AlignPoint,
    Capture,
    Vertex,
    Tool,
}

enum CanvasAction {
    Escape(EscapeAction),
    FinishCapture,
    UndoCaptureVertex(Key),
    DeleteMapSelection,
    BackspaceMap,
}

impl CanvasAction {
    fn key(&self) -> Key {
        match self {
            Self::Escape(_) => Key::Escape,
            Self::FinishCapture => Key::Enter,
            Self::UndoCaptureVertex(key) => *key,
            Self::DeleteMapSelection => Key::Delete,
            Self::BackspaceMap => Key::Backspace,
        }
    }
}

#[must_use]
pub(crate) struct PendingCanvasInput {
    owner: (MapToolKind, MainTab),
    action: Option<CanvasAction>,
    history: Option<HistoryAction>,
}

pub(super) enum HistoryAction {
    UndoMap,
    RedoMap,
    UndoCoordinates,
}

impl HistoryAction {
    fn key(&self) -> Key {
        match self {
            Self::UndoMap | Self::UndoCoordinates => Key::Z,
            Self::RedoMap => Key::Y,
        }
    }
}

impl StrataAppState {
    pub(super) fn pending_canvas_input(&self, ctx: &egui::Context) -> PendingCanvasInput {
        let escape = self.pending_canvas_escape(ctx).map(CanvasAction::Escape);
        let action = if ctx.input(|i| i.key_pressed(Key::Escape)) {
            escape
        } else if !ctx.wants_keyboard_input() && self.modals.active.is_none() && !self.modal_window_open() && !self.app_popup_menu_open() && !self.ribbon.app_bar.address_popup_open {
            ctx.input(|i| {
                if self.capture_active() {
                    if i.key_pressed(Key::Enter) {
                        Some(CanvasAction::FinishCapture)
                    } else if i.key_pressed(Key::Delete) {
                        Some(CanvasAction::UndoCaptureVertex(Key::Delete))
                    } else if i.key_pressed(Key::Backspace) {
                        Some(CanvasAction::UndoCaptureVertex(Key::Backspace))
                    } else {
                        None
                    }
                } else if self.active_tab == MainTab::Map && self.active_tool != MapToolKind::GeorefAlign {
                    if i.key_pressed(Key::Delete) {
                        Some(CanvasAction::DeleteMapSelection)
                    } else if i.key_pressed(Key::Backspace) {
                        Some(CanvasAction::BackspaceMap)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        } else {
            None
        };
        PendingCanvasInput { owner: (self.active_tool, self.active_tab), action, history: None }
    }

    fn pending_canvas_escape(&self, ctx: &egui::Context) -> Option<EscapeAction> {
        // target/cpp-ref/src/app/KaRegionLocator.cpp:106-120: popup/modal Escape never reaches the canvas.
        if !ctx.input(|i| i.key_pressed(Key::Escape)) || self.modal_window_open() || self.app_popup_menu_open() || self.ribbon.app_bar.address_popup_open {
            None
        } else if !ctx.wants_keyboard_input() && self.active_tool == MapToolKind::GeorefAlign {
            Some(EscapeAction::AlignPoint)
        } else if !ctx.wants_keyboard_input() && self.modals.active.is_none() && self.capture_active() {
            Some(EscapeAction::Capture)
        } else if self.home_map.active_vertex.is_some() {
            Some(EscapeAction::Vertex)
        } else if self.home_map.inline_rename.is_none() {
            Some(EscapeAction::Tool)
        } else {
            None
        }
    }
}

impl PendingCanvasInput {
    pub(super) fn with_history(mut self, app: &StrataAppState, ctx: &egui::Context, history: Option<HistoryAction>) -> Self {
        // target/cpp-ref/src/app/MainWindowUndo.cpp:325-484: keep the original shortcut routing and edit-before-history order.
        if !ctx.input(|i| i.key_pressed(Key::Escape)) && !ctx.wants_keyboard_input() && !app.modal_window_open() && !app.app_popup_menu_open() && !app.ribbon.app_bar.address_popup_open {
            self.history = history;
        }
        self
    }

    pub(crate) fn apply(self, app: &mut StrataAppState, ctx: &egui::Context) {
        let (tool, tab) = self.owner;
        // target/cpp-ref/src/app/MainWindow.cpp:256-268 and KaCaptureMapTool.cpp:379-391 keep the original editor owner.
        if app.active_tool != tool || app.active_tab != tab || app.modal_window_open() {
            return;
        }
        if let Some(action) = self.action.filter(|action| ctx.input(|i| i.key_pressed(action.key()))) {
            match action {
                CanvasAction::DeleteMapSelection => {
                    // target/cpp-ref/src/app/MainWindow.cpp:256-268 and MainWindowUndo.cpp:497-500 preserve vertex/legend/map routing.
                    if !app.handle_vertex_delete_key() {
                        app.delete_selected();
                    }
                }
                CanvasAction::BackspaceMap => {
                    // target/cpp-ref/src/app/KaFeatureSelectTool.cpp:158-170: the picked vertex owns Backspace.
                    if !app.handle_vertex_delete_key() {
                        app.home_map.cad.on_backspace(&mut app.home_map.scene);
                    }
                }
                CanvasAction::FinishCapture => app.sketch_finish(),
                CanvasAction::UndoCaptureVertex(_) => {
                    app.sketch_undo_vertex();
                }
                CanvasAction::Escape(action) => match action {
                    EscapeAction::AlignPoint => {
                        app.align.cancel_target_action();
                        app.align.pending_source = None;
                        app.status_bar_hint = app.align.status_text();
                    }
                    EscapeAction::Capture => app.sketch_cancel(),
                    EscapeAction::Vertex => {
                        // target/cpp-ref/src/app/KaFeatureSelectTool.cpp:164-170 keeps the tool when releasing a picked vertex.
                        app.home_map.active_vertex = None;
                    }
                    EscapeAction::Tool => {
                        app.home_map.cad.on_escape(&mut app.home_map.scene);
                        app.modals.active = None;
                        app.ribbon.sub_toolbar_visible = false;
                        app.active_tool = MapToolKind::Pan;
                    }
                },
            }
            ctx.request_repaint();
        }
        if app.active_tool != tool || app.active_tab != tab || app.modal_window_open() {
            return;
        }
        if let Some(history) = self.history.filter(|history| ctx.input(|i| i.modifiers.ctrl && i.key_pressed(history.key()))) {
            match history {
                HistoryAction::UndoMap => app.undo_map_action(),
                HistoryAction::RedoMap => app.redo_map_action(),
                HistoryAction::UndoCoordinates => {
                    app.undo_last_coord_callout();
                }
            }
            ctx.request_repaint();
        }
    }
}
