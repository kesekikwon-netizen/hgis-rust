//! Map rail placement and pointer ownership, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:205-260.
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
        ctx.data_mut(|d| d.insert_temp(state_id().with("keyboard"), owner));
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
        let card = self.map_opacity_rect(ctx, host, guide);
        let mut state = ctx.data(|d| d.get_temp::<Placement>(state_id()).unwrap_or_default());
        let (pos, pressed, primary, down, primary_down, focused) = ui.input(|i| (i.pointer.interact_pos(), i.pointer.any_pressed(), i.pointer.primary_pressed(), i.pointer.any_down(), i.pointer.primary_down(), i.focused));
        let over = pos.is_some_and(|p| card.contains(p) && ui.clip_rect().contains(p) && ctx.layer_id_at(p) == Some(ui.layer_id())) && ui.is_enabled();
        let on_slider = self.right_inspector.selected_layer_key.is_some() && pos.is_some_and(|p| (card.top() + 30.0..=card.top() + 74.0).contains(&p.y) && (card.left() + 52.0..=card.right() - 42.0).contains(&p.x));
        if let Some(pos) = pos.filter(|_| pressed && over && state.gesture.is_none()) {
            if let Some(owner) = ctx.data(|d| d.get_temp::<Option<Id>>(state_id().with("keyboard")).flatten()) {
                ctx.memory_mut(|m| m.request_focus(owner));
            }
            state.gesture = Some(if primary && !on_slider { Gesture::Move { grab: pos - card.min } } else { Gesture::Control });
        }
        let owned = state.gesture.is_some() || (over && !down);
        // A lost OS focus interrupts movement, but this press stays owned until release (:254-260).
        if (!focused || (!primary_down && matches!(state.gesture, Some(Gesture::Move { .. })))) && state.gesture.is_some() {
            state.gesture = Some(Gesture::Control);
        }
        if let (Some(Gesture::Move { grab }), Some(pos), true) = (state.gesture, pos, down) {
            if ui.input(|i| i.pointer.delta() != Vec2::ZERO) {
                state.offset = Some(constrained(host, card.size(), pos - grab, guide).min - host.min);
            }
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if over {
            ctx.set_cursor_icon(if on_slider { egui::CursorIcon::Default } else { egui::CursorIcon::Grab });
        }
        if !down {
            state.gesture = None;
        }
        ctx.data_mut(|d| d.insert_temp(state_id(), state));
        owned
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
    }
}
