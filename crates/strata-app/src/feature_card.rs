//! 오른쪽 카드에 보이는 한 도형의 기록 — C++ `KaFeatureCard`(KaFeatureCard.cpp:45-68, 258-299 refresh, KaFeatureCardEdit.cpp:50-87
//! setValue, KaFeatureCardEditors.cpp:77-82·140-275 줄·기준점). 그리기는 3b(ri_attr_card).

use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_storage::wkb_codec::WkbGeometry;

pub(crate) use crate::checklist_state::attr;
use crate::checklist_state_fields::normalized_number;
use crate::feature_form::{canonical_value, kind_key};
use crate::feature_form_state::FeatureForm;
use crate::survey_area_setup::is_survey_area_key;

/// titleFor — 레이어 키로.
pub(crate) fn title_for(key: &str) -> &'static str {
    match key {
        "feature_poly" | "feature_line" => "선택한 유구",
        "artifact_point" => "선택한 유물",
        "control_points" => "선택한 기준점",
        "section_line" => "선택한 단면선",
        k if is_survey_area_key(k) => "선택한 조사구역",
        _ => "선택한 도형",
    }
}

/// layerKindFor — 레이어 키 낱말, 없으면 레이어 이름.
pub(crate) fn layer_kind_for(key: &str, layer_name: &str) -> String {
    match key {
        "feature_poly" => "유구 면",
        "feature_line" => "유구 선",
        "artifact_point" => "유물 위치",
        "control_points" => "기준점",
        "section_line" => "단면선",
        k if is_survey_area_key(k) => "조사구역",
        _ => return layer_name.to_owned(),
    }
    .to_owned()
}

/// 상태 칩 — Neutral 「저장된 값입니다」, Warn 「변경됨 · 아직 저장 안 됨」, Danger 「바꾸지 못했습니다: %1」.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CardState {
    Saved,
    Changed,
    Failed(String),
}

impl CardState {
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Saved => "저장된 값입니다".to_owned(),
            Self::Changed => "변경됨 · 아직 저장 안 됨".to_owned(),
            Self::Failed(e) => format!("바꾸지 못했습니다: {e}"),
        }
    }
}

/// 기준점 줄(새 모양) — 측지 글, X (동)·Y (북)·Z.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlRows {
    pub(crate) meta: String,
    pub(crate) x: String,
    pub(crate) y: String,
    pub(crate) z: String,
}

/// 카드가 보이는 값.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CardView {
    pub(crate) title: &'static str,
    pub(crate) header: String,
    pub(crate) layer_kind: String,
    pub(crate) sub_line: String,
    pub(crate) number_field: Option<String>,
    pub(crate) number_label: &'static str,
    pub(crate) number: String,
    pub(crate) next_number: String,
    pub(crate) note_number: Option<String>,
    /// 조사구역의 조사명·유적명·이름(칸, 낱말, 값).
    pub(crate) name_rows: Vec<(&'static str, &'static str, String)>,
    pub(crate) has_kind: bool,
    pub(crate) kind: String,
    pub(crate) has_period: bool,
    pub(crate) period: String,
    /// 조사 상태·조사자·조사일.
    pub(crate) work_rows: Vec<(&'static str, &'static str, String)>,
    pub(crate) has_note: bool,
    pub(crate) note: String,
    pub(crate) area: Option<String>,
    /// (「둘레」|「길이」, 값).
    pub(crate) perimeter: Option<(&'static str, String)>,
    pub(crate) control: Option<ControlRows>,
    pub(crate) state: CardState,
}

fn has(t: &SurveyFileTable, field: &str) -> bool {
    t.columns.iter().any(|(c, _)| c.eq_ignore_ascii_case(field))
}

/// FeatureNumbering::fieldsOf — 번호 칸.
fn number_field(t: &SurveyFileTable) -> Option<&'static str> {
    ["feature_no", "artifact_no", "section_id", "point_id"].into_iter().find(|f| has(t, f))
}

/// KaFeatureCard.cpp:45-47 metres — 소수 둘째, 모르면 「-」.
fn metres(value: Option<f64>, unit: &str) -> String {
    value.filter(|v| !v.is_nan()).map_or_else(|| "-".to_owned(), |v| format!("{v:.2} {unit}"))
}

