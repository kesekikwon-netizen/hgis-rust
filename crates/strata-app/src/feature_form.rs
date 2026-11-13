//! 「이름·번호」 창의 값 — C++ `KaFeatureFormDialog`(KaFeatureFormDialog.cpp)와 그 밑의 `LayerOps::featureFormFields`
//! (LayerOps.cpp:313-332), `FeatureRecord::choices`·`canonicalValue`·`distinctValues`(FeatureRecord.cpp:10-98),
//! `FeatureNumbering::parse`·`nextFrom`·`defaultTemplate`(FeatureNumbering.cpp:58-101), `FeaturePresets::kindKey`·
//! `periodKey`. 창 모양은 `dlg_feature_form.rs`.

use std::collections::HashMap;

use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::checklist_state::attr;
use crate::right_inspector::{KA_BUILTIN_KINDS, KA_BUILTIN_PERIODS};

/// `FeaturePresets::kindKey` — 앞뒤 공백을 빼고 띄어쓰기를 지운다.
pub(crate) fn kind_key(text: &str) -> String {
    text.trim().replace(' ', "")
}

/// `periodKey` — kindKey 에서 끝 「시대」를 뗀다.
pub(crate) fn period_key(text: &str) -> String {
    let k = kind_key(text);
    k.strip_suffix("시대").map(str::to_owned).unwrap_or(k)
}

fn key_of(field: &str, text: &str) -> String {
    if field == "period" { period_key(text) } else { kind_key(text) }
}

fn has_column(t: &SurveyFileTable, field: &str) -> bool {
    t.columns.iter().any(|(c, _)| c == field)
}

/// `distinctValues(layer, field, 200)` — 빈 값 빼고, 많이 쓴 것 먼저(같으면 글자 차례).
pub(crate) fn distinct_values(t: &SurveyFileTable, field: &str) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for f in &t.layer.features {
        if let Some(v) = attr(f, field).filter(|v| !v.is_empty()) {
            *counts.entry(v).or_insert(0) += 1;
        }
    }
    let mut values: Vec<String> = counts.keys().cloned().collect();
    values.sort_by(|a, b| counts[b].cmp(&counts[a]).then_with(|| a.cmp(b)));
    values.truncate(200);
    values
}

fn preset_labels(t: &SurveyFileTable, field: &str) -> Vec<&'static str> {
    match field {
        "kind" if !t.columns.iter().any(|(c, _)| c.eq_ignore_ascii_case("artifact_no")) => KA_BUILTIN_KINDS.to_vec(),
        "period" => KA_BUILTIN_PERIODS.iter().map(|(n, _)| *n).collect(),
        _ => Vec::new(),
    }
}

/// `FeatureRecord::choices` — 미리 정한 차례(조사에서 쓴 맞춤법이 있으면 그것), 그다음 조사에만 있는 값. 유물 레이어는
/// 유구 종류를 내놓지 않는다.
pub(crate) fn choices(t: &SurveyFileTable, field: &str) -> Vec<String> {
    let existing = distinct_values(t, field);
    let (mut out, mut keys): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    let mut add = |value: &str| {
        let key = key_of(field, value);
        if !keys.contains(&key) {
            keys.push(key);
            out.push(value.to_owned());
        }
    };
    for label in preset_labels(t, field) {
        let key = key_of(field, label);
        let used = existing.iter().find(|v| key_of(field, v) == key);
        add(used.map_or(label, String::as_str));
    }
    for v in &existing {
        add(v);
    }
    out
}

/// `canonicalValue` — 조사에서 쓴 맞춤법이나 미리 정한 이름으로 맞춘다(공백 정리).
pub(crate) fn canonical_value(t: &SurveyFileTable, field: &str, text: &str) -> String {
    let tidy = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let key = key_of(field, &tidy);
    if key.is_empty() {
        return tidy;
    }
    if let Some(v) = distinct_values(t, field).into_iter().find(|v| key_of(field, v) == key) {
        return v;
    }
    preset_labels(t, field).into_iter().find(|l| kind_key(l) == kind_key(&tidy)).map_or(tidy, str::to_owned)
}

/// `FeatureNumbering::Parts` — 끝 숫자 덩이의 앞·값·자릿수·뒤(숫자가 없으면 값 -1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumberParts {
    pub(crate) prefix: String,
    pub(crate) value: i64,
    pub(crate) width: usize,
    pub(crate) suffix: String,
}

pub(crate) fn parse_number(number: &str) -> NumberParts {
    let t: Vec<char> = number.trim().chars().collect();
    let none = || NumberParts { prefix: t.iter().collect(), value: -1, width: 0, suffix: String::new() };
    let Some(end) = t.iter().rposition(|c| c.is_ascii_digit()).map(|i| i + 1) else { return none() };
    let start = t[..end].iter().rposition(|c| !c.is_ascii_digit()).map_or(0, |i| i + 1);
    let digits: String = t[start..end].iter().collect();
    let Ok(value) = digits.parse::<i64>() else { return none() };
    NumberParts { prefix: t[..start].iter().collect(), value, width: digits.len(), suffix: t[end..].iter().collect() }
}

/// `defaultTemplate` — feature_no 「%1호」, artifact_no 「%1」, point_id 「P%1」, 그 밖(단면 번호)은 짐작하지 않는다.
pub(crate) fn default_template(number_field: &str) -> Option<&'static str> {
    match number_field {
        "feature_no" => Some("{}호"),
        "artifact_no" => Some("{}"),
        "point_id" => Some("P{}"),
        _ => None,
    }
}

/// `nextFrom` — 가장 큰 번호 다음을 그 쓴 꼴대로(자릿수 0 채움), 없으면 틀의 1.
pub(crate) fn next_from(existing: &[String], template: Option<&str>) -> String {
    let best = existing.iter().map(|s| parse_number(s)).filter(|p| p.value >= 0).fold(None::<NumberParts>, |best, p| match best {
        Some(b) if b.value >= p.value => Some(b),
        _ => Some(p),
    });
    match best {
        Some(b) => format!("{}{:0width$}{}", b.prefix, b.value + 1, b.suffix, width = b.width),
        None => template.map_or_else(String::new, |t| t.replace("{}", "1")),
    }
}

/// `featureFormFields` — 번호 칸(feature_no·artifact_no·section_id·point_id 가운데 처음), 이름 칸(survey_name·site_name·
/// name·kind 가운데 처음, 번호 칸과 같으면 없음).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FormFields {
    pub(crate) name_field: Option<String>,
    pub(crate) number_field: Option<String>,
}

pub(crate) fn form_fields(t: &SurveyFileTable) -> FormFields {
    let first = |names: &[&str]| names.iter().find(|n| has_column(t, n)).map(|n| (*n).to_owned());
    let number_field = first(&["feature_no", "artifact_no", "section_id", "point_id"]);
    let name_field = first(&["survey_name", "site_name", "name", "kind"]).filter(|n| Some(n) != number_field.as_ref());
    FormFields { name_field, number_field }
}

/// `canOffer` — 이름이나 번호 칸이 있으면 창을 띄운다.
pub(crate) fn can_offer(t: &SurveyFileTable) -> bool {
    let f = form_fields(t);
    f.name_field.is_some() || f.number_field.is_some()
}

#[cfg(test)]
#[path = "feature_form_tests.rs"]
pub(crate) mod tests;

/// C++ `FeatureRecord::nowText` — 지역 시각 ISO(초까지, 시간대 붙임).
pub(crate) fn now_text() -> String {
    let now = chrono::Local::now();
    if now.offset().local_minus_utc() == 0 {
        now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    } else {
        now.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
    }
}
