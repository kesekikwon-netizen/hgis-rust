//! 「도형 속성」 기록 — C++ tests/test_record_plate.cpp 기댓값과 MainWindowEditing.cpp:616-776 의 규칙.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::DomainLayerKey;

use super::*;

/// test_record_plate.cpp recordWindow — 유구 면(SurveySchema feature_poly 차례) + 단면번호, 그리고 자동 칸.
pub(crate) fn record_table() -> SurveyFileTable {
    let cols: &[(&str, &str)] = &[
        ("kind", "TEXT"),
        ("period", "TEXT"),
        ("feature_no", "TEXT"),
        ("note", "TEXT"),
        ("layer_no", "TEXT"),
        ("depth_m", "REAL"),
        ("top_el", "REAL"),
        ("bottom_el", "REAL"),
        ("relation", "TEXT"),
        ("status", "TEXT"),
        ("photo", "TEXT"),
        ("surveyor", "TEXT"),
        ("surv_date", "TEXT"),
        ("uid", "TEXT"),
        ("created_at", "TEXT"),
        ("updated_at", "TEXT"),
        ("section_id", "TEXT"),
    ];
    let values: &[(&str, &str)] = &[
        ("kind", "주거지"),
        ("period", "청동기"),
        ("feature_no", "1호"),
        ("note", "바닥에서 숯이 나옴"),
        ("layer_no", "Ⅲ-2"),
        ("top_el", "152.84"),
        ("bottom_el", "152.42"),
        ("relation", "1호 수혈보다 먼저"),
        ("status", "조사 중"),
        ("photo", "전경.jpg"),
        ("surveyor", "권영인"),
        ("surv_date", "2026-10-03"),
        ("uid", "00000000-0000-4000-8000-000000000000"),
    ];
    let mut layer = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
    let attributes: BTreeMap<String, String> = values.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    layer.features.push(FeatureRecord { fid: 1, wkt_geom: String::new(), attributes });
    SurveyFileTable {
        table: "feature_poly".to_owned(),
        srs_id: 5187,
        geometry_type: "POLYGON".to_owned(),
        columns: cols.iter().map(|(c, t)| ((*c).to_owned(), (*t).to_owned())).collect(),
        baseline: layer.features.clone(),
        layer,
    }
}

fn form() -> (SurveyFileTable, FeatureRecordForm) {
    let t = record_table();
    let f = FeatureRecordForm::new(&t, 1, "유구").expect("fields");
    (t, f)
}

fn row<'a>(f: &'a mut FeatureRecordForm, field: &str) -> &'a mut RecordRow {
    f.rows.iter_mut().find(|r| r.field == field).expect(field)
}

/// ported from tests/test_record_plate.cpp:fieldsComeInTheBoardSectionsTwoToARow(구역 낱말) — 식별·층위·표고·조사·사진 · 비고.
#[test]
fn fields_come_in_the_board_sections() {
    let (_, f) = form();
    assert_eq!(f.section_words(), vec!["식별", "층위", "표고", "조사", "사진 · 비고"]);
    let across: Vec<&str> = f.rows.iter().filter(|r| r.across).map(|r| r.field.as_str()).collect();
    assert_eq!(across, vec!["relation", "photo", "note"], "선후관계·사진·비고는 한 줄 전체");
}

/// ported from tests/test_record_plate.cpp:tabFollowsTheScreenOrder — 화면 차례(구역 안은 보드 차례, 이름 없는 칸은 식별 끝).
#[test]
fn rows_follow_the_board_order() {
    let (_, f) = form();
    let walked: Vec<&str> = f.rows.iter().map(|r| r.field.as_str()).chain(RECORD_BUTTONS).collect();
    assert_eq!(
        walked,
        vec![
            "kind", "period", "feature_no", "section_id", "layer_no", "relation", "top_el", "bottom_el", "depth_m", "status",
            "surv_date", "surveyor", "photo", "note", "저장", "취소"
        ]
    );
}

/// 자동 칸(uid·created_at·updated_at, SurveySchema::isAutoField)·fid·ogc_* 는 없다. 낱말은 attributeFieldLabelKo.
#[test]
fn auto_fields_are_left_out_and_words_are_korean() {
    let (_, f) = form();
    assert!(f.rows.iter().all(|r| !["uid", "created_at", "updated_at"].contains(&r.field.as_str())));
    let words: Vec<&str> = f.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(
        words,
        vec![
            "유구종류", "시대", "유구번호", "단면번호", "층위·문맥 번호", "선후관계", "상면 표고(m)", "바닥 표고(m)", "깊이(m)",
            "조사 상태", "조사일", "조사자", "사진", "비고"
        ]
    );
    assert_eq!(field_label_ko("z"), "표고 Z");
    assert_eq!(field_label_ko("own_field"), "own_field");
    assert_eq!(f.title(), "도형 속성 — 유구");
}

