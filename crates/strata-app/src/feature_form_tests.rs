//! 「이름·번호」 값 고르기 — C++ tests/test_feature_records.cpp·test_presets.cpp 의 기댓값.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::DomainLayerKey;

use super::*;

/// 메모리 레이어처럼 칸과 행을 주는 표.
pub(crate) fn table(columns: &[&str], rows: &[&[(&str, &str)]]) -> SurveyFileTable {
    let mut layer = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
    for (i, row) in rows.iter().enumerate() {
        let attributes: BTreeMap<String, String> = row.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        layer.features.push(FeatureRecord { fid: i as i64 + 1, wkt_geom: String::new(), attributes });
    }
    SurveyFileTable {
        table: "feature_poly".to_owned(),
        srs_id: 5187,
        geometry_type: "POLYGON".to_owned(),
        columns: columns.iter().map(|c| ((*c).to_owned(), "TEXT".to_owned())).collect(),
        baseline: layer.features.clone(),
        layer,
    }
}

/// ported from tests/test_presets.cpp:jsonMatchesBuiltInTable — 기본 종류·시대가 data/styles/feature_presets.json 과 같다.
#[test]
fn builtin_presets_match_the_json() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/styles/feature_presets.json"))
        .expect("presets json");
    let json: serde_json::Value = serde_json::from_str(&text).expect("json");
    let kinds: Vec<&str> = json["kinds"].as_array().unwrap().iter().map(|k| k["label"].as_str().unwrap()).collect();
    let periods: Vec<&str> = json["periods"].as_array().unwrap().iter().map(|p| p["label"].as_str().unwrap()).collect();
    assert_eq!(kinds, KA_BUILTIN_KINDS.to_vec());
    assert_eq!(periods, KA_BUILTIN_PERIODS.iter().map(|(n, _)| *n).collect::<Vec<_>>());
    assert_eq!(periods.last(), Some(&"미정"));
}

/// ported from tests/test_presets.cpp:spellingKeys_matchPresets
#[test]
fn spelling_keys() {
    assert_eq!(kind_key(" 주거 지 "), "주거지");
    assert_eq!(period_key("청동기 시대"), "청동기");
}

/// ported from tests/test_feature_records.cpp:normalizedAndParse(parse 부분)
#[test]
fn parse_number_parts() {
    let p = parse_number("제03호");
    assert_eq!((p.prefix.as_str(), p.value, p.width, p.suffix.as_str()), ("제", 3, 2, "호"));
    assert_eq!(parse_number("A-A'").value, -1);
    assert_eq!(crate::checklist_state_fields::normalized_number("s-01"), "S-1");
}

/// ported from tests/test_feature_records.cpp:nextFrom_keepsTheWrittenForm
#[test]
fn next_from_keeps_the_written_form() {
    let fallback = default_template("feature_no");
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
    assert_eq!(next_from(&s(&["1호", "2호"]), fallback), "3호");
    assert_eq!(next_from(&s(&["02호"]), fallback), "03호");
    assert_eq!(next_from(&[], fallback), "1호");
    assert_eq!(next_from(&s(&["B-3"]), None), "B-4");
    assert_eq!(next_from(&s(&["A-A'"]), None), "");
    assert_eq!(default_template("point_id"), Some("P{}"));
}

/// ported from tests/test_feature_records.cpp:choices_presetOrderThenSurveySpelling
#[test]
fn choices_preset_order_then_survey_spelling() {
    let t = table(&["kind", "period"], &[&[("period", "청동기시대")], &[("period", "미상기"), ("kind", "토기가마")]]);
    let periods = choices(&t, "period");
    assert_eq!(periods[0], "구석기");
    assert_eq!(periods[2], "청동기시대");
    assert!(!periods.contains(&"청동기".to_owned()));
    assert_eq!(periods.last().map(String::as_str), Some("미상기"));
    let kinds = choices(&t, "kind");
    assert_eq!(kinds[0], "주거지");
    assert!(kinds.contains(&"토기가마".to_owned()));
}

/// ported from tests/test_feature_records.cpp:artifactNumbers_runOverTheLayer(choices 부분) — 유물 레이어는 유구 종류를 안 내놓는다.
#[test]
fn artifact_layer_offers_its_own_kinds_only() {
    let t = table(&["kind", "artifact_no"], &[&[("kind", "토기"), ("artifact_no", "1")], &[("kind", "석기"), ("artifact_no", "2")]]);
    assert_eq!(choices(&t, "kind"), vec!["석기".to_owned(), "토기".to_owned()]);
}

/// featureFormFields — 유구 면은 종류·번호, 새 조사 조사구역은 조사명, 번호 칸이 이름 칸보다 먼저다.
#[test]
fn form_fields_per_layer() {
    let poly = table(&["kind", "period", "feature_no", "note"], &[]);
    assert_eq!(form_fields(&poly), FormFields { name_field: Some("kind".to_owned()), number_field: Some("feature_no".to_owned()) });
    let area = table(&["survey_name", "survey_id", "note"], &[]);
    assert_eq!(form_fields(&area), FormFields { name_field: Some("survey_name".to_owned()), number_field: None });
    assert!(!can_offer(&table(&["note"], &[])));
}
