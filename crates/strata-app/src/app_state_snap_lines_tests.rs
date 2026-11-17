//! 「가까운 선에 붙이기」 — 열어 둔 조사 표(저장 안 한 도형 포함)의 대상을 붙이고, 되돌릴 수 있다. 임시 폴더만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::{MainTab, StrataAppState};
use crate::real_survey::RealCadastralParcel;

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app
}

fn outer_ring(app: &StrataAppState) -> Vec<[f64; 2]> {
    let t = app.survey_tables.iter().find(|t| t.table == "survey_area").expect("table");
    let parts = wkt_to_parts(&t.layer.features.last().expect("feature").wkt_geom).expect("wkt");
    let WkbGeometry::Polygon(rings) = &parts[0] else { panic!("polygon") };
    rings[0].clone()
}

/// ported from tests/test_parcel_copy_keys.cpp:snapTraceShowsNearbyVerticesOnlyWhenCloseAndFillsSkippedVertex — 지도
/// 80 m 를 640 px(점당 0.125 m)로 보고, 도면 선 위에 대략 그린(저장 안 한) 조사구역을 붙이면 6점(빠진 사이 꼭짓점
/// (200010, 450001.2) 채움 + 닫는 점). 되돌리면 그린 그대로.
#[test]
fn just_drawn_area_snaps_to_the_line_and_undoes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = new_survey(dir.path());
    app.home_map.real_survey.cadastral_parcels.push(RealCadastralParcel {
        jibun: "도면".to_owned(),
        ring: vec![[200_000.0, 450_000.0], [200_010.0, 450_001.2], [200_020.0, 450_000.0]],
        center: [200_010.0, 450_000.0],
    });
    app.home_map.viewport.meters_per_pixel = 0.125;
    app.start_edit_survey_area();
    app.accept_survey_area_setup(); // 이어 그릴 조사구역이 없으면 「조사구역 그리기 설정」 창 → 「그리기 시작」
    for p in [[200_001.2, 449_999.1], [200_019.0, 450_000.8], [200_020.8, 450_019.2], [199_999.3, 450_020.6]] {
        app.sketch_press(p);
    }
    app.sketch_finish();
    let drawn = outer_ring(&app);

    assert_eq!(app.snap_shapes_to_nearest_lines(), 1);
    let snapped = outer_ring(&app);
    assert_eq!(snapped.len(), 6, "{snapped:?}");
    assert!(snapped.iter().any(|p| (p[0] - 200_010.0).abs() < 1e-6 && (p[1] - 450_001.2).abs() < 1e-6));
    assert!(app.status_left_message().starts_with("도형을 가장 가까운 선에 붙였습니다 (꼭짓점 "));
    assert!(app.is_dirty);

    app.undo_map_action();
    assert_eq!(outer_ring(&app), drawn);
}

/// 대상이 없으면 C++ 안내 글.
#[test]
fn nothing_to_snap_says_so() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = new_survey(dir.path());
    assert_eq!(app.snap_shapes_to_nearest_lines(), 0);
    assert_eq!(app.status_left_message(), "먼저 지적도·도면 위에 도형을 그리거나 도형선택으로 도형을 고르세요.");
}

/// A received cadastral map with one parcel, shown on the map and read (one map frame).
fn with_received_cadastral_map(app: &mut StrataAppState, dir: &std::path::Path) {
    use strata_engine::{write_new_table, Cell, ColumnKind, Row, ShapeKind, Shapes, TableLayout};
    let file = dir.join("지적도-시험.gpkg");
    let layout = TableLayout { table: "cadastral", epsg: 5187, kind: ShapeKind::Polygons, columns: &[("PNU", ColumnKind::Text), ("JIBUN", ColumnKind::Text)] };
    let ring = vec![[200_000.0, 450_000.0], [200_010.0, 450_001.2], [200_020.0, 450_000.0], [200_020.0, 449_980.0], [200_000.0, 449_980.0], [200_000.0, 450_000.0]];
    let parcel = Row { shapes: Shapes::Polygons(vec![vec![ring]]), cells: vec![Cell::Text("4713010100100010000".to_owned()), Cell::Text("1대".to_owned())] };
    write_new_table(&file, &layout, &mut vec![parcel].into_iter()).expect("write the test table");
    app.ref_vectors.run_at_once();
    app.show_cadastral_layer(file, None);
    app.home_map.viewport = strata_canvas2d::viewport::ViewportTransform::new([200_010.0, 450_005.0], 0.125, [640.0, 480.0]);
    let ctx = eframe::egui::Context::default();
    let rect = eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(640.0, 480.0));
    for _ in 0..3 {
        let _ = ctx.run(eframe::egui::RawInput { screen_rect: Some(rect), ..Default::default() }, |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| app.paint_reference_vector_layers(ctx, &ui.painter_at(rect), rect));
        });
    }
    assert!(app.ref_vectors.extent_of(crate::ref_vector::cadastral::layer::CADASTRAL_KEY).is_some(), "the received map is read");
}

