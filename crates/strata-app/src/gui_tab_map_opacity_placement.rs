//! Map rail placement and pointer ownership, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:205-260.
use super::SliderControl;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Id, Pos2, Rect, Vec2};

#[derive(Clone, Copy, Default)]
struct Placement {
    offset: Option<Vec2>,
    gesture: Option<Gesture>,
    /// This gesture changed the card. A click that does not move stays false (MENU22).
    gesture_moved: bool,
}
#[derive(Clone, Copy)]
enum Gesture {
    Move { grab: Vec2 },
    Control,
}

#[derive(Clone, Copy)]
enum PhysicalEvent {
    Press(Pos2, egui::PointerButton),
    Move(Pos2),
    Release(egui::PointerButton),
    FocusLost,
}

#[derive(Clone, Copy)]
pub(super) enum FocusRequest {
    Keep(Id),
    Slider(SliderControl),
}

pub(super) fn focus_request(ctx: &egui::Context, kind: super::RailKind) -> Option<FocusRequest> {
    ctx.data(|d| d.get_temp::<Option<FocusRequest>>(state_id(kind).with("focus_request")).flatten())
}

fn restore_keyboard_focus(ctx: &egui::Context, kind: super::RailKind) {
    if let Some(FocusRequest::Keep(owner)) = focus_request(ctx, kind) {
        if ctx.read_response(owner).is_some_and(|r| r.enabled() && r.sense.is_focusable()) {
            ctx.memory_mut(|m| m.request_focus(owner));
            ctx.request_repaint();
        }
    }
}

fn state_id(kind: super::RailKind) -> Id {
    Id::new(match kind {
        super::RailKind::Map => "ka_map_opacity_placement",
        super::RailKind::Drawing => "ka_drawing_opacity_placement",
    })
}

/// Top inset, the clear band at the bottom, and where an unmoved card sits above that band.
fn rail_insets(kind: super::RailKind, host: Rect, guide: bool) -> (f32, f32, f32) {
    match kind {
        // Map controls keep the bottom 44 points; the default card sits 12 points above them (UI11).
        super::RailKind::Map => (if guide { 56.0 } else { 0.0 }, 44.0, 56.0),
        // The drawing toolbar is 42 points, or 34 when the sheet is short, and 4 points up from the edge.
        super::RailKind::Drawing => {
            let toolbar = if host.height() < 460.0 { 34.0 } else { 42.0 };
            let reserve = toolbar + 8.0;
            (0.0, reserve, reserve + 12.0)
        }
    }
}

fn constrained(host: Rect, size: Vec2, wanted: Pos2, top_inset: f32, bottom_reserve: f32) -> Rect {
    // Keep the card inside the host (target/cpp-ref/src/app/KaLayerOpacityRail.cpp:221-226).
    let lower = host.min + Vec2::new(0.0, top_inset);
    let upper = Pos2::new(host.right() - size.x, host.bottom() - bottom_reserve - size.y).max(lower);
    Rect::from_min_size(wanted.clamp(lower, upper), size)
}

impl StrataAppState {
    pub(crate) fn remember_map_opacity_focus(&self, ctx: &egui::Context) {
        // Capture before a TextEdit processes an outside press; Qt NoFocus leaves that owner unchanged
        // (target/cpp-ref/src/app/KaLayerOpacityRail.cpp:18-28; egui0.31.1 context.rs:1352-1354).
        let owner = ctx.memory(|m| m.focused());
        ctx.data_mut(|d| {
            d.insert_temp(state_id(super::RailKind::Map).with("keyboard"), owner);
            d.insert_temp(state_id(super::RailKind::Map).with("focus_request"), None::<FocusRequest>);
        });
    }

    pub(crate) fn remember_drawing_opacity_focus(&self, ctx: &egui::Context) {
        let owner = ctx.memory(|m| m.focused());
        ctx.data_mut(|d| {
            d.insert_temp(state_id(super::RailKind::Drawing).with("keyboard"), owner);
            d.insert_temp(state_id(super::RailKind::Drawing).with("focus_request"), None::<FocusRequest>);
        });
    }

