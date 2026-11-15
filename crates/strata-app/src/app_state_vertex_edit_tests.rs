//! The vertex handles of the selected shape, on a new survey in a temp folder — ported from the C++ tests
//! named at each test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use strata_storage::gpkg_wkt::wkt_to_parts;

use crate::app_state::{MainTab, StrataAppState};
use crate::app_state::vertex_edit_geom::vertices;

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    // One screen point is 0.1 m: the grab of 10 points reaches 1 m.
    app.home_map.viewport.meters_per_pixel = 0.1;
    app
}

fn add(app: &mut StrataAppState, table: &str, wkt: &str) -> i64 {
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::from_id(table).expect("a domain table"));
    app.survey_tables.iter_mut().find(|t| t.table == table).expect("table").layer.add_unsaved_feature(wkt.to_owned(), BTreeMap::new())
}

fn points(app: &StrataAppState, table: &str, fid: i64) -> Vec<[f64; 2]> {
    let t = app.survey_tables.iter().find(|t| t.table == table).expect("table");
    vertices(&wkt_to_parts(&t.layer.features.iter().find(|f| f.fid == fid).expect("shape").wkt_geom).expect("wkt"))
}

const PENTAGON: &str = "POLYGON ((200020 450020, 200060 450020, 200070 450045, 200040 450065, 200015 450045, 200020 450020))";

/// ported from tests/test_editing_tools.cpp:228-267 (clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex): a
/// click on a handle picks that vertex and changes nothing; Delete removes only it, once; a drag moves a
/// vertex as one more step. Words: KaFeatureSelectTool.cpp:78, 242-256.
#[test]
fn a_clicked_handle_is_picked_and_delete_removes_only_that_vertex() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let fid = add(&mut app, "feature_poly", PENTAGON);
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), fid));
    let steps = app.undo_tables.len();

    let handle = app.vertex_at([200_060.3, 450_020.2], 10.0).expect("the handle under the pointer");
    assert_eq!(app.vertex_at([200_045.0, 450_040.0], 10.0), None, "inside the shape there is no handle");
    app.pick_vertex(handle);
    assert_eq!((app.undo_tables.len(), app.status_left_message()), (steps, "점을 골랐습니다. Delete로 지우고, 끌면 옮깁니다."));

    assert!(app.delete_active_vertex());
    assert_eq!(points(&app, "feature_poly", fid).len(), 5);
    assert!(!points(&app, "feature_poly", fid).contains(&[200_060.0, 450_020.0]));
    assert_eq!((app.undo_tables.len(), app.status_left_message()), (steps + 1, "점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다."));
    assert!(!app.delete_active_vertex(), "no vertex is picked any more");

    let corner = app.vertex_at([200_070.0, 450_045.0], 10.0).unwrap();
    assert!(app.move_vertex_to(corner, [200_073.0, 450_043.8]));
    assert!(points(&app, "feature_poly", fid).contains(&[200_073.0, 450_043.8]));
    assert_eq!((app.undo_tables.len(), app.status_left_message()), (steps + 2, "수정점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다."));
    assert_eq!(app.home_map.selected_shape, Some(("feature_poly".to_owned(), fid)), "the shape stays selected");
    app.undo_map_action();
    assert!(points(&app, "feature_poly", fid).contains(&[200_070.0, 450_045.0]));
}

/// ported from tests/test_layer_edits.cpp:108-129 (sharedVertexMoveTouchesOnlyNeighbours): with 「공유 경계」
/// on, a moved corner takes the same corner of the shapes of its layer along (within 1 mm), as one step; a
/// shape apart stays. Without it only the moved shape changes.
#[test]
fn a_shared_corner_moves_with_its_neighbours_when_shared_edges_are_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let square = |x: f64, y: f64| format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + 10.0, x + 10.0, y + 10.0, y + 10.0);
    let left = add(&mut app, "feature_poly", &square(200_000.0, 450_000.0));
    let right = add(&mut app, "feature_poly", &square(200_010.0, 450_000.0));
    let far = add(&mut app, "feature_poly", &square(200_500.0, 450_500.0));
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), left));
    let steps = app.undo_tables.len();

    app.home_map.cad.snap_topological = true;
    assert!(app.move_vertex_to(2, [200_011.0, 450_012.0]));
    assert!(points(&app, "feature_poly", left).contains(&[200_011.0, 450_012.0]));
    assert!(points(&app, "feature_poly", right).contains(&[200_011.0, 450_012.0]));
    assert!(points(&app, "feature_poly", far).contains(&[200_500.0, 450_500.0]));
    assert_eq!(app.undo_tables.len(), steps + 1, "one step for both shapes");
    app.undo_map_action();
    assert!(points(&app, "feature_poly", right).contains(&[200_010.0, 450_010.0]));

    app.home_map.cad.snap_topological = false;
    assert!(app.move_vertex_to(2, [200_011.0, 450_012.0]));
    assert!(points(&app, "feature_poly", right).contains(&[200_010.0, 450_010.0]), "the neighbour stays");
}

/// A vertex goes into the side under the pointer (「점추가」, KaVertexEditTool.cpp:381-390), and a shape does
/// not go below its fewest points (`:361-373`), with the C++ words.
#[test]
fn a_vertex_is_put_into_a_side_and_the_fewest_points_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let fid = add(&mut app, "feature_poly", "POLYGON ((200000 450000, 200010 450000, 200005 450010, 200000 450000))");
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), fid));

    assert!(!app.delete_vertex(1));
    assert_eq!(app.status_left_message(), "면은 꼭짓점 3개보다 줄일 수 없습니다.");
    assert!(app.insert_vertex_at([200_005.0, 450_000.3]));
    assert_eq!(app.status_left_message(), "점을 넣었습니다.");
    assert_eq!(points(&app, "feature_poly", fid)[..3], [[200_000.0, 450_000.0], [200_005.0, 450_000.0], [200_010.0, 450_000.0]]);
    assert!(!app.insert_vertex_at([200_005.0, 450_005.0]), "no side under the pointer");
    assert!(app.delete_vertex(1));
    assert_eq!(app.status_left_message(), "점을 지웠습니다.");
}

/// Handles belong to exactly one selected line or polygon of an editable survey layer (`syncVertexTarget`):
/// a point has none.
#[test]
fn only_a_line_or_a_polygon_of_the_survey_has_handles() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    assert!(app.vertex_target().is_none(), "nothing selected");
    let point = add(&mut app, "control_points", "POINT (200000 450000)");
    app.home_map.selected_shape = Some(("control_points".to_owned(), point));
    assert!(app.vertex_target().is_none());
    let line = add(&mut app, "feature_line", "LINESTRING (200000 450000, 200010 450000)");
    app.home_map.selected_shape = Some(("feature_line".to_owned(), line));
    assert_eq!(app.vertex_target().map(|target| target.fid), Some(line));
    assert!(!app.delete_vertex(0));
    assert_eq!(app.status_left_message(), "선은 꼭짓점 2개보다 줄일 수 없습니다.");
}