fn draw_area_along_the_parcel(app: &mut StrataAppState) {
    app.start_edit_survey_area();
    app.accept_survey_area_setup();
    for p in [[200_001.2, 449_999.1], [200_019.0, 450_000.8], [200_020.8, 450_019.2], [199_999.3, 450_020.6]] {
        app.sketch_press(p);
    }
    app.sketch_finish();
}

/// Owner 2026-11-15: 「그린도형 가까운곳에 붙이기 가능하게하라」. The lines a shape is snapped to are those of the
/// received cadastral map while its row is checked (C++ `isSnapSourceLayer`, LayerOps.cpp:1278-1282). Before,
/// the snap read a list that only the old sample data filled, so on a real survey there was never a line to
/// snap to. The same area as above: two corners go onto the parcel's corners and the corner between them is
/// filled in.
#[test]
fn a_drawn_area_snaps_to_the_received_cadastral_map() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = new_survey(dir.path());
    with_received_cadastral_map(&mut app, dir.path());
    draw_area_along_the_parcel(&mut app);

    assert_eq!(app.snap_shapes_to_nearest_lines(), 1);
    let snapped = outer_ring(&app);
    assert_eq!(snapped.len(), 6, "{snapped:?}");
    for corner in [[200_000.0, 450_000.0], [200_010.0, 450_001.2], [200_020.0, 450_000.0]] {
        assert!(snapped.iter().any(|p| (p[0] - corner[0]).abs() < 0.002 && (p[1] - corner[1]).abs() < 0.002), "{corner:?} in {snapped:?}");
    }
    assert!(app.status_left_message().starts_with("도형을 가장 가까운 선에 붙였습니다 (꼭짓점 "));
}

/// An unchecked cadastral row gives no line: the drawn area stays as it is, with the C++ words.
#[test]
fn an_unchecked_cadastral_map_gives_no_line_to_snap_to() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = new_survey(dir.path());
    with_received_cadastral_map(&mut app, dir.path());
    draw_area_along_the_parcel(&mut app);
    let drawn = outer_ring(&app);
    let row = app.left_layers.entries.iter().find(|e| e.layer_key == crate::ref_vector::cadastral::layer::CADASTRAL_KEY).map(|e| e.label_ko.clone()).expect("row");
    app.home_map.real_survey.layer_visible.insert(row, false);

    assert_eq!(app.snap_shapes_to_nearest_lines(), 0);
    assert_eq!(outer_ring(&app), drawn);
    assert_eq!(app.status_left_message(), "이미 가장 가까운 선에 붙어 있거나 주변에 붙일 기준 선(지적·도면)이 없습니다.");
}

// EPSG:5187 fixture values follow target/cpp-ref/tests/test_parcel_copy_keys.cpp's snap trace case.
fn snap_crs_table(app: &mut StrataAppState, key: strata_types::schema::DomainLayerKey, srs_id: i32, kind: &str, wkt: &str) {
    use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};
    use strata_storage::survey_gpkg_read::SurveyFileTable;
    let mut layer = LayerEditBuffer::new(key);
    layer.features.push(FeatureRecord { fid: 1, wkt_geom: wkt.to_owned(), attributes: Default::default() });
    app.survey_tables.retain(|t| t.table != key.id());
    app.survey_tables.push(SurveyFileTable { table: key.id().to_owned(), srs_id, geometry_type: kind.to_owned(), columns: Vec::new(), baseline: layer.features.to_vec(), layer });
    app.left_layers.ensure_domain_layer(key);
    app.home_map.real_survey.layer_visible.insert(key.name_ko().to_owned(), true);
}

