//! 면 그리기 도구의 지도 입력·그리던 선 — C++ `KaCaptureMapTool` 누름(KaCaptureMapTool.cpp:260-298, 우클릭은
//! 누를 때 마침)·더블클릭(:305-314)·고무줄(:192-226, QgsRubberBand 굵기 3, 파랑 30,103,198, 채움 알파 80,
//! 둘째 선 흰색 알파 200). 그리는 동안 왼쪽 끌기는 지도를 옮기지 않는다(QGIS 그리기 도구) — 가운데 끌기로 옮긴다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use crate::app_state::StrataAppState;

use super::gui_tab_map_geom::{s2w, w2s};
use super::gui_tab_map_survey::fill_mesh;

/// 앞·마지막 왼쪽 누름 자리(화면).
type Presses = (Option<Pos2>, Option<Pos2>);

impl StrataAppState {
    /// 누름·더블클릭을 그리던 면에 넘긴다. 면 그리기 도구가 켜져 있지 않으면 false(다른 도구가 처리).
    pub(crate) fn handle_polygon_capture_input(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) -> bool {
        if !self.polygon_capture_active() {
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
        // egui 는 빠른 두 번 누름이면 자리가 달라도 더블클릭으로 본다. Qt(Windows)는 두 누름이 4 px 안일 때만이다
        // (SM_CXDOUBLECLK) — 빠르게 다른 자리를 찍어도 면이 마쳐지지 않게 한다.
        if response.double_clicked() {
            let (prev, last) = ui.data(|d| d.get_temp::<Presses>(presses_id).unwrap_or_default());
            let same_spot = matches!((prev, last), (Some(a), Some(b)) if a.distance(b) <= 4.0);
            if let (true, Some(pos)) = (same_spot, response.interact_pointer_pos()) {
                self.sketch_double_click(s2w(rect, &self.home_map.viewport, pos));
            }
        }
        true
    }

    /// 그리던 면(찍은 점 + 마우스 자리).
    pub(crate) fn paint_polygon_sketch(&self, p: &egui::Painter, rect: Rect, cursor: Option<Pos2>) {
        if !self.polygon_capture_active() || self.draw_sketch.points.is_empty() {
            return;
        }
        let vp = &self.home_map.viewport;
        let mut pts: Vec<Pos2> = self.draw_sketch.points.iter().map(|xy| w2s(rect, vp, *xy)).collect();
        if let Some(c) = cursor.filter(|c| rect.contains(*c)) {
            pts.push(c);
        }
        let closed = pts.len() >= 3;
        if closed {
            p.add(fill_mesh(std::slice::from_ref(&pts), Color32::from_rgba_unmultiplied(30, 103, 198, 80)));
        }
        for (width, color) in [(5.0, Color32::from_rgba_unmultiplied(255, 255, 255, 200)), (3.0, Color32::from_rgb(30, 103, 198))] {
            let stroke = Stroke::new(width, color);
            p.add(if closed { egui::Shape::closed_line(pts.clone(), stroke) } else { egui::Shape::line(pts.clone(), stroke) });
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_map_capture_tests.rs"]
mod tests;