    fn opacity_rect(&self, ctx: &egui::Context, host: Rect, kind: super::RailKind, guide: bool) -> Rect {
        let state = ctx.data(|d| d.get_temp::<Placement>(state_id(kind)).unwrap_or_default());
        let adjustable = self.rail_title(kind).is_some();
        let size = Vec2::new(260.0_f32.min(host.width()), if adjustable { 86.0 } else { 34.0 });
        let (top_inset, bottom_reserve, default_bottom) = rail_insets(kind, host, guide);
        // A drag wins over a point read from ini. An unmoved card is the lower left (UI11), or the saved point.
        let saved = self.opacity_rail_ini[super::rail_index(kind)];
        let wanted = if let Some(offset) = state.offset {
            host.min + offset
        } else if let Some((x, y)) = saved {
            host.min + Vec2::new(x as f32, y as f32)
        } else {
            Pos2::new(host.left() + 12.0, host.bottom() - size.y - default_bottom)
        };
        constrained(host, size, wanted, top_inset, bottom_reserve)
    }

    pub(crate) fn map_opacity_rect(&self, ctx: &egui::Context, host: Rect, guide: bool) -> Rect {
        self.opacity_rect(ctx, host, super::RailKind::Map, guide)
    }

    pub(crate) fn drawing_opacity_rect(&self, ctx: &egui::Context, host: Rect) -> Rect {
        self.opacity_rect(ctx, host, super::RailKind::Drawing, false)
    }

    fn rail_title(&self, kind: super::RailKind) -> Option<String> {
        match kind {
            super::RailKind::Map => self.right_inspector.selected_layer_key.clone(),
            super::RailKind::Drawing => self.drawing.rail_layer.clone(),
        }
    }

    pub(crate) fn handle_map_opacity_pointer(&self, ui: &egui::Ui, host: Rect, guide: bool) -> bool {
        self.handle_opacity_pointer(ui, host, super::RailKind::Map, guide)
    }

    pub(crate) fn handle_drawing_opacity_pointer(&self, ui: &egui::Ui, host: Rect) -> bool {
        self.handle_opacity_pointer(ui, host, super::RailKind::Drawing, false)
    }

