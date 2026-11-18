//! Map rail placement and pointer ownership, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:205-260.
use super::SliderControl;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Id, Pos2, Rect, Vec2};

#[derive(Clone, Copy, Default)]
struct Placement {
    offset: Option<Vec2>,
    gesture: Option<Gesture>,
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

pub(super) fn focus_request(ctx: &egui::Context) -> Option<FocusRequest> {
    ctx.data(|d| d.get_temp::<Option<FocusRequest>>(state_id().with("focus_request")).flatten())
}

fn restore_keyboard_focus(ctx: &egui::Context) {
    if let Some(FocusRequest::Keep(owner)) = focus_request(ctx) {
        if ctx.read_response(owner).is_some_and(|r| r.enabled() && r.sense.is_focusable()) {
            ctx.memory_mut(|m| m.request_focus(owner));
            ctx.request_repaint();
        }
    }
}

fn state_id() -> Id {
    Id::new("ka_map_opacity_placement")
}

fn constrained(host: Rect, size: Vec2, wanted: Pos2, guide: bool) -> Rect {
    // Keep the existing guide and bottom scale controls usable (target/cpp-ref/src/app/KaLayerOpacityRail.cpp:221-226).
    let lower = host.min + Vec2::new(0.0, if guide { 56.0 } else { 0.0 });
    let upper = Pos2::new(host.right() - size.x, host.bottom() - 44.0 - size.y).max(lower);
    Rect::from_min_size(wanted.clamp(lower, upper), size)
}

impl StrataAppState {
    pub(crate) fn remember_map_opacity_focus(&self, ctx: &egui::Context) {
        // Capture before a TextEdit processes an outside press; Qt NoFocus leaves that owner unchanged
        // (target/cpp-ref/src/app/KaLayerOpacityRail.cpp:18-28; egui0.31.1 context.rs:1352-1354).
        let owner = ctx.memory(|m| m.focused());
        ctx.data_mut(|d| {
            d.insert_temp(state_id().with("keyboard"), owner);
            d.insert_temp(state_id().with("focus_request"), None::<FocusRequest>);
        });
    }

    pub(crate) fn map_opacity_rect(&self, ctx: &egui::Context, host: Rect, guide: bool) -> Rect {
        let state = ctx.data(|d| d.get_temp::<Placement>(state_id()).unwrap_or_default());
        let size = Vec2::new(260.0_f32.min(host.width()), if self.right_inspector.selected_layer_key.is_some() { 86.0 } else { 34.0 });
        // Owner UI11 overrides the C++ default corner; the bottom forty-four points belong to map controls.
        let wanted = state.offset.map_or(Pos2::new(host.left() + 12.0, host.bottom() - size.y - 56.0), |offset| host.min + offset);
        constrained(host, size, wanted, guide)
    }

    pub(crate) fn handle_map_opacity_pointer(&self, ui: &egui::Ui, host: Rect, guide: bool) -> bool {
        let ctx = ui.ctx();
        let mut card = self.map_opacity_rect(ctx, host, guide);
        let mut state = ctx.data(|d| d.get_temp::<Placement>(state_id()).unwrap_or_default());
        let keyboard = ctx.data(|d| d.get_temp::<Option<Id>>(state_id().with("keyboard")).flatten());
        let mut focus = focus_request(ctx);
        // Preserve physical order rather than combining final coordinates with an earlier press (:229-260).
        // O(events) per frame; copy only pointer/focus events, never text or feature geometry.
        let (events, down, focused, hover) = ui.input(|i| {
            let events: Vec<_> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::PointerMoved(pos) => Some(PhysicalEvent::Move(*pos)),
                    egui::Event::PointerButton { pos, button, pressed, .. } => Some(if *pressed { PhysicalEvent::Press(*pos, *button) } else { PhysicalEvent::Release(*button) }),
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
                        let row = track(pos, card).filter(|_| self.right_inspector.selected_layer_key.is_some());
                        let control = match row {
                            Some(0) => Some(SliderControl::Opacity),
                            Some(1) if self.right_inspector.selected_layer_key.as_deref().is_some_and(super::is_raster_image_layer) => Some(SliderControl::Brightness),
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
                        let moved = constrained(host, card.size(), pos - grab, guide);
                        if moved.min != card.min {
                            state.offset = Some(moved.min - host.min);
                            card = moved;
                        }
                    }
                }
                PhysicalEvent::Release(egui::PointerButton::Primary) | PhysicalEvent::FocusLost => {
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
            ctx.set_cursor_icon(if hover.is_some_and(|pos| track(pos, card).is_some()) && self.right_inspector.selected_layer_key.is_some() {
                egui::CursorIcon::Default
            } else {
                egui::CursorIcon::Grab
            });
        }
        if !down {
            state.gesture = None;
        }
        ctx.data_mut(|d| {
            d.insert_temp(state_id(), state);
            d.insert_temp(state_id().with("focus_request"), focus);
        });
        owned || (hovered && !down)
    }

    pub(crate) fn paint_map_opacity_rail(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, p: &egui::Painter, host: Rect, guide: bool) {
        let card = self.map_opacity_rect(ctx, host, guide);
        // Register after map controls, before sliders: blank background owns input without keyboard focus (:18-28,229-260).
        ui.interact(card, ui.id().with("ka_opacity_card"), egui::Sense::CLICK | egui::Sense::DRAG);
        if let Some(title) = self.right_inspector.selected_layer_key.clone() {
            self.paint_opacity_card(ctx, ui, p, card, &title);
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
        restore_keyboard_focus(ctx);
    }
}
