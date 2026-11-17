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
}

impl CanvasAction {
    fn key(&self) -> Key {
        match self {
            Self::Escape(_) => Key::Escape,
            Self::FinishCapture => Key::Enter,
            Self::UndoCaptureVertex(key) => *key,
        }
    }
}

#[must_use]
pub(crate) struct PendingCanvasInput(Option<(MapToolKind, MainTab, CanvasAction)>);

impl StrataAppState {
    pub(super) fn pending_canvas_input(&self, ctx: &egui::Context) -> PendingCanvasInput {
        let escape = self.pending_canvas_escape(ctx).map(CanvasAction::Escape);
        let action = if ctx.input(|i| i.key_pressed(Key::Escape)) {
            escape
        } else if !ctx.wants_keyboard_input() && self.modals.active.is_none() && self.capture_active() && !self.app_popup_menu_open() && !self.ribbon.app_bar.address_popup_open {
            ctx.input(|i| {
                if i.key_pressed(Key::Enter) {
                    Some(CanvasAction::FinishCapture)
                } else if i.key_pressed(Key::Delete) {
                    Some(CanvasAction::UndoCaptureVertex(Key::Delete))
                } else if i.key_pressed(Key::Backspace) {
                    Some(CanvasAction::UndoCaptureVertex(Key::Backspace))
                } else {
                    None
                }
            })
        } else {
            None
        };
        PendingCanvasInput(action.map(|action| (self.active_tool, self.active_tab, action)))
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
    pub(crate) fn apply(self, app: &mut StrataAppState, ctx: &egui::Context) {
        let Some((tool, tab, action)) = self.0 else {
            return;
        };
        // target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391: only the original canvas owner can edit capture.
        if !ctx.input(|i| i.key_pressed(action.key())) || app.active_tool != tool || app.active_tab != tab || app.modal_window_open() {
            return;
        }
        match action {
            CanvasAction::FinishCapture => app.sketch_finish(),
            CanvasAction::UndoCaptureVertex(_) => {
                app.sketch_undo_vertex();
            }
            CanvasAction::Escape(action) => match action {
                EscapeAction::AlignPoint => {
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
}