    fn handle_opacity_pointer(&self, ui: &egui::Ui, host: Rect, kind: super::RailKind, guide: bool) -> bool {
        let ctx = ui.ctx();
        let mut card = self.opacity_rect(ctx, host, kind, guide);
        let mut state = ctx.data(|d| d.get_temp::<Placement>(state_id(kind)).unwrap_or_default());
        let keyboard = ctx.data(|d| d.get_temp::<Option<Id>>(state_id(kind).with("keyboard")).flatten());
        let mut focus = focus_request(ctx, kind);
        let title = self.rail_title(kind);
        // Preserve physical order rather than combining final coordinates with an earlier press (:229-260).
        // O(events) per frame; copy only pointer/focus events, never text or feature geometry.
        let (events, down, focused, hover) = ui.input(|i| {
            let events: Vec<_> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::PointerMoved(pos) => Some(PhysicalEvent::Move(*pos)),
                    egui::Event::PointerButton { pos, button, pressed, .. } => Some(if *pressed {
                        PhysicalEvent::Press(*pos, *button)
                    } else {
                        PhysicalEvent::Release(*button)
                    }),
                    egui::Event::WindowFocused(false) => Some(PhysicalEvent::FocusLost),
                    _ => None,
                })
                .collect();
            (events, i.pointer.any_down(), i.focused, i.pointer.hover_pos())
        });
        let over = |pos, card: Rect| card.contains(pos) && ui.clip_rect().contains(pos) && ui.is_enabled() && ctx.layer_id_at(pos) == Some(ui.layer_id());
        let track = |pos, card| (0..2).find(|row| super::slider_rect(card, *row).contains(pos));
        let mut owned = state.gesture.is_some();
        if !focused && owned {
            state.gesture = Some(Gesture::Control);
        }
        for event in events {
            match event {
                PhysicalEvent::Press(pos, button) => {
                    if state.gesture.is_none() && over(pos, card) {
                        let row = track(pos, card).filter(|_| title.is_some());
                        let control = match row {
                            Some(0) => Some(SliderControl::Opacity),
                            Some(1) if title.as_deref().is_some_and(|name| self.rail_brightness_enabled(kind, name)) => Some(SliderControl::Brightness),
                            _ => None,
                        };
                        state.gesture = Some(if button == egui::PointerButton::Primary && row.is_none() {
                            Gesture::Move { grab: pos - card.min }
                        } else {
                            Gesture::Control
                        });
                        if button == egui::PointerButton::Primary {
                            if let Some(control) = control {
                                focus = Some(FocusRequest::Slider(control));
                            }
                        }
                    }
                    if state.gesture.is_some() {
                        owned = true;
                        if focus.is_none() {
                            focus = keyboard.map(FocusRequest::Keep);
                        }
                    }
                }
                PhysicalEvent::Move(pos) => {
                    if let Some(Gesture::Move { grab }) = state.gesture {
                        let (top_inset, bottom_reserve, _) = rail_insets(kind, host, guide);
                        let moved = constrained(host, card.size(), pos - grab, top_inset, bottom_reserve);
                        if moved.min != card.min {
                            state.offset = Some(moved.min - host.min);
                            state.gesture_moved = true;
                            card = moved;
                        }
                    }
                }
                PhysicalEvent::Release(egui::PointerButton::Primary) | PhysicalEvent::FocusLost => {
                    if state.gesture_moved {
                        state.gesture_moved = false;
                        if let Some(offset) = state.offset {
                            self.write_opacity_rail(kind, offset);
                        }
                    }
                    if state.gesture.is_some() {
                        state.gesture = Some(Gesture::Control);
                        owned = true;
                    }
                }
                PhysicalEvent::Release(_) => {}
            }
        }
        let hovered = hover.is_some_and(|pos| over(pos, card));
        if matches!(state.gesture, Some(Gesture::Move { .. })) && down {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if hovered {
            ctx.set_cursor_icon(if hover.is_some_and(|pos| track(pos, card).is_some()) && title.is_some() {
                egui::CursorIcon::Default
            } else {
                egui::CursorIcon::Grab
            });
        }
        if !down {
            state.gesture = None;
        }
        ctx.data_mut(|d| {
            d.insert_temp(state_id(kind), state);
            d.insert_temp(state_id(kind).with("focus_request"), focus);
        });
        owned || (hovered && !down)
    }

    pub(crate) fn paint_map_opacity_rail(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, p: &egui::Painter, host: Rect, guide: bool) {
        self.paint_opacity_rail(ctx, ui, p, host, super::RailKind::Map, guide);
    }

    pub(crate) fn paint_drawing_opacity_rail(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, p: &egui::Painter, host: Rect) {
        self.paint_opacity_rail(ctx, ui, p, host, super::RailKind::Drawing, false);
    }

    fn paint_opacity_rail(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, p: &egui::Painter, host: Rect, kind: super::RailKind, guide: bool) {
        let card = self.opacity_rect(ctx, host, kind, guide);
        let card_id = match kind {
            super::RailKind::Map => "ka_opacity_card",
            super::RailKind::Drawing => "ka_drawing_opacity_card",
        };
        // Register before sliders: blank background owns input without keyboard focus (:18-28,229-260).
        ui.interact(card, ui.id().with(card_id), egui::Sense::CLICK | egui::Sense::DRAG);
        if let Some(title) = self.rail_title(kind) {
            self.paint_opacity_card(ctx, ui, p, card, &title, kind);
        } else {
            p.rect_filled(card, 6.0, egui::Color32::from_rgba_unmultiplied(252, 251, 247, 248));
            p.rect_stroke(card, 6.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
            p.text(
                card.min + Vec2::new(12.0, card.height() * 0.5),
                egui::Align2::LEFT_CENTER,
                "투명도·밝기: 조절할 레이어를 고르세요",
                egui::FontId::proportional(11.5),
                egui::Color32::from_rgb(65, 64, 60),
            );
        }
        restore_keyboard_focus(ctx, kind);
    }

    /// `updateLayerOpacityControl` copies the current layer into `m_railLayer` (`KaDrawingStudio.cpp:2562-2566`).
    /// A slider gesture keeps that layer: a sheet click can change the tree selection underneath (`:1199-1201`).
    pub(crate) fn sync_drawing_rail_layer(&mut self, ctx: &egui::Context) {
        let busy = ctx.data(|d| d.get_temp::<Placement>(state_id(super::RailKind::Drawing)).is_some_and(|state| state.gesture.is_some()));
        if busy {
            return;
        }
        self.drawing.rail_layer = self.right_inspector.selected_layer_key.clone().filter(|title| self.legend_row_index(title).is_some());
    }
}
