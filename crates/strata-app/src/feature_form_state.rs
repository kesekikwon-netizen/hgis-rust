//! 열려 있는 「이름·번호」 창 — C++ `KaFeatureFormDialog` 생성자의 줄 고르기(:36-118)·`updateNumberHint`(:120-142)·
//! `nameText`·`numberText`·`periodText`·`applyTo`(:184-216). 값은 그 도형의 편집 버퍼에만 쓴다(파일은 「저장」).

use std::collections::HashMap;

use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::checklist_state::attr;
use crate::checklist_state_fields::normalized_number;
use crate::feature_form::{canonical_value, choices, default_template, form_fields, kind_key, next_from, FormFields};

/// 창의 값.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FeatureForm {
    pub(crate) table: String,
    pub(crate) fid: i64,
    pub(crate) fields: FormFields,
    /// 이름 칸이 kind 면 「종류」 고르기(목록 + 직접), 아니면 「이름」 글칸.
    pub(crate) name_is_kind: bool,
    /// 시대 칸이 있으면 「시대」 줄.
    pub(crate) has_period: bool,
    pub(crate) kind_choices: Vec<String>,
    pub(crate) period_choices: Vec<String>,
    pub(crate) name: String,
    pub(crate) period: String,
    pub(crate) number: String,
    pub(crate) number_typed: bool,
    group_by_kind: bool,
    numbers_by_kind: HashMap<String, Vec<String>>,
    template: Option<&'static str>,
    /// 창을 닫은 뒤 띄울 「면을 고쳐 넣었습니다」 알림(그린 면이 고쳐졌을 때).
    pub(crate) repair_notice: Option<String>,
    /// 창이 열린 그 프레임의 Enter(도형을 마친 누름)는 「저장」이 아니다 — 다음 프레임부터 키를 받는다.
    pub(crate) keys_armed: bool,
}

impl FeatureForm {
    /// 그 표의 도형 `fid` 에 대한 창. 이름·번호 칸이 없으면 None(canOffer).
    pub(crate) fn new(t: &SurveyFileTable, fid: i64) -> Option<Self> {
        let fields = form_fields(t);
        if fields.name_field.is_none() && fields.number_field.is_none() {
            return None;
        }
        let name_is_kind = fields.name_field.as_deref() == Some("kind");
        let has_period = t.columns.iter().any(|(c, _)| c.eq_ignore_ascii_case("period"));
        // FeatureNumbering::fieldsOf — 번호 칸, 유구번호(feature_no)면 종류마다.
        let number_col = ["feature_no", "artifact_no", "section_id", "point_id"]
            .into_iter()
            .find(|c| t.columns.iter().any(|(n, _)| n.eq_ignore_ascii_case(c)));
        let kind_col = t.columns.iter().any(|(n, _)| n.eq_ignore_ascii_case("kind"));
        let group_by_kind = number_col == Some("feature_no") && kind_col;
        let mut numbers_by_kind: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(col) = number_col {
            for f in &t.layer.features {
                let Some(n) = attr(f, col).filter(|n| !n.is_empty()) else { continue };
                let kind = if group_by_kind { attr(f, "kind").unwrap_or_default() } else { String::new() };
                numbers_by_kind.entry(kind_key(&kind)).or_default().push(n);
            }
        }
        let mut form = Self {
            table: t.table.clone(),
            fid,
            name_is_kind,
            has_period,
            kind_choices: if name_is_kind { choices(t, "kind") } else { Vec::new() },
            period_choices: if has_period { choices(t, "period") } else { Vec::new() },
            name: String::new(),
            period: String::new(),
            number: String::new(),
            number_typed: false,
            group_by_kind,
            numbers_by_kind,
            template: number_col.and_then(default_template),
            repair_notice: None,
            keys_armed: false,
            fields,
        };
        form.update_number_hint();
        Some(form)
    }

    fn used_numbers(&self) -> &[String] {
        let kind = if self.group_by_kind && self.name_is_kind { self.name.as_str() } else { "" };
        self.numbers_by_kind.get(&kind_key(kind)).map_or(&[], Vec::as_slice)
    }

    /// `updateNumberHint` — 직접 적기 전에는 이 종류의 다음 번호를 넣는다.
    pub(crate) fn update_number_hint(&mut self) {
        if self.fields.number_field.is_some() && !self.number_typed {
            self.number = next_from(self.used_numbers(), self.template);
        }
    }

    /// 「다음 번호를 미리 넣었습니다」를 보이는가.
    pub(crate) fn number_filled_visible(&self) -> bool {
        !self.number_typed && !self.number.is_empty()
    }

    /// 같은 번호가 있으면 주황 글(#B45309).
    pub(crate) fn number_note(&self) -> Option<String> {
        let wanted = normalized_number(&self.number);
        let repeated = !wanted.is_empty() && self.used_numbers().iter().any(|n| normalized_number(n) == wanted);
        repeated.then(|| {
            let scope = if self.group_by_kind { "같은 종류에" } else { "이 레이어에" };
            format!("{scope} 이미 있는 번호입니다({}). 그래도 저장할 수 있습니다.", self.number.trim())
        })
    }

    /// `nameText` — 종류는 조사에서 쓴 맞춤법으로(valueEditorText → canonicalValue).
    pub(crate) fn name_text(&self, t: &SurveyFileTable) -> String {
        let text = self.name.trim();
        if self.name_is_kind && !text.is_empty() { canonical_value(t, "kind", text) } else { text.to_owned() }
    }

    pub(crate) fn period_text(&self, t: &SurveyFileTable) -> String {
        let text = self.period.trim();
        if self.has_period && !text.is_empty() { canonical_value(t, "period", text) } else { String::new() }
    }

    pub(crate) fn number_text(&self) -> String {
        self.number.trim().to_owned()
    }
}

#[cfg(test)]
#[path = "feature_form_state_tests.rs"]
mod tests;
