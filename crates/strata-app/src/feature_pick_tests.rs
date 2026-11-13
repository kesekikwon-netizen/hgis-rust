//! 도형선택 고르기 — C++ FeaturePick::candidates(core/FeaturePick.cpp) 차례.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))")
}

fn layer(key: &str, survey: bool, polygons: bool, features: Vec<(i64, String)>) -> PickLayer {
    PickLayer { key: key.to_owned(), survey, polygons, features }
}

fn first(layers: &[PickLayer], x: f64, y: f64) -> Option<(String, i64)> {
    pick_candidates(layers, [x, y], 1.0).into_iter().next()
}

/// ported from tests/test_polygon_erase.cpp:pick_takesTheSmallShapeDrawnOnTopEvenWhenUnsaved
#[test]
fn small_unsaved_shape_on_top_is_picked() {
    let layers = [layer("survey_area", true, true, vec![(1, rect(0.0, 0.0, 100.0, 100.0)), (-1, rect(40.0, 40.0, 60.0, 60.0))])];
    assert_eq!(first(&layers, 50.0, 50.0), Some(("survey_area".to_owned(), -1)));
    assert_eq!(first(&layers, 10.0, 10.0), Some(("survey_area".to_owned(), 1)));
    assert_eq!(first(&layers, 500.0, 500.0), None);
}

/// ported from tests/test_polygon_erase.cpp:pick_findsAShapeInsideAShapeOfTheLayerAbove
#[test]
fn shape_inside_a_shape_of_the_layer_above_is_picked() {
    let layers = [
        layer("survey_area", true, true, vec![(1, rect(0.0, 0.0, 100.0, 100.0))]),
        layer("feature_poly", true, true, vec![(7, rect(40.0, 40.0, 60.0, 60.0))]),
    ];
    assert_eq!(first(&layers, 50.0, 50.0), Some(("feature_poly".to_owned(), 7)));
    assert_eq!(first(&layers, 10.0, 10.0), Some(("survey_area".to_owned(), 1)));
}

/// ported from tests/test_polygon_erase.cpp:pick_prefersSurveyShapesAndNearbyLines
#[test]
fn survey_shapes_and_nearby_lines_come_first() {
    let parcels = layer("parcels", false, true, vec![(3, rect(45.0, 45.0, 55.0, 55.0))]);
    let area = layer("survey_area", true, true, vec![(1, rect(0.0, 0.0, 100.0, 100.0))]);
    let lines = layer("feature_line", true, false, vec![(9, "LINESTRING (0 80, 100 80)".to_owned())]);
    let layers = [parcels, area, lines];
    assert_eq!(first(&layers, 50.0, 50.0), Some(("survey_area".to_owned(), 1)));
    assert_eq!(first(&layers, 50.0, 80.5), Some(("feature_line".to_owned(), 9)));
    let only_parcels = [layer("parcels", false, true, vec![(3, rect(45.0, 45.0, 55.0, 55.0))])];
    assert_eq!(first(&only_parcels, 50.0, 50.0), Some(("parcels".to_owned(), 3)));
}

fn opened_demo(dir: &std::path::Path) -> crate::app_state::StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = crate::app_state::StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = crate::app_state::MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    app
}

/// 표(5186) 좌표를 작업 좌표계(5187)로.
fn work(x: f64, y: f64) -> [f64; 2] {
    strata_engine::Transformer::new(5186, 5187).expect("PROJ").transform([x, y]).expect("transform")
}

/// demo 사본: 유구 1호 안을 누르면 유구 → 조사구역 차례로 돌고(C++ 글), 기준점 가까이는 점이 먼저, 빈 자리는 「선택된 도형 없음」.
#[test]
fn select_on_demo_cycles_overlaps_like_cpp() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let inside_pit = work(200_016.0, 450_014.0);
    assert!(app.select_at(inside_pit, [500.0, 400.0]));
    assert_eq!(app.home_map.selected_shape, Some(("feature_poly".to_owned(), 1)));
    assert_eq!(app.status_left_message(), "겹친 도형 1/2 — 같은 자리를 다시 누르면 다음 도형을 고릅니다.");
    app.select_at(inside_pit, [501.0, 401.0]);
    assert_eq!(app.home_map.selected_shape, Some(("survey_area".to_owned(), 1)));
    assert_eq!(app.status_left_message(), "겹친 도형 2/2 — 같은 자리를 다시 누르면 다음 도형을 고릅니다.");
    app.select_at(inside_pit, [520.0, 400.0]);
    assert_eq!(app.home_map.selected_shape, Some(("feature_poly".to_owned(), 1)), "a click elsewhere starts again");

    app.select_at(work(200_000.0, 450_000.2), [300.0, 300.0]);
    assert_eq!(app.home_map.selected_shape.as_ref().map(|s| s.0.as_str()), Some("control_points"));

    assert!(!app.select_at(work(201_000.0, 451_000.0), [10.0, 10.0]));
    assert_eq!(app.home_map.selected_shape, None);
    assert_eq!(app.status_left_message(), "선택된 도형 없음");
}

/// 그린(저장 안 한) 작은 면은 그 자리에서 먼저 골라진다(음수 fid).
#[test]
fn select_picks_an_unsaved_drawing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.start_edit_feature_poly();
    for p in [[200_080.0, 450_060.0], [200_090.0, 450_060.0], [200_090.0, 450_070.0], [200_080.0, 450_070.0]] {
        app.sketch_press(work(p[0], p[1]));
    }
    app.sketch_finish();
    app.active_tool = strata_canvas2d::map_tools::MapToolKind::Select;
    app.select_at(work(200_085.0, 450_065.0), [0.0, 0.0]);
    let (table, fid) = app.home_map.selected_shape.clone().expect("picked");
    assert_eq!(table, "feature_poly");
    assert!(fid < 0);
}
