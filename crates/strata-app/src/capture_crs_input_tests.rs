//! Capture target CRS and retained drafts, target/cpp-ref/src/app/KaCaptureMapTool.cpp:393-455.
use super::menu_selection_keys_tests::key;
use super::popup_input_harness::Harness;
use super::vertex_crs_input_tests::{fixture_table, records};
use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use strata_canvas2d::map_tools::MapToolKind;

fn start(table: &str, source: i32) -> Harness {
    let mut h = fixture_table(table, source);
    h.app.handle_ribbon_click("그리기");
    match table {
        "feature_poly" => h.app.start_edit_feature_poly(),
        "feature_line" => h.app.start_edit_feature_line(),
        "artifact_point" => h.app.start_edit_artifact(),
        _ => panic!("unsupported capture fixture"),
    }
    h.app.ribbon.sub_toolbar_visible = false;
    h.frame(vec![]);
    assert!(h.app.capture_active());
    assert_eq!(h.app.edit_layer_key.as_deref(), Some(table));
    assert_eq!(h.app.work_crs.auth_id(), "EPSG:5187");
    h
}

fn capture(h: &mut Harness) -> Vec<[f64; 2]> {
    let count = match h.app.active_tool {
        MapToolKind::PlacePoint => 1,
        MapToolKind::DrawLine => 2,
        MapToolKind::DrawPolygon => 3,
        _ => panic!("not a capture tool"),
    };
    let mut work = Vec::new();
    for position in [[700.0, 450.0], [820.0, 450.0], [820.0, 570.0]].into_iter().take(count) {
        let at = Pos2::from(position);
        h.frame(vec![Event::PointerMoved(at)]);
        work.push(h.app.home_map.cursor_world_xy);
        h.press(at, PointerButton::Primary);
    }
    if count != 1 {
        assert_eq!(h.app.draw_sketch.points, work, "actual input did not produce the intended draft");
        key(h, Key::Enter, Modifiers::default());
    }
    work
}

fn rejected(table: &str, source: i32) {
    rejected_capture(start(table, source), table, source);
}

fn rejected_capture(mut h: Harness, table: &str, source: i32) {
    let before = records(&h);
    let tool = h.app.active_tool;
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source");
    let draft = capture(&mut h);
    h.save(&format!("capture-crs-{table}-{source}-rejected"));
    assert_eq!(records(&h), before, "capture inserted raw work coordinates into an unknown target CRS");
    assert_eq!(h.app.draw_sketch.points, draft, "failed capture lost the pending draft");
    assert!(!h.app.is_dirty && h.app.undo_tables.is_empty() && h.app.redo_tables.is_empty());
    assert!(h.app.home_map.feature_form.is_none() && h.app.home_map.last_drawn_shape.is_none());
    assert!(h.app.status_left_message().starts_with("아직 넣지 않았습니다"));
    for _ in 0..2 {
        h.app.show_status_message("retry pending");
        key(&mut h, Key::Enter, Modifiers::default());
        assert!(h.app.status_left_message().starts_with("아직 넣지 않았습니다"), "Enter did not retry the failed capture");
        assert_eq!(records(&h), before);
        assert_eq!(h.app.draw_sketch.points, draft);
        assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
    }
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty());
    assert_eq!(h.app.active_tool, tool, "cancel changed the capture tool");
    assert!(h.app.capture_active());
    let again = capture(&mut h);
    assert_eq!(records(&h), before, "reusing capture after cancel inserted unknown coordinates");
    assert_eq!(h.app.draw_sketch.points, again);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty() && h.app.active_tool == tool);
    assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
    assert_eq!(records(&h), before);
    assert_eq!(std::fs::read(&path).expect("unchanged file"), bytes);
}

