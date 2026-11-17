//! The selection box of 「도형선택」 — C++ `KaFeatureSelectTool` (KaFeatureSelectTool.cpp:209-225, 263-270,
//! 417-484): a drag that starts on bare map (not on a handle) and goes further than 4 points draws a blue box;
//! when it is let go every shape of the visible survey layers that meets the box is selected, added to the
//! selection with Shift. The middle button still moves the map.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use super::gui_tab_map_geom::s2w;
use crate::app_state::StrataAppState;

/// `kClickSlopPx`.
const CLICK_SLOP_PX: f32 = 4.0;

/// A box being drawn: where the press was, where the pointer is, and whether it went further than a click.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SelectBox {
    from: Pos2,
    to: Pos2,
    moved: bool,
}

impl StrataAppState {
    /// The pointer of 「도형선택」 on bare map. `true` while a box is drawn and in the frame it is let go: the map
    /// does not pan then.
    pub(crate) fn handle_select_box(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) -> bool {
        if self.active_tool != MapToolKind::Select || self.modal_window_open() || self.home_map.held_vertex.is_some() {
            self.home_map.select_box = None;
            return false;
        }
        let (pressed, down, at, shift) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.interact_pos(), i.modifiers.shift));
        if let (true, true, Some(at)) = (pressed, response.is_pointer_button_down_on(), at) {
            self.home_map.select_box = Some(SelectBox { from: at, to: at, moved: false });
        }
        let Some(mut drawn) = self.home_map.select_box.take() else { return false };
        if let Some(at) = at {
            drawn.to = at;
            drawn.moved |= (at.x - drawn.from.x).abs() + (at.y - drawn.from.y).abs() > CLICK_SLOP_PX;
        }
        if down {
            self.home_map.select_box = Some(drawn);
            return drawn.moved;
        }
        if drawn.moved {
            let (a, b) = (s2w(rect, &self.home_map.viewport, drawn.from), s2w(rect, &self.home_map.viewport, drawn.to));
            self.select_in_box(a, b, shift);
        }
        drawn.moved
    }

    /// The box while it is drawn (`m_rubberBand`: blue, 1 point, light fill).
    pub(crate) fn paint_select_box(&self, p: &egui::Painter) {
        if let Some(SelectBox { from, to, moved: true }) = self.home_map.select_box {
            let area = Rect::from_two_pos(from, to);
            p.rect_filled(area, 0.0, Color32::from_rgba_unmultiplied(30, 103, 198, 40));
            p.rect_stroke(area, 0.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(30, 103, 198, 180)), egui::StrokeKind::Middle);
        }
    }

    /// Every shape of the visible survey layers that meets the box `a`..`b` (work CRS) is selected — added to
    /// the selection with `add` — and the status line counts them (`selectInRect`, :417-484).
    pub(crate) fn select_in_box(&mut self, a: [f64; 2], b: [f64; 2], add: bool) {
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let mut hits: Vec<(String, i64)> = Vec::new();
        for e in self.left_layers.entries.iter().filter(|e| self.home_map.real_survey.is_layer_visible(&e.label_ko)) {
            let Some(t) = self.survey_tables.iter().find(|t| t.table == e.layer_key) else { continue };
            // The box in the table's CRS: the box around its four corners there.
            let corners = [[a[0], a[1]], [b[0], a[1]], [b[0], b[1]], [a[0], b[1]]];
            // target/cpp-ref/src/app/KaVertexEditTool.cpp:110-133: never interpret unknown source coordinates as work CRS.
            let Ok(table_srs) = u32::try_from(t.srs_id) else { continue };
            let carried: Option<Vec<[f64; 2]>> = if table_srs == work { Some(corners.to_vec()) } else { strata_engine::Transformer::new(work, table_srs).ok().and_then(|carry| corners.iter().map(|c| carry.transform(*c).ok()).collect()) };
            let Some(corners) = carried else { continue };
            let min = [corners.iter().map(|c| c[0]).fold(f64::INFINITY, f64::min), corners.iter().map(|c| c[1]).fold(f64::INFINITY, f64::min)];
            let max = [corners.iter().map(|c| c[0]).fold(f64::NEG_INFINITY, f64::max), corners.iter().map(|c| c[1]).fold(f64::NEG_INFINITY, f64::max)];
            for f in &t.layer.features {
                let Ok(parts) = wkt_to_parts(&f.wkt_geom) else { continue };
                let (mut polygons, mut lines, mut points) = (Vec::new(), Vec::new(), Vec::new());
                for part in parts {
                    match part {
                        WkbGeometry::Polygon(rings) => polygons.push(rings),
                        WkbGeometry::LineString(line) => lines.push(line),
                        WkbGeometry::Point(point) => points.push(point),
                    }
                }
                if strata_geo::polygon_split::meets_box(&polygons, &lines, &points, min, max) {
                    hits.push((t.table.clone(), f.fid));
                }
            }
        }
        let mut all = if add { self.all_selected() } else { Vec::new() };
        for hit in hits {
            if !all.contains(&hit) {
                all.push(hit);
            }
        }
        self.home_map.selected_shape = all.pop();
        (self.home_map.also_selected, self.home_map.active_vertex, self.home_map.pick_cycle) = (all, None, None);
        self.sync_card_with_selection();
        let count = self.all_selected().len();
        self.show_status_message(format!("도형 {count}개 선택됨"));
    }
}

#[cfg(test)]
#[path = "gui_tab_map_select_box_tests.rs"]
mod tests;
