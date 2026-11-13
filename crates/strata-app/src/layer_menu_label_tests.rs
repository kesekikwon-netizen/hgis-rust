//! 라벨 상태 — C++ tests/test_layer_labels.cpp 기댓값(우리 라벨 모양으로).

use super::*;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_owned()).collect()
}

/// ported from tests/test_layer_labels.cpp:unknownVectorDoesNotExposeOpaqueFields
#[test]
fn unknown_vector_does_not_expose_opaque_fields() {
    let info = describe_label(&LabelSetting::Unconfigured, &s(&["PNU", "A1", "fid"]), true, true, "외부 자료", false);
    assert!(!info.supported && info.needs_field && info.field.is_empty() && !info.area_editable);
}

/// ported from tests/test_layer_labels.cpp:knownFieldsAndSurveyArea — 지번, 조사구역(라벨 없음·면적 라벨) 면적.
#[test]
fn known_fields_and_survey_area() {
    let cad = describe_label(&LabelSetting::Unconfigured, &s(&["PNU", "JIBUN"]), true, true, "지적도", false);
    assert_eq!((cad.caption.as_str(), cad.field.as_str()), ("지번", "JIBUN"));
    assert!(!cad.area_editable);
    assert!(describe_label(&LabelSetting::Unconfigured, &[], true, true, "survey_area", false).supported);
    let area = describe_label(&LabelSetting::Area(None), &[], true, true, "survey_area", false);
    assert!(area.area_editable);
    assert_eq!(area.caption, "면적");
}

/// ported from tests/test_layer_labels.cpp:rasterHasNoMisleadingCheckbox
#[test]
fn raster_has_no_misleading_checkbox() {
    let info = describe_label(&LabelSetting::Unconfigured, &[], false, false, "위성영상", false);
    assert!(!info.supported && !info.needs_field);
    assert!(info.reason.contains("이미지"));
}

/// ported from tests/test_layer_labels.cpp:specializedLabelsRejectGenericContentChanges — 지번·기호 라벨은 항목을 못 바꾼다.
#[test]
fn specialized_labels_reject_field_changes() {
    assert!(!describe_label(&LabelSetting::Field("JIBUN".into()), &s(&["JIBUN", "PNU"]), true, true, "지적도", false).field_editable);
    assert!(!describe_label(&LabelSetting::Field("기호".into()), &s(&["기호", "name"]), true, true, "지질도", false).field_editable);
}

/// ported from tests/test_layer_labels.cpp:areaWithoutNameNeverFallsBackToOpaqueField — 면적만·빈 식은 항목 없이 면적만 고칠 수 있다.
#[test]
fn area_without_name_never_falls_back_to_an_opaque_field() {
    let fields = s(&["opaque"]);
    assert!(describe_label(&LabelSetting::Area(None), &fields, true, true, "x", false).field.is_empty());
    let off = describe_label(&LabelSetting::Empty, &fields, true, true, "x", false);
    assert!(off.area_editable && off.field.is_empty());
}

/// 우리 조사 레이어 라벨의 글자 — 유구 면·조사구역(면적) 「면적」, 시굴(name+면적) 「이름·면적」, 유구 선 번호 「번호」, 기준점 이름 「이름」.
#[test]
fn survey_layer_captions() {
    let cap = |set: LabelSetting, fields: &[&str], polygon: bool, key: &str| describe_label(&set, &s(fields), true, polygon, key, false).caption;
    assert_eq!(cap(LabelSetting::Area(None), &["kind", "feature_no"], true, "feature_poly"), "면적");
    assert_eq!(cap(LabelSetting::Area(Some("name".into())), &["name"], true, "trial_trench"), "이름·면적");
    assert_eq!(cap(LabelSetting::Field("feature_no".into()), &["kind", "feature_no"], false, "feature_line"), "번호");
    assert_eq!(cap(LabelSetting::Field("name".into()), &["name"], false, "control_points"), "이름");
    assert_eq!(field_title("feature_no"), "유구 번호");
    assert_eq!(field_title("fid"), "fid");
}
