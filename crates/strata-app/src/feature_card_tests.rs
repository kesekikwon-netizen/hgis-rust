//! 오른쪽 카드 — C++ tests/test_feature_card.cpp card_* 기댓값.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::DomainLayerKey;

use super::*;

/// test_feature_card.cpp surveyFeatures — EPSG:5187 유구 면, 10×5 m 네모 셋(저장된 값), 시대 청동기.
pub(crate) fn survey_features() -> SurveyFileTable {
    let mut layer = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
    for (fid, kind, number, x) in [(1, "주거지", "1호", 0.0), (2, "주거지", "2호", 20.0), (3, "수혈", "1호", 40.0)] {
        let (x0, x1) = (200_000.0 + x, 200_010.0 + x);
        let attributes: BTreeMap<String, String> =
            [("kind", kind), ("period", "청동기"), ("feature_no", number)].iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        let wkt_geom = format!("POLYGON (({x0} 450000, {x1} 450000, {x1} 450005, {x0} 450005, {x0} 450000))");
        layer.features.push(FeatureRecord { fid, wkt_geom, attributes });
    }
    SurveyFileTable {
        table: "feature_poly".to_owned(),
        srs_id: 5187,
        geometry_type: "POLYGON".to_owned(),
        columns: ["kind", "period", "feature_no", "note", "updated_at"].iter().map(|c| ((*c).to_owned(), "TEXT".to_owned())).collect(),
        baseline: layer.features.clone(),
        layer,
    }
}

fn view(t: &SurveyFileTable, fid: i64) -> CardView {
    card_view(t, fid, "유구", None).expect("card")
}

/// 바꿀 값을 그 도형에 넣는다(앱의 change_attributes 와 같은 일).
fn apply(t: &mut SurveyFileTable, fid: i64, field: &str, text: &str) -> bool {
    match card_value(t, fid, field, text).expect("valid") {
        Some(values) => t.layer.change_attributes(fid, &values),
        None => false,
    }
}

/// ported from tests/test_feature_card.cpp:card_showsOneSelectedFeature(보이는 글).
#[test]
fn card_shows_one_feature() {
    let t = survey_features();
    let v = view(&t, 1);
    assert_eq!(v.title, "선택한 유구");
    assert_eq!((v.number.as_str(), v.kind.as_str(), v.period.as_str()), ("1호", "주거지", "청동기"));
    assert_eq!(v.state.text(), "저장된 값입니다");
    assert!(card_view(&t, 99, "유구", None).is_none());
}

/// ported from tests/test_feature_card.cpp:card_editsStayInEditBuffer — 바뀐 값만 한 단계, 같은 값·같은 맞춤법은 단계 없음.
#[test]
fn card_edits_stay_in_the_edit_buffer() {
    let mut t = survey_features();
    assert!(apply(&mut t, 2, "note", "북벽 교란"));
    assert_eq!(attr(t.layer.features.iter().find(|f| f.fid == 2).unwrap(), "note").as_deref(), Some("북벽 교란"));
    assert!(view(&t, 2).state.text().contains("아직 저장 안 됨"));
    assert!(attr(t.layer.features.iter().find(|f| f.fid == 2).unwrap(), "updated_at").is_some_and(|s| !s.is_empty()));
    assert_eq!(card_value(&t, 2, "note", "북벽 교란").unwrap(), None, "unchanged: no new step");
    assert_eq!(card_value(&t, 2, "kind", "주거 지").unwrap(), None, "same key: no change");
    assert!(apply(&mut t, 2, "kind", "수 혈"));
    assert_eq!(attr(t.layer.features.iter().find(|f| f.fid == 2).unwrap(), "kind").as_deref(), Some("수혈"));
    assert!(t.layer.undo(), "Ctrl+Z takes one card edit back");
    let f = t.layer.features.iter().find(|f| f.fid == 2).unwrap();
    assert_eq!(attr(f, "kind").as_deref(), Some("주거지"));
    assert_eq!(attr(f, "note").as_deref(), Some("북벽 교란"));
    assert_eq!(card_value(&t, 2, "nothere", "x").unwrap_err(), "이 레이어에는 「nothere」 칸이 없습니다.");
}

