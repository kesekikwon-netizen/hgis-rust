//! 그린 뒤 「이름·번호」 — C++ `onGeometryCaptured`(MainWindowEditing.cpp:851-871): 이름·번호 칸이 있는 레이어이고 연속
//! 그리기가 꺼져 있으면 「도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요.」와 함께 창을 띄우고, 「저장」이면
//! `applyTo`(이름·번호, 시대, updated_at)를 그 도형의 편집 버퍼에 쓴다 — 그린 것과 같은 되돌리기 한 단계(KaUndoGroup).
//! 창을 닫은 뒤에 상태 글(종류·시대 빈 유구 수 포함)을 센다.

use crate::app_state::StrataAppState;
use crate::feature_form::can_offer;
use crate::feature_form_state::FeatureForm;

impl StrataAppState {
    /// 도형을 넣은 바로 뒤.
    pub(crate) fn after_capture(&mut self, key: &str, fid: i64, notice: Option<String>) {
        let offer = self.survey_tables.iter().find(|t| t.table == key).is_some_and(can_offer);
        if offer && !self.ribbon.continuous_draw {
            if let Some(mut form) = self.survey_tables.iter().find(|t| t.table == key).and_then(|t| FeatureForm::new(t, fid)) {
                form.repair_notice = notice;
                self.home_map.feature_form = Some(form);
                self.show_status_message("도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요.");
                return;
            }
        }
        let text = self.captured_status(key, offer && self.ribbon.continuous_draw, notice.as_deref());
        self.show_status_message(text);
    }

    /// 창의 「저장」(Enter).
    pub(crate) fn save_feature_form(&mut self) {
        let Some(form) = self.home_map.feature_form.take() else { return };
        match self.apply_feature_form(&form) {
            Ok(()) => {
                let text = self.captured_status(&form.table, false, form.repair_notice.as_deref());
                self.show_status_message(text);
            }
            // C++ 은 알림 창(「속성」) — 창이 생기기 전까지 상태줄에.
            Err(e) => self.show_status_message(format!(
                "이름·번호를 넣지 못했습니다. 도형은 그대로 있습니다. {e} 도형을 우클릭해 「이 도형 기록 입력」으로 다시 적으세요."
            )),
        }
    }

    /// 창의 「건너뛰기」(Esc) — 그린 도형은 빈 값으로 남는다.
    pub(crate) fn skip_feature_form(&mut self) {
        let Some(form) = self.home_map.feature_form.take() else { return };
        let text = self.captured_status(&form.table, false, form.repair_notice.as_deref());
        self.show_status_message(text);
    }

    /// `applyTo` — 이름·번호(applyFeatureFormValues: 칸이 있으면 빈 글도 쓴다), 시대(적었을 때만), updated_at(touch).
    fn apply_feature_form(&mut self, form: &FeatureForm) -> Result<(), String> {
        let idx = self.survey_tables.iter().position(|t| t.table == form.table).ok_or("레이어가 없습니다.")?;
        let t = &self.survey_tables[idx];
        let (name, number, period) = (form.name_text(t), form.number_text(), form.period_text(t));
        let has_updated_at = t.columns.iter().find(|(c, _)| c.eq_ignore_ascii_case("updated_at")).map(|(c, _)| c.clone());
        let fields = form.fields.clone();
        let feature = self.survey_tables[idx].layer.features.iter_mut().find(|f| f.fid == form.fid).ok_or("도형을 찾지 못했습니다.")?;
        if let Some(field) = fields.name_field {
            feature.attributes.insert(field, name);
        }
        if let Some(field) = fields.number_field {
            feature.attributes.insert(field, number);
        }
        if form.has_period && !period.is_empty() {
            feature.attributes.insert("period".to_owned(), period);
        }
        if let Some(col) = has_updated_at {
            feature.attributes.insert(col, crate::feature_form::now_text());
        }
        self.is_dirty = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "app_state_feature_form_tests.rs"]
mod tests;