/// QLocale(English).toString(v, 'f', 3) — 천 단위 쉼표.
fn grouped(v: f64) -> String {
    let s = format!("{:.3}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{}{out}.{frac}", if v < 0.0 { "-" } else { "" })
}

fn control_rows(f: &strata_storage::edit_buffer::FeatureRecord) -> ControlRows {
    let text = |k: &str| attr(f, k).map(|v| v.trim().to_owned()).unwrap_or_default();
    let mut meta: Vec<String> = ["datum", "ellipsoid", "projection", "origin"].iter().map(|k| text(k)).filter(|v| !v.is_empty()).collect();
    if text("origin").is_empty() {
        meta.push("원점 비어 있음".to_owned());
    }
    let point = match wkt_to_parts(&f.wkt_geom).ok().as_deref() {
        Some([WkbGeometry::Point(p)]) => Some(*p),
        _ => None,
    };
    let number = |k: &str| attr(f, k).and_then(|v| v.trim().parse::<f64>().ok());
    let none = "—".to_owned();
    ControlRows {
        meta: meta.join(" · "),
        x: number("x").or(point.map(|p| p[0])).map_or_else(|| none.clone(), grouped),
        y: number("y").or(point.map(|p| p[1])).map_or_else(|| none.clone(), grouped),
        z: number("z").map_or(none, |z| format!("EL. {z:.2}")),
    }
}

/// 그 표의 도형 `fid` 의 카드(도형이 없으면 None). `failure` 는 마지막으로 못 바꾼 이유.
pub(crate) fn card_view(t: &SurveyFileTable, fid: i64, layer_name: &str, failure: Option<&str>) -> Option<CardView> {
    let f = t.layer.features.iter().find(|f| f.fid == fid)?;
    let value = |k: &str| attr(f, k).unwrap_or_default();
    let key = t.table.as_str();
    let number_field = number_field(t);
    let (kind, period) = (value("kind"), value("period"));
    let number = number_field.map(value).unwrap_or_default();
    let area_layer = is_survey_area_key(key);
    let name_rows: Vec<_> = if area_layer {
        [("survey_name", "조사명"), ("site_name", "유적명"), ("name", "이름")].into_iter().filter(|(c, _)| has(t, c)).map(|(c, l)| (c, l, value(c))).collect()
    } else {
        Vec::new()
    };
    let mut header = match (number.is_empty(), kind.is_empty()) {
        (true, _) => kind.clone(),
        (false, true) => number.clone(),
        (false, false) => format!("{number} {kind}"),
    };
    if area_layer && header.is_empty() {
        header = name_rows.iter().map(|r| r.2.clone()).find(|v| !v.is_empty()).unwrap_or_default();
    }
    let layer_kind = layer_kind_for(key, layer_name);
    let survey_name = if area_layer && value("survey_name") != header { value("survey_name") } else { String::new() };
    let sub_line = [layer_kind.clone(), kind.clone(), period.clone(), survey_name].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    // FeatureRecord::measure — 표 좌표계에서 평면(면적·둘레, 선은 길이), 경위도 표나 도형 없음은 「-」.
    let geo = t.srs_id == 4326 || f.wkt_geom.trim().is_empty();
    let ty = t.geometry_type.to_ascii_uppercase();
    let measured = |m: fn(&str) -> Result<f64, strata_engine::EngineError>| if geo { None } else { m(&f.wkt_geom).ok() };
    let (area, perimeter) = if ty.contains("POLYGON") {
        (Some(metres(measured(strata_engine::wkt_area), "m²")), Some(("둘레", metres(measured(strata_engine::wkt_length), "m"))))
    } else if ty.contains("LINE") {
        (None, Some(("길이", metres(measured(strata_engine::wkt_length), "m"))))
    } else {
        (None, None)
    };
    // FeatureNumbering::sameNumber·suggestNext — 유구번호는 종류마다, 그 밖은 레이어 전체.
    let by_kind = number_field == Some("feature_no") && has(t, "kind");
    let group = |k: &str| if by_kind { kind_key(k) } else { String::new() };
    let wanted = normalized_number(&number);
    let others = if number_field.is_none() || wanted.is_empty() {
        0
    } else {
        t.layer.features.iter().filter(|o| o.fid != fid && group(&attr(o, "kind").unwrap_or_default()) == group(&kind)).filter(|o| number_field.and_then(|n| attr(o, n)).is_some_and(|n| normalized_number(&n) == wanted)).count()
    };
    let next_number = FeatureForm::new(t, fid).map(|mut form| {
        form.name = kind.clone();
        form.update_number_hint();
        form.number
    });
    let changed = t.baseline.iter().find(|b| b.fid == fid) != Some(f);
    Some(CardView {
        title: title_for(key),
        header: if header.is_empty() { "이름 없음".to_owned() } else { header },
        layer_kind,
        sub_line,
        number_field: number_field.map(str::to_owned),
        number_label: match number_field {
            Some("feature_no") => "유구번호",
            Some("artifact_no") => "유물번호",
            Some("section_id") => "단면번호",
            _ => "점 이름",
        },
        number,
        next_number: if number_field.is_some() { next_number.unwrap_or_default() } else { String::new() },
        note_number: (others > 0).then(|| format!("같은 종류에 같은 번호가 {others}개 더 있습니다. 제출 전 검수에서도 알려 드립니다.")),
        name_rows,
        has_kind: has(t, "kind"),
        kind,
        has_period: has(t, "period"),
        period,
        work_rows: [("status", "조사 상태"), ("surveyor", "조사자"), ("surv_date", "조사일")].into_iter().filter(|(c, _)| has(t, c)).map(|(c, l)| (c, l, value(c))).collect(),
        has_note: has(t, "note"),
        note: value("note"),
        area,
        perimeter,
        control: (key == "control_points").then(|| control_rows(f)),
        state: match failure {
            Some(e) => CardState::Failed(e.to_owned()),
            None if changed => CardState::Changed,
            None => CardState::Saved,
        },
    })
}

