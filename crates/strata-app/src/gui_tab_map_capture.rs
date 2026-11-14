//! 면 그리기 도구의 지도 입력·그리던 선 — C++ `KaCaptureMapTool` 누름(KaCaptureMapTool.cpp:260-298, 우클릭은
//! 누를 때 마침)·더블클릭(:305-314)·고무줄(:192-226, QgsRubberBand 굵기 3, 파랑 30,103,198, 채움 알파 80,
//! 둘째 선 흰색 알파 200). 그리는 동안 왼쪽 끌기는 지도를 옮기지 않는다(QGIS 그리기 도구) — 가운데 끌기로 옮긴다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;

use super::gui_tab_map_geom::{s2w, w2s};
use super::gui_tab_map_survey::fill_mesh;

/// 앞·마지막 왼쪽 누름 자리(화면).
type Presses = (Option<Pos2>, Option<Pos2>);

impl StrataAppState {
    // Route canvas presses and double-clicks to polygon, line, and point capture
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:260-314).
    pub(crate) fn handle_polygon_capture_input(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) -> bool {
        if !self.capture_active() {
            return false;
        }
        let presses_id = response.id.with("ka_capture_presses");
        if response.hovered() {
            let (left, right, pos) = ui.input(|i| {
                (
                    i.pointer.button_pressed(egui::PointerButton::Primary),
                    i.pointer.button_pressed(egui::PointerButton::Secondary),
                    i.pointer.interact_pos(),
                )
            });
            if let Some(pos) = pos {
                if right {
                    self.sketch_finish();
                } else if left {
                    ui.data_mut(|d| {
                        let (_, last) = d.get_temp::<Presses>(presses_id).unwrap_or_default();
                        d.insert_temp(presses_id, (last, Some(pos)));
                    });
                    self.sketch_press(s2w(rect, &self.home_map.viewport, pos));
                }
            }
        }
        // Ignore double-click in Mode::Point (target/cpp-ref/src/app/KaCaptureMapTool.cpp:306); Qt counts double-clicks within 4 px.
        if self.active_tool != MapToolKind::PlacePoint && response.double_clicked() {
            let (prev, last) = ui.data(|d| d.get_temp::<Presses>(presses_id).unwrap_or_default());
            let same_spot = matches!((prev, last), (Some(a), Some(b)) if a.distance(b) <= 4.0);
            if let (true, Some(pos)) = (same_spot, response.interact_pointer_pos()) {
                self.sketch_double_click(s2w(rect, &self.home_map.viewport, pos));
            }
        }
        true
    }

    // Paint capture rubberband: closed with fill for >= 3 polygon points, open polyline without fill for lines
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:192-217).
    pub(crate) fn paint_polygon_sketch(&self, p: &egui::Painter, rect: Rect, cursor: Option<Pos2>) {
        if !self.capture_active() || self.draw_sketch.points.is_empty() {
            return;
        }
        let vp = &self.home_map.viewport;
        let mut pts: Vec<Pos2> = self.draw_sketch.points.iter().map(|xy| w2s(rect, vp, *xy)).collect();
        if let Some(c) = cursor.filter(|c| rect.contains(*c)) {
            pts.push(c);
        }
        let closed = self.polygon_capture_active() && pts.len() >= 3;
        if closed {
            p.add(fill_mesh(std::slice::from_ref(&pts), Color32::from_rgba_unmultiplied(30, 103, 198, 80)));
        }
        for (width, color) in [(5.0, Color32::from_rgba_unmultiplied(255, 255, 255, 200)), (3.0, Color32::from_rgb(30, 103, 198))] {
            let stroke = Stroke::new(width, color);
            p.add(if closed { egui::Shape::closed_line(pts.clone(), stroke) } else { egui::Shape::line(pts.clone(), stroke) });
        }
    }
}

impl StrataAppState {
    /// 고른 조사 도형 — QGIS 선택 색(255,235,59, 알파 45, MainWindowChrome.cpp setSelectionColor)으로 면은 채움만, 선·점은 그 색.
    pub(crate) fn paint_selected_shape(&self, p: &egui::Painter, rect: Rect) {
        use strata_storage::wkb_codec::WkbGeometry;
        let Some((table, fid)) = &self.home_map.selected_shape else { return };
        let Some(t) = self.survey_tables.iter().find(|t| &t.table == table) else { return };
        let Some(f) = t.layer.features.iter().find(|f| f.fid == *fid) else { return };
        let Ok(parts) = strata_storage::gpkg_wkt::wkt_to_parts(&f.wkt_geom) else { return };
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let parts = match u32::try_from(t.srs_id).ok().filter(|s| *s > 0 && *s != work) {
            Some(src) => match strata_engine::Transformer::new(src, work).ok().and_then(|tr| crate::checklist_state_geometry::moved(&parts, &tr)) {
                Some(moved) => moved,
                None => return,
            },
            None => parts,
        };
        let sel = Color32::from_rgba_unmultiplied(255, 235, 59, 45);
        let vp = &self.home_map.viewport;
        let to = |pts: &[[f64; 2]]| pts.iter().map(|xy| w2s(rect, vp, *xy)).collect::<Vec<Pos2>>();
        for part in &parts {
            match part {
                WkbGeometry::Polygon(rings) => {
                    let screen: Vec<Vec<Pos2>> = rings.iter().map(|r| to(r)).collect();
                    p.add(fill_mesh(&screen, sel));
                }
                WkbGeometry::LineString(l) => {
                    p.add(egui::Shape::line(to(l), Stroke::new(3.0, sel)));
                }
                WkbGeometry::Point(xy) => {
                    p.circle_filled(w2s(rect, vp, *xy), 5.0, sel);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_map_capture_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "gui_tab_map_capture_line_tests.rs"]
mod line_tests;
#[cfg(test)]
#[path = "gui_tab_map_capture_point_tests.rs"]
mod point_tests;
