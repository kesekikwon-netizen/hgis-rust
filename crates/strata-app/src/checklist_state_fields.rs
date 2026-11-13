//! 검수 상태의 속성 검사 — C++ `ChecklistStateChecks::scanFields`(ChecklistStateFields.cpp)와 유구번호 겹침
//! `FeatureNumbering::duplicates`·`normalized`(FeatureNumbering.cpp:38-58, 103-119, 145-163).

use serde_json::{json, Map, Value};
use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::checklist_state::{attr, feature_label, layers_for, DomainLayer, Offenders};

/// 빈 칸이면 true(칸이 없거나 NULL 이어도 빈 칸).
fn blank(f: &strata_storage::edit_buffer::FeatureRecord, field: &str) -> bool {
    attr(f, field).is_none_or(|v| v.is_empty())
}

fn has_column(t: &SurveyFileTable, field: &str) -> bool {
    t.columns.iter().any(|(c, _)| c.eq_ignore_ascii_case(field))
}

/// data-model 필수 글 칸(kRequired). 칸이 없는 레이어(가져온 SHP)는 보지 않는다.
const REQUIRED: [(&str, &str); 4] =
    [("survey_area", "survey_name"), ("control_points", "point_id"), ("section_line", "section_id"), ("feature_line", "kind")];

