//! 「이름·번호」 창 — C++ tests/test_feature_card.cpp 의 기댓값.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::feature_form::tests::table;

/// test_feature_card.cpp surveyFeatures — 주거지 1호·2호, 수혈 1호(시대 청동기), 그리고 막 그린 빈 도형.
fn survey_features() -> SurveyFileTable {
    let mut t = table(
        &["kind", "period", "feature_no", "note", "updated_at"],
        &[
            &[("kind", "주거지"), ("period", "청동기"), ("feature_no", "1호")],
            &[("kind", "주거지"), ("period", "청동기"), ("feature_no", "2호")],
            &[("kind", "수혈"), ("period", "청동기"), ("feature_no", "1호")],
        ],
    );
    t.layer.features.push(strata_storage::edit_buffer::FeatureRecord { fid: -1, wkt_geom: String::new(), attributes: Default::default() });
    t
}

/// ported from tests/test_feature_card.cpp:form_suggestsNextNumberPerKindAndOnlyWarns
#[test]
fn form_suggests_next_number_per_kind_and_only_warns() {
    let t = survey_features();
    let mut form = FeatureForm::new(&t, -1).expect("offered");
    assert!(form.name_is_kind && form.has_period);
    assert_eq!(form.kind_choices[0], "주거지");
    assert_eq!(form.period_choices[0], "구석기");
    assert_eq!(form.name, "", "nothing is pre-chosen");
    assert_eq!(form.number, "1호");
    form.name = "주거지".to_owned();
    form.update_number_hint();
    assert_eq!(form.number, "3호");
    form.name = "수혈".to_owned();
    form.update_number_hint();
    assert_eq!(form.number, "2호");
    // 직접 적은 번호가 이긴다. 같은 번호는 알려 주기만 한다.
    form.number = "1호".to_owned();
    form.number_typed = true;
    form.update_number_hint();
    assert_eq!(form.number, "1호");
    assert_eq!(form.number_note().as_deref(), Some("같은 종류에 이미 있는 번호입니다(1호). 그래도 저장할 수 있습니다."));
    form.name = "구".to_owned();
    form.update_number_hint();
    assert_eq!(form.number, "1호", "kept: the user typed it");
    assert_eq!(form.number_note(), None);
}

/// ported from tests/test_feature_card.cpp:form_applyToWritesEditBufferOnly(값 부분) — 「주거 지」는 조사 맞춤법 「주거지」로, 번호 3호.
#[test]
fn form_values_use_the_survey_spelling() {
    let t = survey_features();
    let mut form = FeatureForm::new(&t, -1).expect("offered");
    form.name = "주거 지".to_owned();
    form.update_number_hint();
    form.period = "원삼국".to_owned();
    assert_eq!(form.name_text(&t), "주거지");
    assert_eq!(form.number_text(), "3호");
    assert_eq!(form.period_text(&t), "원삼국");
}