#[test]
fn snap_crs_unknown_reference_lines_and_points_never_become_work_coordinates() {
    use strata_types::schema::DomainLayerKey::{ControlPoints, FeatureLine};
    for srs in [-1, 0, 999_999] {
        let dir = tempfile::tempdir().expect("temporary survey");
        let mut app = new_survey(dir.path());
        snap_crs_table(&mut app, FeatureLine, srs, "LINESTRING", "LINESTRING (200000 450000,200020 450000)");
        snap_crs_table(&mut app, ControlPoints, srs, "POINT", "POINT (200010 450001.2)");
        let (lines, points) = app.survey_snap_parts();
        assert!(lines.is_empty() && points.is_empty(), "unknown EPSG:{srs} leaked raw reference coordinates: {lines:?}, {points:?}");
    }
}

#[test]
fn snap_crs_unknown_target_does_not_change_geometry_history_or_dirty_state() {
    for srs in [-1, 0, 999_999] {
        let dir = tempfile::tempdir().expect("temporary survey");
        let mut app = new_survey(dir.path());
        app.home_map.viewport.meters_per_pixel = 0.125;
        draw_area_along_the_parcel(&mut app);
        assert_eq!(outer_ring(&app).len(), 5);
        app.home_map.real_survey.cadastral_parcels.push(RealCadastralParcel {
            jibun: "도면".to_owned(),
            ring: vec![[200_000.0, 450_000.0], [200_010.0, 450_001.2], [200_020.0, 450_000.0]],
            center: [200_010.0, 450_000.0],
        });
        app.home_map.viewport.meters_per_pixel = 0.125;
        let area = app.survey_tables.iter_mut().find(|t| t.table == "survey_area").expect("area");
        area.srs_id = srs;
        let before = serde_json::to_value(&area.layer).expect("buffer including history");
        let undo_count = app.undo_tables.len();
        app.is_dirty = false;
        assert_eq!(app.snap_shapes_to_nearest_lines(), 0, "unknown EPSG:{srs} target was edited");
        let area = app.survey_tables.iter().find(|t| t.table == "survey_area").expect("area");
        assert_eq!(serde_json::to_value(&area.layer).expect("buffer"), before);
        assert_eq!(app.undo_tables.len(), undo_count);
        assert!(!app.is_dirty);
    }
}

#[test]
fn snap_crs_invalid_reference_cannot_move_a_valid_target() {
    for srs in [-1, 0, 999_999] {
        let dir = tempfile::tempdir().expect("temporary survey");
        let mut app = new_survey(dir.path());
        app.home_map.viewport.meters_per_pixel = 0.125;
        draw_area_along_the_parcel(&mut app);
        let drawn = outer_ring(&app);
        snap_crs_table(&mut app, strata_types::schema::DomainLayerKey::FeatureLine, srs, "LINESTRING", "LINESTRING (200000 450000,200010 450001.2,200020 450000)");
        assert_eq!(app.snap_shapes_to_nearest_lines(), 0, "unknown EPSG:{srs} reference moved the valid EPSG:5187 target");
        assert_eq!(outer_ring(&app), drawn);
    }
}

// Conversion boundary: target/cpp-ref/src/core/SnapTraceOps.cpp:114-125; independent GDAL literals in strata-engine/src/crs_tests.rs:82-83.
// A 1 mm fixture comparison is 0.008 screen px at 0.125 m/px, not a global projection-error bound.
#[test]
fn snap_crs_valid_projected_lines_and_identity_points_keep_their_coordinates() {
    use strata_types::schema::DomainLayerKey::{ControlPoints, FeatureLine};
    let dir = tempfile::tempdir().expect("temporary survey");
    let mut app = new_survey(dir.path());
    snap_crs_table(&mut app, FeatureLine, 5186, "LINESTRING", "LINESTRING (200050 450040,200000 450000)");
    snap_crs_table(&mut app, ControlPoints, 5187, "POINT", "POINT (21200 451902)");
    let (lines, points) = app.survey_snap_parts();
    assert_eq!(points, [[21200.0, 451902.0]]);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 2);
    for (got, want) in lines[0].iter().zip([[21200.748364012, 451902.649385984], [21149.9056088904, 451863.684731702]]) {
        assert!((got[0] - want[0]).hypot(got[1] - want[1]) <= 0.001, "EPSG:5186 -> 5187 mismatch: {got:?} vs {want:?}");
    }
}
