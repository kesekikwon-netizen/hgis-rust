//! The handles of the selected shape on the map (C++ `KaVertexEditTool` markers and the vertex part of
//! `KaFeatureSelectTool`): a box on every vertex, the clicked one red; a held handle moves its vertex; a
//! right click on a handle offers 「점삭제」 and on a side 「점추가」.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;

use super::gui_tab_map_geom::{s2w, w2s};
use crate::app_state::StrataAppState;

/// `KaEditTolerance.h`: a handle is grabbed within 10 screen points, the menu finds a vertex within 12 and a
/// side within 10, and a press that moves no further than 4 points is a click.
const GRAB_PX: f64 = 10.0;
const MENU_VERTEX_PX: f64 = 12.0;
const MENU_SIDE_PX: f64 = 10.0;
const CLICK_SLOP_PX: f32 = 4.0;

/// A handle held with the mouse: its vertex, where the press was, and where the vertex is now (work CRS)
/// once the pointer left the place of the press.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HeldVertex {
    index: usize,
    pressed_at: [f32; 2],
    to: Option<[f64; 2]>,
}

/// The one-row menu of a right click on a handle (「점삭제」) or on a side (「점추가」).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VertexMenu {
    at: [f32; 2],
    place: [f64; 2],
    vertex: Option<usize>,
}

impl StrataAppState {
    /// The pointer on the handles while 「도형선택」 is on. `true` while a handle is held and in the frame it
    /// is let go: the map does not pan then, and the click is not a new selection.
    pub(crate) fn handle_vertex_handles(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) -> bool {
        if self.active_tool != MapToolKind::Select || self.modal_window_open() || self.vertex_target().is_none() {
            (self.home_map.held_vertex, self.home_map.active_vertex) = (None, None);
            return false;
        }
        let (pressed, down, at) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.interact_pos()));
        let on_map = |at: Pos2| s2w(rect, &self.home_map.viewport, at);
        if let (true, true, Some(at)) = (pressed, response.is_pointer_button_down_on(), at) {
            if let Some(index) = self.vertex_at(on_map(at), GRAB_PX) {
                self.home_map.held_vertex = Some(HeldVertex { index, pressed_at: [at.x, at.y], to: None });
                self.home_map.active_vertex = Some(index);
            }
        }
        let Some(mut held) = self.home_map.held_vertex.take() else { return false };
        if let Some(at) = at {
            let far = (at.x - held.pressed_at[0]).abs() + (at.y - held.pressed_at[1]).abs() > CLICK_SLOP_PX;
            if far || held.to.is_some() {
                held.to = Some(on_map(at));
            }
        }
        if down {
            self.home_map.held_vertex = Some(held);
        } else if let Some(to) = held.to {
            self.move_vertex_to(held.index, to);
        } else {
            self.pick_vertex(held.index);
        }
        true
    }

    /// A right click in 「도형선택」 on a handle or a side of the selected shape opens the vertex menu;
    /// `false` leaves the click to the menu of the map.
    pub(crate) fn open_vertex_menu(&mut self, at: Pos2, place: [f64; 2]) -> bool {
        if self.active_tool != MapToolKind::Select {
            return false;
        }
        let vertex = self.vertex_at(place, MENU_VERTEX_PX);
        if vertex.is_none() && !self.side_at(place, MENU_SIDE_PX) {
            return false;
        }
        self.home_map.vertex_menu = Some(VertexMenu { at: [at.x, at.y], place, vertex });
        true
    }

    /// The vertex menu: one row, and a click anywhere else closes it.
    pub(crate) fn render_vertex_menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.home_map.vertex_menu else { return };
        let label = if menu.vertex.is_some() { "점삭제" } else { "점추가" };
        let area = egui::Area::new(egui::Id::new("ka_vertex_menu")).order(egui::Order::Foreground).fixed_pos(Pos2::new(menu.at[0], menu.at[1]));
        let shown = area.show(ctx, |ui| egui::Frame::popup(ui.style()).show(ui, |ui| ui.button(label).clicked()).inner);
        if shown.inner {
            self.home_map.vertex_menu = None;
            match menu.vertex {
                Some(index) => drop(self.delete_vertex(index)),
                None => drop(self.insert_vertex_at(menu.place)),
            }
        } else if shown.response.clicked_elsewhere() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.home_map.vertex_menu = None;
        }
    }

    /// The handles: a white box with a blue edge on every vertex, the clicked one red and larger; while a
    /// handle is held, the shape as it will be.
    pub(crate) fn paint_vertex_handles(&self, p: &egui::Painter, rect: Rect) {
        if self.active_tool != MapToolKind::Select {
            return;
        }
        let Some(target) = self.vertex_target() else { return };
        let (view, blue) = (&self.home_map.viewport, Color32::from_rgb(30, 103, 198));
        let mut points = self.vertex_points_on_map(&target);
        if let Some(HeldVertex { index, to: Some(to), .. }) = self.home_map.held_vertex {
            let twin = crate::app_state::vertex_edit_geom::ring_closing_vertex(&target.parts, index);
            for moved in std::iter::once(index).chain(twin) {
                if let Some(point) = points.get_mut(moved) {
                    *point = to;
                }
            }
            // The sides at the held vertex, as they will run.
            for side in [index.checked_sub(1), Some(index + 1), twin.and_then(|t| t.checked_sub(1)), twin.map(|t| t + 1)].into_iter().flatten() {
                if let (Some(a), Some(b)) = (points.get(index), points.get(side)) {
                    p.line_segment([w2s(rect, view, *a), w2s(rect, view, *b)], Stroke::new(2.0, blue.gamma_multiply(0.8)));
                }
            }
        }
        for (index, point) in points.iter().enumerate() {
            let (side, edge, fill) = if self.home_map.active_vertex == Some(index) {
                (16.0, Color32::from_rgb(220, 38, 38), Color32::from_rgb(254, 226, 226))
            } else {
                (14.0, blue, Color32::WHITE)
            };
            let handle = Rect::from_center_size(w2s(rect, view, *point), Vec2::splat(side));
            p.rect_filled(handle, 0.0, fill);
            p.rect_stroke(handle, 0.0, Stroke::new(2.0, edge), egui::StrokeKind::Middle);
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_map_vertex_tests.rs"]
mod tests;
