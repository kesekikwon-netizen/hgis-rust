//! 「폴리곤 나누기」 on a new survey in a temp folder — ported from tests/test_buffer.cpp:280-376 and
//! tests/test_save_open.cpp:3854-3918, with the words of MainWindowEditing.cpp:794-816, 1193-1280.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.1;
    app
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))")
}

fn add(app: &mut StrataAppState, table: &str, wkt: &str) -> i64 {
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::from_id(table).unwrap());
    let values = BTreeMap::from([("feature_no".to_owned(), "1호".to_owned())]);
    app.survey_tables.iter_mut().find(|t| t.table == table).unwrap().layer.add_unsaved_feature(wkt.to_owned(), values)
}

fn areas(app: &StrataAppState, table: &str) -> Vec<f64> {
    let t = app.survey_tables.iter().find(|t| t.table == table).unwrap();
    t.layer.features.iter().map(|f| strata_engine::wkt_area(&f.wkt_geom).unwrap().abs()).collect()
}

/// With one polygon selected the button starts drawing a line across it, with the C++ words; a line beside
/// the square cuts nothing and says why; a line across cuts it in two pieces of 100 together, keeping the
/// values, as one step Ctrl+Z takes back; the mode stays for the next line.
#[test]
fn a_line_drawn_across_a_polygon_cuts_it_in_two() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let square = add(&mut app, "feature_poly", &rect(200_000.0, 450_000.0, 200_010.0, 450_010.0));
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), square));
    app.start_split_polygon_tool();
    assert_eq!((app.active_tool, app.home_map.splitting.as_deref()), (MapToolKind::DrawLine, Some("feature_poly")));
    assert_eq!(app.status_left_message(), "폴리곤 나누기 모드 — 폴리곤을 가로지르는 선을 클릭하여 그리고 우클릭으로 분할 (또는 Shift로 2개 도형 선택 후 실행).");
    assert_eq!(app.tool_chip_label(), "나누기 선");

    app.split_polygons_with_line("feature_poly", vec![[199_995.0, 449_995.0], [199_999.0, 449_999.0]]);
    assert_eq!(app.status_left_message(), "선으로 면을 나누지 못했습니다. 분할선이 관통하는 폴리곤이 없습니다. 폴리곤 양쪽 경계를 완전히 가로질러야 합니다.");
    assert_eq!(areas(&app, "feature_poly").len(), 1);

    let steps = app.undo_tables.len();
    for xy in [[200_005.0, 449_998.0], [200_005.0, 450_012.0]] {
        app.sketch_press(xy);
    }
    app.sketch_finish();
    let cut = areas(&app, "feature_poly");
    assert_eq!(cut.len(), 2, "{cut:?}");
    assert!((cut.iter().sum::<f64>() - 100.0).abs() < 0.1);
    assert_eq!(app.status_left_message(), "폴리곤을 성공적으로 나누었습니다.");
    assert_eq!(app.undo_tables.len(), steps + 1);
    let values: Vec<_> = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.iter().map(|f| f.attributes.get("feature_no").cloned()).collect();
    assert!(values.iter().all(|v| v.as_deref() == Some("1호")), "{values:?}");
    assert!(app.home_map.splitting.is_some(), "the mode stays");
    app.undo_map_action();
    assert_eq!(areas(&app, "feature_poly").len(), 1);
    app.stop_capture_tool();
    app.drop_sketch_when_tool_changed();
    assert!(app.home_map.splitting.is_none(), "another tool ends the mode");
}

/// Two overlapping shapes selected with Shift give their shared place as a new shape — on the second layer
/// when the first is the survey area — and stay as they were (ported from tests/test_buffer.cpp:316-376).
#[test]
fn two_selected_shapes_give_their_shared_place() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let area = add(&mut app, "survey_area", &rect(200_000.0, 450_000.0, 200_020.0, 450_020.0));
    let pit = add(&mut app, "feature_poly", &rect(200_010.0, 450_000.0, 200_030.0, 450_020.0));
    (app.home_map.also_selected, app.home_map.selected_shape) = (vec![("survey_area".to_owned(), area)], Some(("feature_poly".to_owned(), pit)));
    let steps = app.undo_tables.len();
    app.start_split_polygon_tool();
    assert_eq!(areas(&app, "survey_area"), [400.0]);
    let pits = areas(&app, "feature_poly");
    assert_eq!(pits.len(), 2, "{pits:?}");
    assert!(pits.iter().any(|a| (a - 400.0).abs() < 0.5) && pits.iter().any(|a| (a - 200.0).abs() < 0.5));
    assert_eq!(app.status_left_message(), "선택한 두 도형의 겹치는 구간을 잘라 분할했습니다! (A, B, 중첩부 3개로 분할됨)");
    assert_eq!(app.undo_tables.len(), steps + 1);

    let far = add(&mut app, "feature_poly", &rect(200_100.0, 450_100.0, 200_110.0, 450_110.0));
    (app.home_map.also_selected, app.home_map.selected_shape) = (vec![("survey_area".to_owned(), area)], Some(("feature_poly".to_owned(), far)));
    app.start_split_polygon_tool();
    assert_eq!(app.status_left_message(), "두 도형의 겹친 구간을 나누지 못했습니다. 선택한 두 도형이 서로 겹치지 않습니다. 겹치는 도형 두 개를 Shift+클릭으로 다시 고른 뒤 누르세요.");
}

/// A selected shape of several polygons comes apart into one shape each (`explodeMultipartFeatures`); without
/// a selection or a polygon layer chosen the button says what to do.
#[test]
fn a_shape_of_several_polygons_comes_apart_and_nothing_chosen_says_what_to_do() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let two = add(&mut app, "feature_poly", "MULTIPOLYGON (((200000 450000, 200010 450000, 200010 450010, 200000 450010, 200000 450000)), ((200100 450100, 200105 450100, 200105 450105, 200100 450105, 200100 450100)))");
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), two));
    app.start_split_polygon_tool();
    let mut parts = areas(&app, "feature_poly");
    parts.sort_by(f64::total_cmp);
    assert_eq!(parts, [25.0, 100.0]);
    assert_eq!(app.status_left_message(), "그룹으로 묶여 있던 폴리곤을 개별 폴리곤들로 분리했습니다!");

    let other = tempfile::tempdir().unwrap();
    let mut fresh = new_survey(other.path());
    fresh.home_map.selected_shape = None;
    fresh.right_inspector.selected_layer_key = None;
    fresh.start_split_polygon_tool();
    assert_eq!(fresh.status_left_message(), "도형을 Shift+클릭으로 선택하거나, 나눌 폴리곤 레이어를 좌측 목록에서 선택해 주세요.");
    assert!(fresh.home_map.splitting.is_none());
}
