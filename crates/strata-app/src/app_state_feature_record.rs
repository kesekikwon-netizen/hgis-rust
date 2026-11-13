//! 「도형 속성」 열기·저장 — C++ `MainWindow::editFeatureAttributes`(MainWindowEditing.cpp:616-776). 값은 그 도형의 편집 버퍼에
//! 「속성 편집」 한 단계로 쓰고(Ctrl+Z 로 되돌림), 파일에는 「저장」(Ctrl+S)이 쓴다. C++ 의 알림 창(KaUserError·
//! QMessageBox)은 창이 생기기 전까지 상태줄에 쓴다.

use crate::app_state::StrataAppState;
use crate::feature_record::FeatureRecordForm;

impl StrataAppState {
    /// 그 표의 도형 `fid` 로 창을 연다.
    pub(crate) fn open_feature_record(&mut self, table: &str, fid: i64) {
        let name = self.left_layers.entries.iter().find(|e| e.layer_key == table).map(|e| e.label_ko.clone()).unwrap_or_default();
        let Some(t) = self.survey_tables.iter().find(|t| t.table == table) else { return };
        match FeatureRecordForm::new(t, fid, &name) {
            Ok(form) => self.home_map.feature_record = Some(form),
            Err(text) => self.show_status_message(text.to_owned()),
        }
    }

    /// 「취소」 — 아무것도 바꾸지 않는다.
    pub(crate) fn cancel_feature_record(&mut self) {
        self.home_map.feature_record = None;
    }

    /// 「저장」 — 모든 칸을 검사한 뒤(나쁜 숫자면 아무것도 바꾸지 않음) 값과 updated_at(touch)을 한 단계로 쓴다.
    pub(crate) fn save_feature_record(&mut self) {
        let Some(form) = self.home_map.feature_record.take() else { return };
        let Some(idx) = self.survey_tables.iter().position(|t| t.table == form.table) else { return };
        let t = &self.survey_tables[idx];
        let mut values = match form.values(t) {
            Ok(values) => values,
            Err(w) => {
                self.show_status_message(format!("{} {} {}", w.text, w.detail, w.action));
                return;
            }
        };
        if let Some((col, _)) = t.columns.iter().find(|(c, _)| c.eq_ignore_ascii_case("updated_at")) {
            values.push((col.clone(), Some(crate::feature_form::now_text())));
        }
        if !self.survey_tables[idx].layer.change_attributes(form.fid, &values) {
            // KaEditErrors::show(「속성을 바꾸지 못했습니다.」) — 그 사이 도형이 없어졌을 때.
            self.show_status_message("속성을 바꾸지 못했습니다. 같은 파일을 연 다른 프로그램을 닫은 뒤 다시 저장하세요.".to_owned());
            return;
        }
        self.redo_tables.clear();
        self.undo_tables.push(form.table.clone());
        self.is_dirty = true;
        self.after_layer_edit(&form.table);
        self.show_status_message(format!(
            "속성을 바꿨습니다: {} (#{}). Ctrl+Z로 되돌리고, 「저장」(Ctrl+S)으로 파일에 씁니다.",
            form.layer_name, form.fid
        ));
    }
}

#[cfg(test)]
#[path = "app_state_feature_record_tests.rs"]
mod tests;
