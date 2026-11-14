//! 그린 면을 편집 레이어(메모리)에 넣기 — C++ `KaCaptureMapTool::finish`(KaCaptureMapTool.cpp:393-456,
//! 작업 좌표계 → 레이어 좌표계)와 `MainWindow::onGeometryCaptured`(MainWindowEditing.cpp:782-920).
//! 좌표계 옮기기와 면 고치기는 C++ 과 같은 PROJ·GEOS(strata-engine). 파일에는 「저장」(2C)이 쓴다.
//! 그린 뒤 이름·번호 창은 app_state_feature_form.rs. 겹친 곳 지우기 제안(지시 4)·고친 면 알림 창은 아직 없다.

use std::collections::BTreeMap;

use strata_canvas2d::map_tools::MapToolKind;
use strata_engine::{PolygonRings, Transformer};
use strata_types::crs::CrsCode;

use crate::app_state::{MainTab, StrataAppState};
use crate::draw_sketch::FinishShort;

/// MainWindowEditing.cpp:996 — captureCanceled.
pub(crate) const TOO_FEW_TEXT: &str = "아직 넣지 않았습니다 — 면은 점 3개, 선은 2개 이상 필요합니다. 우클릭·Enter로 완료.";

use crate::feature_form::now_text;

/// QgsFields::lookupField — 이름이 같거나(대소문자 무시) 같은 칸.
pub(super) fn column<'a>(columns: &'a [(String, String)], name: &str) -> Option<&'a str> {
    columns.iter().map(|(c, _)| c.as_str()).find(|c| c.eq_ignore_ascii_case(name))
}

/// `FeatureRecord::stampNew` — 칸이 있으면 uid(UUID v4)·created_at·updated_at.
fn stamped_attributes(columns: &[(String, String)]) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let now = now_text();
    if let Some(c) = column(columns, "uid") {
        attrs.insert(c.to_owned(), uuid::Uuid::new_v4().to_string());
    }
    for name in ["created_at", "updated_at"] {
        if let Some(c) = column(columns, name) {
            attrs.insert(c.to_owned(), now.clone());
        }
    }
    attrs
}

fn polygon_wkt(rings: &PolygonRings) -> String {
    let body: Vec<String> = rings
        .iter()
        .map(|r| format!("({})", r.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", ")))
        .collect();
    format!("POLYGON ({})", body.join(", "))
}

fn linestring_wkt(pts: &[[f64; 2]]) -> String {
    let body = pts.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", ");
    format!("LINESTRING ({body})")
}

fn point_wkt(pt: [f64; 2]) -> String {
    format!("POINT ({} {})", pt[0], pt[1])
}

impl StrataAppState {
    // Polygon, line, and point capture share KaCaptureMapTool
    // (target/cpp-ref/src/app/MainWindowEditing.cpp:975-1015, target/cpp-ref/src/app/MainWindowFeatureCard.cpp:60,72).
    pub(crate) fn is_capture_tool(tool: MapToolKind) -> bool {
        matches!(tool, MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint)
    }

    // Active capture session on the map tab with an edit layer and no modal dialog open
    // (target/cpp-ref/src/app/KaDrawSketchTools.cpp:144-145, target/cpp-ref/src/app/MainWindowEditing.cpp:989-1015).
    pub(crate) fn capture_active(&self) -> bool {
        self.active_tab == MainTab::Map
            && Self::is_capture_tool(self.active_tool)
            && self.edit_layer_key.is_some()
            && !self.modal_window_open()
    }

    // True when polygon capture is active on the map tab (target/cpp-ref/src/app/MainWindowEditing.cpp:975-988).
    pub(crate) fn polygon_capture_active(&self) -> bool {
        self.capture_active() && self.active_tool == MapToolKind::DrawPolygon
    }

