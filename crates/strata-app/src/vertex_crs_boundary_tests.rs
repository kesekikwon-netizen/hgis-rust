//! Direct conversion boundaries, target/cpp-ref/src/app/KaVertexEditTool.cpp:110-133.
//! Explicit selection here is a unit fixture, not evidence of UI reachability.
use super::popup_input_harness::Harness;
use super::vertex_crs_input_tests::{fixture, records};

fn selected(source: i32) -> Harness {
    let mut h = fixture(source);
    h.app.home_map.selected_shape = Some(("feature_poly".to_owned(), 1));
    h
}

#[test]
fn vertex_crs_boundary_unknown_source_has_no_raw_work_handles() {
    for source in [0, -1, 999_999] {
        let h = selected(source);
        let target = h.app.vertex_target().expect("explicit unit fixture target");
        assert!(h.app.vertex_points_on_map(&target).is_empty(), "unknown source {source} exposed raw work handles");
        assert!(h.app.vertex_at([200_010.0, 450_010.0], 10.0).is_none());
        assert!(!h.app.side_at([200_016.0, 450_010.0], 10.0));
    }
}

#[test]
fn vertex_crs_boundary_unknown_source_move_preserves_records_history_and_file() {
    for source in [0, -1, 999_999] {
        let mut h = selected(source);
        let before = records(&h);
        let path = h.app.survey_path.clone().expect("temp source");
        let bytes = std::fs::read(&path).expect("source");
        assert!(!h.app.move_vertex_to(0, [200_012.0, 450_011.0]), "unknown source {source} admitted a raw move");
        assert_eq!(records(&h), before);
        assert!(!h.app.is_dirty && h.app.undo_tables.is_empty() && h.app.redo_tables.is_empty());
        assert_eq!(std::fs::read(&path).expect("unchanged file"), bytes);
    }
}

#[test]
fn vertex_crs_boundary_unknown_source_insert_preserves_records_and_history() {
    for source in [0, -1, 999_999] {
        let mut h = selected(source);
        let before = records(&h);
        assert!(!h.app.insert_vertex_at([200_016.0, 450_010.0]), "unknown source {source} admitted a raw insertion");
        assert_eq!(records(&h), before);
        assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
    }
}

#[test]
fn vertex_crs_boundary_failed_point_transform_never_falls_back_to_raw_move() {
    let mut h = selected(5186);
    let outside_domain = [1e100, 1e100];
    assert!(strata_engine::Transformer::new(5187, 5186).expect("known CRS").transform(outside_domain).is_err(), "fixture must exercise a real PROJ failure");
    let before = records(&h);
    assert!(!h.app.move_vertex_to(0, outside_domain), "failed PROJ conversion admitted a raw move");
    assert_eq!(records(&h), before);
    assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
}

#[test]
fn vertex_crs_boundary_failed_source_point_transform_has_no_raw_handles() {
    let mut h = selected(5186);
    h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("unit target").srs_id = 4326;
    assert!(strata_engine::Transformer::new(4326, 5187).expect("known CRS").transform([200_010.0, 450_010.0]).is_err(), "fixture values are not valid longitude/latitude");
    let target = h.app.vertex_target().expect("explicit unit selection");
    assert!(h.app.vertex_points_on_map(&target).is_empty(), "failed PROJ conversion exposed raw handles");
}