/// 한 도형에 쓸 (칸, 값) — None 은 NULL.
pub(crate) type AttributeChanges = Vec<(String, Option<String>)>;

/// setValue — 적은 값을 바꿀 값으로. 같은 값(빈 칸과 NULL 도 같음)이면 Ok(None) = 되돌리기 단계 없음.
/// 종류·시대는 조사 맞춤법, 빈 칸은 NULL, 바뀌면 updated_at 도 함께(한 단계).
pub(crate) fn card_value(t: &SurveyFileTable, fid: i64, field: &str, text: &str) -> Result<Option<AttributeChanges>, String> {
    let col = t.columns.iter().find(|(c, _)| c.eq_ignore_ascii_case(field)).ok_or_else(|| format!("이 레이어에는 「{field}」 칸이 없습니다."))?;
    let f = t.layer.features.iter().find(|f| f.fid == fid).ok_or("도형을 찾지 못했습니다.")?;
    let tidy = text.trim();
    let ty = col.1.to_ascii_uppercase();
    let numeric = ty.contains("INT") || ty.contains("REAL") || ty.contains("DOUB") || ty.contains("FLOA");
    let value = if tidy.is_empty() {
        None
    } else if numeric {
        Some(crate::feature_record::qt_to_double(tidy).ok_or_else(|| format!("숫자만 넣을 수 있습니다(소수점은 .): {tidy}"))?.to_string())
    } else if field == "kind" || field == "period" {
        Some(canonical_value(t, field, tidy))
    } else {
        Some(tidy.to_owned())
    };
    let current = attr(f, &col.0).map(|v| v.trim().to_owned()).unwrap_or_default();
    if current == value.clone().unwrap_or_default() {
        return Ok(None);
    }
    let mut values = vec![(col.0.clone(), value)];
    if let Some((c, _)) = t.columns.iter().find(|(c, _)| c.eq_ignore_ascii_case("updated_at")) {
        values.push((c.clone(), Some(crate::feature_form::now_text())));
    }
    Ok(Some(values))
}

#[cfg(test)]
#[path = "feature_card_tests.rs"]
pub(crate) mod tests;
