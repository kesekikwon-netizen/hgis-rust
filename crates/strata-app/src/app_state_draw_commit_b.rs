//! Capture status message, map undo/redo dispatch, and missing-attribute counter
//! (`target/cpp-ref/src/app/MainWindowEditing.cpp:905-916`,
//! `target/cpp-ref/src/app/MainWindowUndo.cpp:325-389,469-484`,
//! `target/cpp-ref/src/app/KaSubmitFlow.cpp:269-290`).

use crate::app_state::StrataAppState;

use super::column;

impl StrataAppState {
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