    // Left press: add vertex for polygon/line, or place and finish immediately in Mode::Point
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:179,271-297).
    pub(crate) fn sketch_press(&mut self, xy: [f64; 2]) {
        if !self.capture_active() {
            return;
        }
        if self.active_tool == MapToolKind::PlacePoint {
            if !xy[0].is_finite() || !xy[1].is_finite() {
                return;
            }
            self.draw_sketch.points.clear();
            self.draw_sketch.points.push(xy);
            self.sketch_finish();
            return;
        }
        self.draw_sketch.press_left(xy, self.home_map.viewport.meters_per_pixel);
    }

    // Double-click: ignored in Mode::Point; adds final vertex and finishes for polygon/line
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:305-314).
    pub(crate) fn sketch_double_click(&mut self, xy: [f64; 2]) {
        if self.capture_active() && self.active_tool != MapToolKind::PlacePoint {
            self.draw_sketch.double_click(xy);
            self.sketch_finish();
        }
    }

    /// Ctrl+Z·Backspace·Delete·「되돌리기」 — 마지막 점 하나(MainWindowUndo.cpp:331).
    pub(crate) fn sketch_undo_vertex(&mut self) -> bool {
        if !self.capture_active() || !self.draw_sketch.undo_last() {
            return false;
        }
        self.show_status_message("꼭짓점 하나를 되돌렸습니다.".to_owned());
        true
    }

    /// Esc·「취소」 — 그리던 도형만 지운다(도구·창은 그대로).
    pub(crate) fn sketch_cancel(&mut self) {
        if self.draw_sketch.cancel() {
            self.show_status_message("그리던 도형을 지웠습니다.".to_owned());
        }
    }

    // Finish point, line, or polygon capture and insert one feature into the target layer edit buffer
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:393-456, target/cpp-ref/src/app/MainWindowEditing.cpp:782-916).
    pub(crate) fn sketch_finish(&mut self) {
        if !self.capture_active() {
            return;
        }
        let mupp = self.home_map.viewport.meters_per_pixel;
        let tool = self.active_tool;
        enum RawGeom {
            Point([f64; 2]),
            Line(Vec<[f64; 2]>),
            Poly(crate::edit_geometry_repair::PolygonResult),
        }
        let raw = match tool {
            MapToolKind::PlacePoint => match self.draw_sketch.points.first().copied() {
                Some(pt) if pt[0].is_finite() && pt[1].is_finite() => RawGeom::Point(pt),
                Some(_) => {
                    self.draw_sketch.points.clear();
                    self.show_status_message(TOO_FEW_TEXT.to_owned());
                    return;
                }
                None => return,
            },
            MapToolKind::DrawLine => match self.draw_sketch.finish_line(mupp) {
                Ok(line) => RawGeom::Line(line),
                Err(FinishShort::Empty) => return,
                Err(FinishShort::TooFew) => {
                    self.show_status_message(TOO_FEW_TEXT.to_owned());
                    return;
                }
            },
            _ => match self.draw_sketch.close(mupp) {
                Ok(closed) => RawGeom::Poly(closed),
                Err(FinishShort::Empty) => return,
                Err(FinishShort::TooFew) => {
                    self.show_status_message(TOO_FEW_TEXT.to_owned());
                    return;
                }
            },
        };
        let Some(key) = self.edit_layer_key.clone() else { return };
        let Some(idx) = self.survey_tables.iter().position(|t| t.table == key) else {
            self.draw_sketch.points.clear();
            self.show_status_message("편집 레이어 없음 — 그리기 도구를 다시 선택하세요".to_owned());
            return;
        };
        let work = CrsCode::from(self.work_crs).epsg_number();
        let dst_crs = u32::try_from(self.survey_tables[idx].srs_id).ok().filter(|&d| d > 0 && d != work);
        let built = match raw {
            RawGeom::Point(pt) => match dst_crs {
                Some(dst) => Transformer::new(work, dst).and_then(|t| t.transform(pt)).map(|p| (point_wkt(p), None)),
                None => Ok((point_wkt(pt), None)),
            },
            RawGeom::Line(line) => match dst_crs {
                Some(dst) => Transformer::new(work, dst).and_then(|t| t.transform_all(&line)).map(|l| (linestring_wkt(&l), None)),
                None => Ok((linestring_wkt(&line), None)),
            },
            RawGeom::Poly(closed) => {
                let Some(rings) = closed.rings.clone() else { return };
                let notice = crate::edit_geometry_repair::notice(&closed);
                match dst_crs {
                    Some(dst) => Transformer::new(work, dst)
                        .and_then(|t| rings.iter().map(|r| t.transform_all(r)).collect::<Result<Vec<_>, _>>())
                        .map(|moved| (polygon_wkt(&moved), notice)),
                    None => Ok((polygon_wkt(&rings), notice)),
                }
            }
        };
        let Ok((wkt, notice)) = built else {
            if tool == MapToolKind::PlacePoint {
                self.draw_sketch.points.clear();
            }
            self.show_status_message(TOO_FEW_TEXT.to_owned());
            return;
        };
        let attrs = stamped_attributes(&self.survey_tables[idx].columns);
        // MainWindowEditing.cpp:826 — 카드에 적던 값은 그 도형의 따로 한 단계로 먼저 쓴다.
        self.commit_card_typing();
        let fid = self.survey_tables[idx].layer.add_unsaved_feature(wkt, attrs);
        // m_lastDrawnLayer / m_lastDrawnFid are polygon-only (target/cpp-ref/src/app/MainWindowEditing.cpp:845-849).
        if tool == MapToolKind::DrawPolygon {
            self.home_map.last_drawn_shape = Some((key.clone(), fid));
        }
        self.draw_sketch.points.clear();
        self.redo_tables.clear();
        self.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table(key.clone()));
        self.is_dirty = true;
        self.after_layer_edit(&key);
        // 이름·번호 창(연속 그리기 꺼짐)과 그 뒤 상태 글 — app_state_feature_form.rs.
        self.after_capture(&key, fid, notice);
    }
}

#[path = "app_state_draw_commit_b.rs"]
mod app_state_draw_commit_b;

#[cfg(test)]
#[path = "app_state_draw_commit_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "app_state_draw_commit_line_tests.rs"]
mod line_tests;
#[cfg(test)]
#[path = "app_state_draw_commit_point_tests.rs"]
mod point_tests;