/// `scanFields` — 유구 종류·시대(모든 유구), 기준점 메타(모든 점), 필수 칸, 기준점 이름 겹침.
pub(crate) fn scan_fields(layers: &[DomainLayer<'_>], st: &mut Map<String, Value>, off: &mut Offenders) {
    let mut kind_period = true;
    for layer in layers_for(layers, "feature_poly") {
        for f in &layer.table.layer.features {
            if blank(f, "kind") || blank(f, "period") {
                kind_period = false;
                off.add("has_kind_period", layer, f.fid, &feature_label(f));
            }
        }
    }
    st.insert("has_kind_period".to_owned(), json!(kind_period));

    let mut points = 0usize;
    let (mut datum, mut ellipsoid, mut projection, mut origin, mut accuracy, mut unique_ids) = (true, true, true, true, true, true);
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for layer in layers_for(layers, "control_points") {
        for f in &layer.table.layer.features {
            points += 1;
            let label = feature_label(f);
            for (field, key, flag) in [
                ("datum", "has_datum", &mut datum),
                ("ellipsoid", "has_ellipsoid", &mut ellipsoid),
                ("projection", "has_projection", &mut projection),
                ("origin", "has_origin", &mut origin),
            ] {
                if blank(f, field) {
                    *flag = false;
                    off.add(key, layer, f.fid, &label);
                }
            }
            if blank(f, "accuracy_m") && blank(f, "accuracy") {
                accuracy = false;
                off.add("has_accuracy", layer, f.fid, &label);
            }
            if let Some(id) = attr(f, "point_id").filter(|v| !v.is_empty()) {
                let n = seen.entry(id.clone()).or_insert(0);
                *n += 1;
                if *n > 1 {
                    unique_ids = false;
                    off.add("control_points_point_id_unique", layer, f.fid, &id);
                }
            }
        }
    }
    // 점이 없으면 메타 규칙은 GCP_MIN_TWO 와 함께 실패한다. 투영원점·정확도는 조용하다.
    st.insert("has_datum".to_owned(), json!(points > 0 && datum));
    st.insert("has_ellipsoid".to_owned(), json!(points > 0 && ellipsoid));
    st.insert("has_projection".to_owned(), json!(points > 0 && projection));
    st.insert("has_origin".to_owned(), json!(origin));
    st.insert("has_accuracy".to_owned(), json!(accuracy));
    st.insert("control_points_point_id_unique".to_owned(), json!(unique_ids));

    let mut required = true;
    let mut missing: Vec<&str> = Vec::new();
    for (key, field) in REQUIRED {
        for layer in layers_for(layers, key) {
            if !has_column(layer.table, field) {
                continue;
            }
            let mut layer_missing = false;
            for f in layer.table.layer.features.iter().filter(|f| blank(f, field)) {
                required = false;
                layer_missing = true;
                off.add("required_fields_filled", layer, f.fid, &format!("{} · {field} 없음", feature_label(f)));
            }
            if layer_missing && !missing.contains(&field) {
                missing.push(field);
            }
        }
    }
    st.insert("required_fields_filled".to_owned(), json!(required));
    if !missing.is_empty() {
        off.note("required_fields_filled", format!("빈 항목: {}", missing.join(", ")));
    }
}

/// 숫자 하나(ASCII·전각)의 값 — QChar::digitValue 가운데 조사 자료에 나오는 것.
fn digit_value(c: char) -> Option<u32> {
    c.to_digit(10).or_else(|| ('０'..='９').contains(&c).then(|| c as u32 - '０' as u32))
}

/// `FeatureNumbering::normalized` — 공백을 빼고, 「제1」의 제·끝 「호」를 떼고, 숫자 앞 0 을 지우고, 글자는 대문자로.
/// 그래서 1호·01호·1 호·제1호는 같은 번호다.
pub(crate) fn normalized_number(number: &str) -> String {
    let mut t: Vec<char> = number.chars().filter(|c| !c.is_whitespace()).collect();
    if t.len() > 1 && t[0] == '제' && digit_value(t[1]).is_some() {
        t.remove(0);
    }
    if t.last() == Some(&'호') {
        t.pop();
    }
    let mut out = String::new();
    let mut i = 0;
    while i < t.len() {
        if digit_value(t[i]).is_none() {
            out.extend(t[i].to_uppercase());
            i += 1;
            continue;
        }
        let mut run = String::new();
        while let Some(d) = t.get(i).and_then(|c| digit_value(*c)) {
            run.push(char::from_digit(d, 10).unwrap_or('0'));
            i += 1;
        }
        let trimmed = run.trim_start_matches('0');
        out.push_str(if trimmed.is_empty() { "0" } else { trimmed });
    }
    out
}

/// `FeatureNumbering::duplicates` — 같은 종류(공백 뺀 이름) 안에서 정규화한 번호가 겹치는 도형 (fid, 「종류 번호」).
/// 번호 칸은 feature_no·artifact_no·section_id·point_id 가운데 처음 있는 것, 종류로 나누는 것은 feature_no 일 때만.
pub(crate) fn feature_number_duplicates(t: &SurveyFileTable) -> Vec<(i64, String)> {
    let Some(number_col) = ["feature_no", "artifact_no", "section_id", "point_id"].into_iter().find(|c| has_column(t, c)) else {
        return Vec::new();
    };
    let by_kind = number_col == "feature_no" && has_column(t, "kind");
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, Vec<(i64, String)>> = std::collections::HashMap::new();
    for f in &t.layer.features {
        let n = attr(f, number_col).unwrap_or_default();
        let number = normalized_number(&n);
        if number.is_empty() {
            continue;
        }
        let k = if by_kind { attr(f, "kind").unwrap_or_default() } else { String::new() };
        let key = format!("{}\u{1F}{number}", k.replace(' ', ""));
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        let label = if k.is_empty() { n } else { format!("{k} {n}") };
        groups.entry(key).or_default().push((f.fid, label));
    }
    order.into_iter().filter_map(|k| groups.remove(&k)).filter(|g| g.len() > 1).flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::normalized_number;

    /// 규칙 문구: 「1호·01호·1 호는 같은 번호로 봅니다.」
    #[test]
    fn numbers_normalize_like_cpp() {
        for n in ["1호", "01호", "1 호", "제1호", "１호"] {
            assert_eq!(normalized_number(n), "1", "{n}");
        }
        assert_eq!(normalized_number("a-02"), "A-2");
        assert_eq!(normalized_number("0"), "0");
        assert_eq!(normalized_number(""), "");
    }
}
