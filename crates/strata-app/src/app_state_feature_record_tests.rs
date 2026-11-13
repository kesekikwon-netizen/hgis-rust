//! 「도형 속성」 열기·저장 흐름 — demo 사본(임시 폴더)과 C++ 시험의 유구 칸 표.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use crate::app_state::{MainTab, StrataAppState};
use crate::checklist_state::attr;

fn opened_demo(dir: &Path) -> (StrataAppState, std::path::PathBuf) {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    (app, path)
}

fn poly(app: &StrataAppState, fid: i64) -> strata_storage::edit_buffer::FeatureRecord {
    app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.iter().find(|f| f.fid == fid).unwrap().clone()
}

/// 저장하면 편집 버퍼에만(파일은 「저장」), 제목·상태 글은 C++, Ctrl+Z 한 번에 옛 값.
#[test]
fn saving_writes_the_edit_buffer_as_one_undo_step() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    let before = poly(&app, 1);
    app.open_feature_record("feature_poly", 1);
    let form = app.home_map.feature_record.as_mut().expect("open");
    assert_eq!(form.title(), "도형 속성 — 유구 (면)");
    form.rows.iter_mut().find(|r| r.field == "note").unwrap().text = "바닥에서 숯이 나옴".to_owned();
    form.rows.iter_mut().find(|r| r.field == "period").unwrap().text = "삼국 시대".to_owned();
    app.save_feature_record();
    assert!(app.home_map.feature_record.is_none());
    let after = poly(&app, 1);
    assert_eq!(attr(&after, "note").as_deref(), Some("바닥에서 숯이 나옴"));
    assert_eq!(attr(&after, "period").as_deref(), Some("삼국시대"), "the survey's own spelling");
    assert_eq!(app.status_left_message(), "속성을 바꿨습니다: 유구 (면) (#1). Ctrl+Z로 되돌리고, 「저장」(Ctrl+S)으로 파일에 씁니다.");
    assert!(app.is_dirty);
    let note_in_file: Option<String> =
        rusqlite::Connection::open(&path).unwrap().query_row("SELECT note FROM feature_poly WHERE fid = 1", [], |r| r.get(0)).unwrap();
    assert_ne!(note_in_file.as_deref(), Some("바닥에서 숯이 나옴"), "nothing written before 저장");

    let n = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len();
    app.undo_map_action();
    assert_eq!(poly(&app, 1), before);
    assert_eq!(app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len(), n);
}

/// 「취소」는 아무것도 바꾸지 않는다.
#[test]
fn cancel_changes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    let before = poly(&app, 1);
    app.open_feature_record("feature_poly", 1);
    app.home_map.feature_record.as_mut().unwrap().rows[0].text = "수혈".to_owned();
    app.cancel_feature_record();
    assert_eq!(poly(&app, 1), before);
    assert!(!app.is_dirty);
}

/// 숫자 칸에 숫자가 아니면 창은 닫히고(C++ dlg.exec 뒤 검사) 아무 값도 바뀌지 않는다. updated_at 은 저장 때 찍힌다.
#[test]
fn a_bad_number_changes_nothing_and_a_good_save_touches_updated_at() {
    let mut app = StrataAppState::new_startup();
    app.survey_tables.push(crate::feature_record::tests::record_table());
    let before = app.survey_tables[0].layer.features[0].clone();
    app.open_feature_record("feature_poly", 1);
    app.home_map.feature_record.as_mut().unwrap().rows.iter_mut().find(|r| r.field == "depth_m").unwrap().text = "약 0.4".to_owned();
    app.save_feature_record();
    assert!(app.home_map.feature_record.is_none());
    assert_eq!(app.survey_tables[0].layer.features[0], before);
    assert!(app.status_left_message().starts_with("「깊이(m)」에 넣은 값이 숫자가 아닙니다: 약 0.4"));

    app.open_feature_record("feature_poly", 1);
    app.home_map.feature_record.as_mut().unwrap().rows.iter_mut().find(|r| r.field == "depth_m").unwrap().text = "0.42".to_owned();
    app.save_feature_record();
    let after = &app.survey_tables[0].layer.features[0];
    assert_eq!(attr(after, "depth_m").as_deref(), Some("0.42"));
    assert!(attr(after, "updated_at").is_some_and(|s| !s.is_empty()));
    assert_eq!(attr(after, "uid"), attr(&before, "uid"), "auto fields are not touched");
}

/// 없는 도형이면 C++ 글, 창은 열리지 않는다.
#[test]
fn a_missing_shape_does_not_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.open_feature_record("feature_poly", 999);
    assert!(app.home_map.feature_record.is_none());
    assert_eq!(app.status_left_message(), "고른 도형을 다시 읽지 못했습니다.");
}