/// createValueEditor — 종류·시대는 목록+직접, 비고는 여러 줄(새 모양), 숫자 칸은 숫자, 나머지 한 줄. 지금 값이 들어 있다.
#[test]
fn editors_follow_the_field() {
    let (_, mut f) = form();
    assert_eq!(row(&mut f, "kind").editor, RecordEditor::Choice);
    assert_eq!(row(&mut f, "period").editor, RecordEditor::Choice);
    assert_eq!(row(&mut f, "note").editor, RecordEditor::Lines);
    assert_eq!(row(&mut f, "top_el").editor, RecordEditor::Real);
    assert_eq!(row(&mut f, "surveyor").editor, RecordEditor::Line);
    assert_eq!(row(&mut f, "top_el").text, "152.84");
    assert_eq!(row(&mut f, "depth_m").text, "");
    assert_eq!(placeholder_for("feature_no"), "예: 1호");
    assert_eq!(f.kind_choices[0], "주거지");
    assert_eq!(f.period_choices[0], "구석기");
}

/// ported from tests/test_record_plate.cpp:sectionPictureFollowsTheElevationBoxes·sectionPictureSaysWhatIsNotKnownYet.
#[test]
fn elevation_picture_reads_the_boxes() {
    assert_eq!(elevation_accessible_name("152.84", "152.42", ""), "상면 표고 152.84, 바닥 표고 152.42, 깊이 0.42 미터");
    assert_eq!(elevation_accessible_name("153.00", "152.42", ""), "상면 표고 153.00, 바닥 표고 152.42, 깊이 0.58 미터");
    assert_eq!(elevation_accessible_name("153.00", "152.42", "0.6"), "상면 표고 153.00, 바닥 표고 152.42, 깊이 0.60 미터");
    assert_eq!(elevation_accessible_name("", "152.42", ""), "상면 표고 —, 바닥 표고 152.42, 깊이 — 미터");
    assert_eq!(elevation_level("152.4"), "EL. 152.40");
    assert_eq!(elevation_level("숫자 아님"), "EL. —");
}

/// QString::toDouble·toLongLong(C 로캘)과 같은 숫자 규칙 — Qt 로 확인한 값.
#[test]
fn numbers_follow_qt() {
    for ok in ["inf", "-inf", "nan", "NaN", "+5", "1e3", ".5", "5."] {
        assert!(qt_to_double(ok).is_some(), "{ok}");
    }
    for bad in ["infinity", "1,5", "1 000", "0x10", "1.5e", "١٢"] {
        assert!(qt_to_double(bad).is_none(), "{bad}");
    }
    assert_eq!(qt_to_long_long("+5"), Some(5));
    assert_eq!(qt_to_long_long("1e3"), None);
}

/// 저장 값: 빈 칸은 NULL(None), 종류는 조사 맞춤법, 숫자는 숫자 글로, 칸마다 모두.
#[test]
fn values_clear_empty_boxes_and_keep_the_survey_spelling() {
    let (t, mut f) = form();
    row(&mut f, "kind").text = " 주거 지 ".to_owned();
    row(&mut f, "depth_m").text = "+0.5".to_owned();
    row(&mut f, "surveyor").text = "  ".to_owned();
    let values = f.values(&t).expect("valid");
    let get = |k: &str| values.iter().find(|(f, _)| f == k).map(|(_, v)| v.clone()).expect(k);
    assert_eq!(get("kind").as_deref(), Some("주거지"));
    assert_eq!(get("depth_m").as_deref(), Some("0.5"));
    assert_eq!(get("surveyor"), None);
    assert_eq!(get("section_id"), None);
    assert_eq!(values.len(), f.rows.len());
}

/// 숫자 칸에 숫자 아닌 값이면 C++ 알림 글(KaUserError)과 함께 아무것도 쓰지 않는다.
#[test]
fn a_bad_number_changes_nothing() {
    let (t, mut f) = form();
    row(&mut f, "depth_m").text = "약 0.4".to_owned();
    let w = f.values(&t).expect_err("bad number");
    assert_eq!(w.title, "속성");
    assert_eq!(w.text, "「깊이(m)」에 넣은 값이 숫자가 아닙니다: 약 0.4");
    assert_eq!(w.detail, "이 칸은 숫자만 받습니다. 아무 값도 바꾸지 않았습니다.");
    assert_eq!(w.action, "숫자만 넣고(소수점은 .) 다시 저장하세요.");
}

/// 이름·번호 칸이 하나도 없으면(자동 칸만) C++ 알림 글.
#[test]
fn a_layer_without_record_fields_says_so() {
    let mut t = record_table();
    t.columns.retain(|(c, _)| ["uid", "created_at", "updated_at"].contains(&c.as_str()));
    assert_eq!(FeatureRecordForm::new(&t, 1, "유구").expect_err("no fields"), NO_FIELDS);
    assert_eq!(NO_FIELDS, "이 레이어에 편집할 속성 필드가 없습니다.");
}
