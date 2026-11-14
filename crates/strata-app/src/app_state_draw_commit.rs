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
fn column<'a>(columns: &'a [(String, String)], name: &str) -> Option<&'a str> {
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

impl StrataAppState {
    // Polygon and line capture share KaCaptureMapTool (target/cpp-ref/src/app/MainWindowEditing.cpp:975-1015).
    pub(crate) fn is_capture_tool(tool: MapToolKind) -> bool {
        matches!(tool, MapToolKind::DrawPolygon | MapToolKind::DrawLine)
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

    /// 왼쪽 누름 — 점 더하기.
    pub(crate) fn sketch_press(&mut self, xy: [f64; 2]) {
        if self.capture_active() {
            self.draw_sketch.press_left(xy, self.home_map.viewport.meters_per_pixel);
        }
    }

    /// 더블클릭 — 그 자리를 더하고 마친다.
    pub(crate) fn sketch_double_click(&mut self, xy: [f64; 2]) {
        if self.capture_active() {
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

    // Finish polygon or line sketch and add one feature to the target layer edit buffer
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:393-456, target/cpp-ref/src/app/MainWindowEditing.cpp:782-916).
    pub(crate) fn sketch_finish(&mut self) {
        if !self.capture_active() {
            return;
        }
        let mupp = self.home_map.viewport.meters_per_pixel;
        let is_line = self.active_tool == MapToolKind::DrawLine;
        let (poly_closed, line_pts) = if is_line {
            match self.draw_sketch.finish_line(mupp) {
                Ok(line) => (None, Some(line)),
                Err(FinishShort::Empty) => return,
                Err(FinishShort::TooFew) => {
                    self.show_status_message(TOO_FEW_TEXT.to_owned());
                    return;
                }
            }
        } else {
            match self.draw_sketch.close(mupp) {
                Ok(closed) => (Some(closed), None),
                Err(FinishShort::Empty) => return,
                Err(FinishShort::TooFew) => {
                    self.show_status_message(TOO_FEW_TEXT.to_owned());
                    return;
                }
            }
        };
        let Some(key) = self.edit_layer_key.clone() else { return };
        let Some(idx) = self.survey_tables.iter().position(|t| t.table == key) else {
            self.draw_sketch.points.clear();
            self.show_status_message("편집 레이어 없음 — 그리기 도구를 다시 선택하세요".to_owned());
            return;
        };
        let work = CrsCode::from(self.work_crs).epsg_number();
        let dst_crs = u32::try_from(self.survey_tables[idx].srs_id).ok().filter(|&d| d > 0 && d != work);
        let (wkt, notice) = if let Some(line) = line_pts {
            let line = match dst_crs {
                Some(dst) => match Transformer::new(work, dst).and_then(|t| t.transform_all(&line)) {
                    Ok(moved) => moved,
                    Err(_) => {
                        self.show_status_message(TOO_FEW_TEXT.to_owned());
                        return;
                    }
                },
                None => line,
            };
            (linestring_wkt(&line), None)
        } else {
            let Some(closed) = poly_closed else { return };
            let Some(rings) = closed.rings.clone() else { return };
            let rings = match dst_crs {
                Some(dst) => {
                    let moved = Transformer::new(work, dst).and_then(|t| {
                        rings.iter().map(|r| t.transform_all(r)).collect::<Result<Vec<_>, _>>()
                    });
                    match moved {
                        Ok(moved) => moved,
                        Err(_) => {
                            self.show_status_message(TOO_FEW_TEXT.to_owned());
                            return;
                        }
                    }
                }
                None => rings,
            };
            (polygon_wkt(&rings), crate::edit_geometry_repair::notice(&closed))
        };
        let attrs = stamped_attributes(&self.survey_tables[idx].columns);
        // MainWindowEditing.cpp:826 — 카드에 적던 값은 그 도형의 따로 한 단계로 먼저 쓴다.
        self.commit_card_typing();
        let fid = self.survey_tables[idx].layer.add_unsaved_feature(wkt, attrs);
        // m_lastDrawnLayer / m_lastDrawnFid are polygon-only (target/cpp-ref/src/app/MainWindowEditing.cpp:845-849).
        if !is_line {
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

    /// onGeometryCaptured 끝의 상태 글(MainWindowEditing.cpp:905-916) — 이름·번호를 받은 뒤에 센다.
    pub(crate) fn captured_status(&self, key: &str, continuous_suffix: bool, notice: Option<&str>) -> String {
        let name = self.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone()).unwrap_or_default();
        let n = self.survey_tables.iter().find(|t| t.table == key).map_or(0, |t| t.layer.features.len());
        let mut text = format!("도형을 넣었습니다 ({name}, {n}개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다");
        if continuous_suffix {
            text.push_str(" · 이름·번호는 나중에 속성에서");
        }
        if key == "feature_poly" {
            text.push_str(&self.missing_attribute_counter_text());
        }
        // C++ 은 고친 면을 알림 창(「면을 고쳐 넣었습니다」)으로 따로 알린다 — 알림 창이 생기기 전까지 이어 쓴다.
        if let Some(notice) = notice {
            text.push_str(&format!(" · 면을 고쳐 넣었습니다: {notice}"));
        }
        text
    }

    /// Ctrl+Z — 그리던 점이 있으면 마지막 점, 없으면 가장 최근에 바꾼 레이어의 편집 하나
    /// (MainWindowUndo.cpp:325-363 undoMapAction), 레이어 삭제면 그 레이어들을 되돌린다(:369-389, 다시 실행은 없다).
    pub(crate) fn undo_map_action(&mut self) {
        if self.sketch_undo_vertex() {
            return;
        }
        while let Some(step) = self.undo_tables.pop() {
            let key = match step {
                crate::app_state_layer_remove::UndoStep::Table(key) => key,
                crate::app_state_layer_remove::UndoStep::LayersRemoved(removed) => {
                    self.restore_removed_layers(removed);
                    self.show_status_message("이전 상태로 되돌렸습니다.".to_owned());
                    return;
                }
            };
            let undone = self.survey_tables.iter_mut().find(|t| t.table == key).is_some_and(|t| t.layer.undo());
            if undone {
                self.redo_tables.push(key.clone());
                self.is_dirty = true;
                self.after_layer_edit(&key);
                self.show_status_message("이전 상태로 되돌렸습니다.".to_owned());
                return;
            }
        }
        self.show_status_message("되돌릴 것이 없습니다.".to_owned());
    }

    /// Ctrl+Y — 가장 최근에 되돌린 레이어 편집을 다시(MainWindowUndo.cpp:469-484 redoMapAction).
    pub(crate) fn redo_map_action(&mut self) {
        while let Some(key) = self.redo_tables.pop() {
            let redone = self.survey_tables.iter_mut().find(|t| t.table == key).is_some_and(|t| t.layer.redo());
            if redone {
                self.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table(key.clone()));
                self.is_dirty = true;
                self.after_layer_edit(&key);
                self.show_status_message("다시 실행했습니다.".to_owned());
                return;
            }
        }
        self.show_status_message("다시 실행할 것이 없습니다.".to_owned());
    }

    /// 범례의 도형 수를 편집 레이어에 맞춘다(QGIS featureCount 는 저장 안 한 도형도 센다).
    pub(crate) fn after_layer_edit(&mut self, key: &str) {
        let Some(n) = self.survey_tables.iter().find(|t| t.table == key).map(|t| t.layer.features.len()) else { return };
        if let Some(e) = self.left_layers.entries.iter_mut().find(|e| e.layer_key == key) {
            e.feature_count = n;
        }
        // 되돌리기로 도형이 없어졌으면 카드를 비운다(KaFeatureCard.cpp:198-203 featureDeleted).
        self.drop_card_if_gone();
    }

    /// C++ `missingAttributeCounterText`(KaSubmitFlow.cpp:269-290) — 종류·시대가 빈 유구 면 수.
    pub(crate) fn missing_attribute_counter_text(&self) -> String {
        let mut missing = 0usize;
        for t in self.survey_tables.iter().filter(|t| t.table == "feature_poly") {
            let (kind, period) = (column(&t.columns, "kind"), column(&t.columns, "period"));
            for f in &t.layer.features {
                let filled = |c: Option<&str>| c.and_then(|c| f.attributes.get(c)).is_some_and(|v| !v.trim().is_empty());
                if !filled(kind) || !filled(period) {
                    missing += 1;
                }
            }
        }
        if missing > 0 {
            format!(" · 종류·시대 빈 유구 {missing}개(제출 전 「검수·제출」에서 확인)")
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
#[path = "app_state_draw_commit_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "app_state_draw_commit_line_tests.rs"]
mod line_tests;