fn valid(table: &str, source: i32) {
    let mut h = start(table, source);
    let before = records(&h);
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source");
    let work = capture(&mut h);
    let old = &before.iter().find(|t| t.0 == table).expect("original table").1;
    let mut after = records(&h);
    let rows = &mut after.iter_mut().find(|t| t.0 == table).expect("target table").1;
    assert_eq!(rows.len(), old.len() + 1, "valid capture did not add exactly one feature");
    let added = rows.iter().find(|f| !old.iter().any(|o| o.fid == f.fid)).expect("new original identifier");
    let fid = added.fid;
    let parts = strata_storage::gpkg_wkt::wkt_to_parts(&added.wkt_geom).expect("new geometry");
    let stored = crate::app_state::vertex_edit_geom::vertices(&parts);
    assert_eq!(stored.len(), work.len() + usize::from(table == "feature_poly"));
    let to_work = strata_engine::Transformer::new(u32::try_from(source).expect("known table CRS"), 5187).expect("known conversion");
    let shown = to_work.transform_all(&stored).expect("stored coordinates valid");
    // target/cpp-ref/tests/test_editing_tools.cpp:263: 1e-6 m local placement control, not an area-wide error bound.
    for expected in &work {
        assert!(shown.iter().any(|p| (p[0] - expected[0]).hypot(p[1] - expected[1]) < 1e-6), "EPSG:{source} capture missed an actual work-CRS EPSG:5187 input");
    }
    rows.retain(|f| f.fid != fid);
    assert_eq!(after, before, "capture changed pre-existing records or attributes");
    assert!(h.app.draw_sketch.points.is_empty() && h.app.is_dirty && h.app.undo_tables.len() == 1);
    if h.app.home_map.feature_form.is_some() {
        let skip = h.point_at("건너뛰기");
        h.press(skip, PointerButton::Primary);
        assert!(h.app.home_map.feature_form.is_none(), "actual skip button did not return to capture");
    }
    h.save(&format!("capture-crs-{table}-{source}-valid"));
    key(&mut h, Key::Z, Modifiers { ctrl: true, ..Default::default() });
    assert_eq!(records(&h), before, "actual Undo did not restore original records");
    assert_eq!(std::fs::read(&path).expect("source remains"), bytes);
}

#[test]
fn capture_crs_input_undefined_polygon_never_inserts_raw_coordinates() {
    rejected("feature_poly", 0);
}
#[test]
fn capture_crs_input_negative_polygon_never_inserts_raw_coordinates() {
    rejected("feature_poly", -1);
}
#[test]
fn capture_crs_input_undefined_line_never_inserts_raw_coordinates() {
    rejected("feature_line", 0);
}
#[test]
fn capture_crs_input_negative_line_never_inserts_raw_coordinates() {
    rejected("feature_line", -1);
}
#[test]
fn capture_crs_input_undefined_point_never_inserts_raw_coordinates() {
    rejected("artifact_point", 0);
}
#[test]
fn capture_crs_input_negative_point_never_inserts_raw_coordinates() {
    rejected("artifact_point", -1);
}
#[test]
fn capture_crs_input_unknown_point_retains_failed_draft_until_cancel() {
    rejected("artifact_point", 999_999);
}
#[test]
fn capture_crs_input_unknown_line_retains_failed_draft_until_cancel() {
    rejected("feature_line", 999_999);
}
#[test]
fn capture_crs_input_unknown_polygon_retains_failed_draft_until_cancel() {
    rejected("feature_poly", 999_999);
}

#[test]
fn capture_crs_input_known_crs_point_transform_failure_retains_draft_until_cancel() {
    let mut h = start("artifact_point", 5186);
    // target/cpp-ref/src/app/KaCaptureMapTool.cpp:437-447: a known transform can reject a point outside its domain.
    let outside_domain = [100_000_000.0, 100_000_000.0];
    assert!(strata_engine::Transformer::new(5187, 5186).expect("known CRS pair").transform(outside_domain).is_err(), "fixture must cause a real point-conversion error");
    h.app.home_map.viewport.center_xy = outside_domain;
    h.frame(vec![]);
    rejected_capture(h, "artifact_point", 5186);
}
#[test]
fn capture_crs_input_cross_crs_polygon_preserves_originals_and_undo() {
    valid("feature_poly", 5186);
}
#[test]
fn capture_crs_input_identity_polygon_preserves_originals_and_undo() {
    valid("feature_poly", 5187);
}
#[test]
fn capture_crs_input_cross_crs_line_preserves_originals_and_undo() {
    valid("feature_line", 5186);
}
#[test]
fn capture_crs_input_identity_line_preserves_originals_and_undo() {
    valid("feature_line", 5187);
}
#[test]
fn capture_crs_input_cross_crs_point_preserves_originals_and_undo() {
    valid("artifact_point", 5186);
}
#[test]
fn capture_crs_input_identity_point_preserves_originals_and_undo() {
    valid("artifact_point", 5187);
}