/// ported from tests/test_feature_card.cpp:card_pointsOutRepeatedNumber.
#[test]
fn card_points_out_a_repeated_number() {
    let mut t = survey_features();
    assert_eq!(view(&t, 2).note_number, None);
    assert!(apply(&mut t, 2, "feature_no", "01호"));
    let note = view(&t, 2).note_number.expect("repeat");
    assert_eq!(note, "같은 종류에 같은 번호가 1개 더 있습니다. 제출 전 검수에서도 알려 드립니다.");
    assert_eq!(view(&t, 3).note_number, None, "수혈 1호 is another kind");
}

/// ported from tests/test_feature_card.cpp:card_headerLineIsKindAndNumber.
#[test]
fn card_header_is_number_and_kind() {
    let mut t = survey_features();
    assert_eq!(view(&t, 1).header, "1호 주거지");
    assert_eq!(view(&t, 1).layer_kind, "유구 면");
    assert!(apply(&mut t, 1, "feature_no", ""));
    assert_eq!(view(&t, 1).header, "주거지", "no number: the kind alone");
    assert_eq!(attr(t.layer.features.iter().find(|f| f.fid == 1).unwrap(), "feature_no"), None, "empty = NULL");
}

/// ported from tests/test_feature_card.cpp:card_measuresAreReadOnlyBoxes(값) — 면은 면적·둘레.
#[test]
fn card_measures_area_and_perimeter() {
    let t = survey_features();
    let v = view(&t, 1);
    assert_eq!(v.area.as_deref(), Some("50.00 m²"));
    assert_eq!(v.perimeter.as_ref().map(|(w, s)| (*w, s.as_str())), Some(("둘레", "30.00 m")));
}

/// ported from tests/test_feature_card.cpp:card_stateIsChipWithTone(글).
#[test]
fn card_state_words() {
    let mut t = survey_features();
    assert_eq!(view(&t, 2).state, CardState::Saved);
    assert!(apply(&mut t, 2, "note", "교란"));
    assert_eq!(view(&t, 2).state.text(), "변경됨 · 아직 저장 안 됨");
    assert_eq!(card_view(&t, 2, "유구", Some("x")).unwrap().state.text(), "바꾸지 못했습니다: x");
}

/// titleFor·layerKindFor·다음 번호·조사구역 머리글·기준점 줄.
#[test]
fn card_words_per_layer() {
    assert_eq!(title_for("survey_area_2"), "선택한 조사구역");
    assert_eq!(title_for("control_points"), "선택한 기준점");
    assert_eq!(title_for("trial_trench"), "선택한 도형");
    assert_eq!(layer_kind_for("artifact_point", "유물"), "유물 위치");
    assert_eq!(layer_kind_for("trial_trench", "시굴 트렌치"), "시굴 트렌치");
    let t = survey_features();
    assert_eq!(view(&t, 1).next_number, "3호");
    assert_eq!(view(&t, 3).next_number, "2호");
    assert_eq!(view(&t, 1).number_label, "유구번호");
    assert_eq!(view(&t, 1).sub_line, "유구 면 · 주거지 · 청동기");
}

/// 기준점(새 모양): 측지 글 + 「원점 비어 있음」, X·Y 는 천 단위 쉼표 소수 셋째, Z 는 「EL. 152.40」, 없으면 「—」.
#[test]
fn control_point_rows() {
    let mut layer = LayerEditBuffer::new(DomainLayerKey::ControlPoints);
    let attributes: BTreeMap<String, String> = [("point_id", "P1"), ("datum", "GRS80"), ("z", "152.4")].iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    layer.features.push(FeatureRecord { fid: 1, wkt_geom: "POINT (219897.0934 361622.354)".to_owned(), attributes });
    let t = SurveyFileTable {
        table: "control_points".to_owned(),
        srs_id: 5187,
        geometry_type: "POINT".to_owned(),
        columns: ["point_id", "x", "y", "z", "datum", "origin"].iter().map(|c| ((*c).to_owned(), "TEXT".to_owned())).collect(),
        baseline: layer.features.clone(),
        layer,
    };
    let v = card_view(&t, 1, "기준점", None).expect("card");
    let c = v.control.expect("rows");
    assert_eq!(c.meta, "GRS80 · 원점 비어 있음");
    assert_eq!((c.x.as_str(), c.y.as_str(), c.z.as_str()), ("219,897.093", "361,622.354", "EL. 152.40"));
    assert_eq!(v.header, "P1");
    assert_eq!(v.number_label, "점 이름");
    assert!(v.area.is_none() && v.perimeter.is_none());
}
