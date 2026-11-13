//! 레이어 삭제·Ctrl+Z — 목록·지도에서만, 저장 안 한 편집은 되돌리기 단계 안에, 그동안 저장하지 않음. demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use crate::app_state::{MainTab, StrataAppState};

fn opened_demo(dir: &Path) -> (StrataAppState, std::path::PathBuf) {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

fn draw_pit(app: &mut StrataAppState) {
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    app.start_edit_feature_poly();
    for p in [[200_080.0, 450_060.0], [200_090.0, 450_060.0], [200_090.0, 450_070.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
    app.skip_feature_form();
}

fn rows(path: &Path) -> i64 {
    rusqlite::Connection::open(path).unwrap().query_row("SELECT count(*) FROM feature_poly", [], |r| r.get(0)).unwrap()
}

/// 빼면 목록·조사 표에서 없어지고(편집 레이어·카드도), C++ 상태 글. Ctrl+Z 면 같은 자리에 저장 안 한 도형까지 돌아온다.
#[test]
fn remove_and_undo_keep_the_unsaved_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    draw_pit(&mut app);
    let index = app.left_layers.entries.iter().position(|e| e.layer_key == "feature_poly").unwrap();
    let count = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len();
    assert!(app.remove_layers(&["feature_poly".to_owned()]));
    assert!(!app.left_layers.entries.iter().any(|e| e.layer_key == "feature_poly"));
    assert!(!app.survey_tables.iter().any(|t| t.table == "feature_poly"));
    assert_eq!(app.edit_layer_key, None);
    assert!(app.home_map.card.is_none());
    assert_eq!(app.status_left_message(), "레이어를 목록에서 제거했습니다. Ctrl+Z로 복원할 수 있습니다. 원본 파일은 그대로입니다.");
    app.undo_map_action();
    assert_eq!(app.left_layers.entries.iter().position(|e| e.layer_key == "feature_poly"), Some(index));
    assert_eq!(app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len(), count);
}

/// 빼 둔 동안 「저장」은 그 레이어를 쓰지 않고 다시 읽어도 돌아오지 않는다. 되돌린 뒤 저장하면 그 도형이 써진다.
#[test]
fn a_removed_layer_is_not_saved_until_restored() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    let before = rows(&path);
    draw_pit(&mut app);
    app.remove_layers(&["feature_poly".to_owned()]);
    assert!(app.save_survey_project());
    assert_eq!(rows(&path), before, "the removed layer's unsaved shape is not written");
    assert!(!app.survey_tables.iter().any(|t| t.table == "feature_poly"), "re-read does not bring it back");
    app.undo_map_action();
    assert!(app.save_survey_project());
    assert_eq!(rows(&path), before + 1);
}

/// 묶음 머리 「레이어 삭제」는 그 묶음의 레이어 모두. 빼 둔 레이어는 검수에 세지 않는다.
#[test]
fn group_removal_and_checklist() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    let polys = app.checklist_state()["feature_poly_count"].as_i64().unwrap_or(0);
    assert!(polys > 0);
    app.remove_layers(&["feature_poly".to_owned()]);
    assert_eq!(app.checklist_state()["feature_poly_count"].as_i64(), Some(0));
    app.undo_map_action();
    assert!(app.remove_group_layers(strata_types::schema::LegendGroup::SurveyData));
    assert!(!app.left_layers.entries.iter().any(|e| e.group == strata_types::schema::LegendGroup::SurveyData));
    assert!(!app.remove_layers(&[]));
    assert_eq!(app.status_left_message(), "제거할 레이어를 먼저 클릭하세요.");
}
