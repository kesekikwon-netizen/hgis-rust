//! Normal and individual-failure controls, target/cpp-ref/tests/test_buffer.cpp:35-125.
use super::{buffer, start, state, Harness};

fn prepared(source: i32, invalid: u8) -> Harness {
    let mut h = start(source);
    let table = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("source");
    let first = table.layer.features[0].fid;
    let second = table.layer.features[1].fid;
    let normal = "POLYGON ((200000 450000,200080 450000,200080 450060,200000 450060,200000 450000))".to_owned();
    let outside = "POLYGON ((100000000 100000000,100000080 100000000,100000080 100000060,100000000 100000060,100000000 100000000))".to_owned();
    if invalid != 0 {
        assert!(
            strata_engine::Transformer::new(5186, 5187).expect("known pair").transform([100_000_000.0, 100_000_000.0]).is_err(),
            "fixture needs a real point-conversion error"
        );
    }
    let changes = match invalid {
        0 => vec![(first, normal)],
        1 => vec![(first, normal), (second, outside)],
        _ => vec![(first, outside.clone()), (second, outside)],
    };
    let deleted = if invalid == 0 { vec![second] } else { vec![] };
    assert!(table.layer.change_add_delete(&changes, vec![], &deleted).is_some());
    h.app.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table("feature_poly".to_owned()));
    h.app.is_dirty = true;
    h.app.after_layer_edit("feature_poly");
    h.app.home_map.viewport.center_xy = strata_engine::Transformer::new(u32::try_from(source).expect("known source"), 5187)
        .expect("known pair")
        .transform([200_040.0, 450_030.0])
        .expect("local view");
    h.frame(vec![]);
    h
}

fn valid(source: i32, meters: u32, partial: bool) {
    let mut h = prepared(source, u8::from(partial));
    let before_records = super::records(&h);
    let path = h.app.survey_path.clone().expect("temp copy");
    let bytes = std::fs::read(&path).expect("file");
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    buffer(&mut h, meters);
    let ring = h.app.home_map.real_survey.distance_rings.get(&meters).expect("actual generated ring");
    assert_eq!((ring.lines.len(), ring.labels.len()), (1, 4));
    let points: Vec<_> = ring.lines.iter().flatten().copied().collect();
    let corners = [[200_000.0, 450_000.0], [200_080.0, 450_000.0], [200_080.0, 450_060.0], [200_000.0, 450_060.0]];
    let work = strata_engine::Transformer::new(u32::try_from(source).expect("source"), 5187)
        .expect("known pair")
        .transform_all(&corners)
        .expect("local corners");
    let bounds = |p: &[[f64; 2]]| {
        p.iter()
            .fold([f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY], |b, p| [b[0].min(p[0]), b[1].max(p[0]), b[2].min(p[1]), b[3].max(p[1])])
    };
    let actual = bounds(&points);
    let original = bounds(&work);
    let r = f64::from(meters);
    let expected = [original[0] - r, original[1] + r, original[2] - r, original[3] + r];
    // target/cpp-ref/tests/test_buffer.cpp:69-74: literal 500m bounds and 12m acceptance control.
    if source == 5187 && meters == 500 {
        for (a, e) in actual.into_iter().zip([199_500.0, 200_580.0, 449_500.0, 450_560.0]) {
            assert!((a - e).abs() < 12.0);
        }
    }
    // target/cpp-ref/src/core/BufferAnalysis.cpp:220: 36 segments/quadrant; R*(1-cos(pi/144)) <0.239m for R<=1000m.
    // 0.3m includes that local chord bound and floating-point rounding; not an area-wide CRS accuracy claim.
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < 0.3, "EPSG:{source} buffer extent {a} missed work-CRS EPSG:5187 bound {e}");
    }
    let count = |h: &Harness| {
        h.app
            .left_layers
            .entries
            .iter()
            .filter(|e| e.layer_key == format!("user:buffer_ring_{meters}") || e.layer_key == format!("user:buffer_label_{meters}"))
            .count()
    };
    assert_eq!(count(&h), 2);
    h.save(&format!("buffer-crs-{source}-{meters}-{partial}-valid"));
    let first = state(&h);
    h.app.show_status_message("repeat pending");
    buffer(&mut h, meters);
    assert_eq!(state(&h), first, "repeated buffer changed original data, geometry, history or legend order");
    assert_eq!(h.app.status_left_message(), format!("주변 {meters}m 경계를 그렸습니다"));
    assert_eq!(count(&h), 2);
    assert_eq!(super::records(&h), before_records);
    assert!(h.app.is_dirty);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    assert_eq!(std::fs::read(&path).expect("unchanged file"), bytes);
}

fn known_failure(meters: u32) {
    let mut h = prepared(5186, 2);
    let before = state(&h);
    buffer(&mut h, meters);
    assert_eq!(state(&h), before);
    assert_eq!(h.app.status_left_message(), format!("{meters}m 경계를 그리지 못했습니다. 선택한 레이어에 도형이 없습니다"));
}

#[test]
fn buffer_crs_input_identity_500_matches_cpp_bounds_and_repeat() {
    valid(5187, 500, false);
}
#[test]
fn buffer_crs_input_identity_1000_matches_local_bounds_and_repeat() {
    valid(5187, 1000, false);
}
#[test]
fn buffer_crs_input_cross_crs_500_matches_local_bounds_and_repeat() {
    valid(5186, 500, false);
}
#[test]
fn buffer_crs_input_cross_crs_1000_matches_local_bounds_and_repeat() {
    valid(5186, 1000, false);
}
#[test]
fn buffer_crs_input_known_point_failure_500_never_creates_ring() {
    known_failure(500);
}
#[test]
fn buffer_crs_input_known_point_failure_1000_never_creates_ring() {
    known_failure(1000);
}
#[test]
fn buffer_crs_input_valid_source_skips_only_failed_feature() {
    valid(5186, 500, true);
}
