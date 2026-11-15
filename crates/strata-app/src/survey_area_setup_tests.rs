//! 「조사구역 그리기 설정」 — C++ 시험(test_survey_area_plate.cpp·test_survey_point_dialogs.cpp)의 값과
//! createSurveyAreaLayer(빈 survey_area 다시 쓰기·survey_area_N 새 표). 임시 폴더만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_canvas2d::map_tools::MapToolKind;

use strata_storage::survey_gpkg_read::SurveyFileTable;

use super::*;
use crate::app_state::MainTab;

/// ported from tests/test_survey_area_plate.cpp:paperWindowReadsLikeTheBoard — 색 8개(넷씩 두 줄), 첫 조사구역은 주황,
/// 첫 색을 고르면 빨강, 굵기 단추 넷 가운데 「기본 1.5」.
#[test]
fn window_values_follow_cpp() {
    let mut s = SurveyAreaSetup::default();
    assert_eq!((PALETTE.len(), SWATCHES_PER_ROW), (8, 4));
    assert_eq!(s.color_name(), "주황");
    s.color = 0;
    assert_eq!(s.color_name(), "빨강");
    assert_eq!(WIDTH_PRESETS.len(), 4);
    assert_eq!(s.chosen_preset(), Some(1));
    s.width_mm = 1.7;
    assert_eq!(s.chosen_preset(), None, "a typed width unticks every preset");
}

/// ported from tests/test_survey_point_dialogs.cpp:surveyAreaDialogIsCompact — 첫 조사구역 이름은 번호 없는 「조사구역」,
/// 비우면 「조사구역」(KaSurveyAreaDialog.cpp:262-265).
#[test]
fn layer_name_defaults_to_josaguyeok() {
    let mut s = SurveyAreaSetup::default();
    assert_eq!(s.layer_name(), "조사구역");
    s.name = "  ".to_owned();
    assert_eq!(s.layer_name(), "조사구역");
    s.name = " 북쪽 구역 ".to_owned();
    assert_eq!(s.layer_name(), "북쪽 구역");
    assert_eq!(s.style().fill, [0xEA, 0x58, 0x0C, 50]);
}

fn new_survey(dir: &std::path::Path) -> (StrataAppState, std::path::PathBuf) {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

/// 이어 그릴 조사구역이 없는 새 조사: 창이 열리고, 「그리기 시작」이면 빈 survey_area 표를 그 이름·색으로 쓰고 그리기 시작.
#[test]
fn new_survey_opens_the_window_and_reuses_the_empty_table() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = new_survey(dir.path());
    assert_eq!(app.survey_area_layer_to_continue(), None);
    app.start_edit_survey_area();
    assert_eq!(app.home_map.survey_area_setup, Some(SurveyAreaSetup::default()));
    assert_eq!(app.active_tool, MapToolKind::Pan, "drawing starts only after the window");
    app.home_map.survey_area_setup.as_mut().unwrap().name = "북쪽 구역".to_owned();
    app.accept_survey_area_setup();
    assert_eq!(app.edit_layer_key.as_deref(), Some("survey_area"));
    assert_eq!(app.active_tool, MapToolKind::DrawPolygon);
    let entry = &app.left_layers.entries[0];
    assert_eq!((entry.layer_key.as_str(), entry.label_ko.as_str()), ("survey_area", "북쪽 구역"));
    assert_eq!(app.home_map.real_survey.layer_styles.get("survey_area").map(|s| s.stroke), Some([0xEA, 0x58, 0x0C, 255]));
    assert_eq!(app.survey_area_layer_to_continue(), Some(("survey_area".to_owned(), "북쪽 구역".to_owned())));
}

/// 조사구역 레이어가 숨겨져 이어 그릴 것이 없으면 survey_area_2 새 표를 파일에 만들고, 그린 도형이 「저장」으로 그 표에 쓰인다.
#[test]
fn hidden_area_layer_gets_a_new_table_that_saves() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = new_survey(dir.path());
    app.start_edit_survey_area();
    app.accept_survey_area_setup();
    let first = app.left_layers.entries[0].label_ko.clone();
    app.home_map.real_survey.layer_visible.insert(first, false);
    app.right_inspector.selected_layer_key = None;
    app.start_edit_survey_area();
    app.accept_survey_area_setup();
    assert_eq!(app.edit_layer_key.as_deref(), Some("survey_area_2"));
    for p in [[21_100.0, 451_800.0], [21_160.0, 451_800.0], [21_160.0, 451_850.0], [21_100.0, 451_850.0]] {
        app.sketch_press(p);
    }
    app.sketch_finish();
    assert!(app.save_survey_project());
    let conn = rusqlite::Connection::open(&path).expect("open");
    let n: i64 = conn.query_row("SELECT count(*) FROM survey_area_2", [], |r| r.get(0)).expect("count");
    assert_eq!(n, 1);
}

fn area_table(name: &str, shapes: &[&str]) -> SurveyFileTable {
    let mut layer = LayerEditBuffer::new(DomainLayerKey::SurveyArea);
    for wkt in shapes {
        layer.add_unsaved_feature((*wkt).to_owned(), std::collections::BTreeMap::new());
    }
    SurveyFileTable { table: name.to_owned(), srs_id: 5187, geometry_type: "POLYGON".to_owned(), columns: Vec::new(), baseline: Vec::new(), layer }
}

/// The survey area the maps are centred on is the largest shape of all the survey-area layers, as its own
/// box: the first of two equal ones, the only one when there is one, and none without a shape.
#[test]
fn the_main_survey_area_is_the_largest_shape_of_all_survey_area_layers() {
    let mut app = StrataAppState::new_startup();
    assert_eq!(main_survey_area_extent(&app), None);
    app.survey_tables.push(area_table("survey_area", &[
        "POLYGON ((208000 442000, 208030 442000, 208030 442040, 208000 442000))",
        "POLYGON ((200000 450000, 200300 450000, 200300 450200, 200000 450200, 200000 450000))",
    ]));
    assert_eq!(main_survey_area_extent(&app), Some([200_000.0, 450_000.0, 200_300.0, 450_200.0]));
    assert_eq!(survey_area_extent(&app), Some([200_000.0, 442_000.0, 208_030.0, 450_200.0]), "every shape still counts where all are asked for");

    app.survey_tables.push(area_table("survey_area_2", &["POLYGON ((190000 460000, 190400 460000, 190400 460400, 190000 460400, 190000 460000))"]));
    assert_eq!(main_survey_area_extent(&app), Some([190_000.0, 460_000.0, 190_400.0, 460_400.0]), "a larger shape of another survey-area layer");

    app.survey_tables.push(area_table("survey_area_3", &["POLYGON ((180000 470000, 180400 470000, 180400 470400, 180000 470400, 180000 470000))"]));
    assert_eq!(main_survey_area_extent(&app), Some([190_000.0, 460_000.0, 190_400.0, 460_400.0]), "of two equal shapes the first");
}
