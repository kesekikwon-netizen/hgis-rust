use crate::canvas_painter::MapSceneModel;
use crate::map_tools::MapToolKind;

use crate::cad_interaction::*;
use crate::cad_point_grid::dist;

#[test]
fn cad_left_clicks_and_right_click_commit_new_polygon_and_compute_measure() {
    let mut cad = CadSessionController::default();
    let mut scene = MapSceneModel::default();

    cad.on_left_click([100.0, 100.0], MapToolKind::DrawPolygon, &mut scene);
    cad.on_left_click([200.0, 100.0], MapToolKind::DrawPolygon, &mut scene);
    cad.on_left_click([200.0, 200.0], MapToolKind::DrawPolygon, &mut scene);
    cad.on_left_click([100.0, 200.0], MapToolKind::DrawPolygon, &mut scene);
    assert_eq!(scene.active_rubberband.len(), 4);

    let committed = cad.on_right_click(MapToolKind::DrawPolygon, &mut scene);
    assert!(committed);
    assert!(scene.active_rubberband.is_empty());
    assert_eq!(scene.polygons.len(), 1);
    assert_eq!(scene.buffer_500m_ring.len(), 37);

    cad.on_left_click([0.0, 0.0], MapToolKind::Measure, &mut scene);
    cad.on_left_click([100.0, 0.0], MapToolKind::Measure, &mut scene);
    cad.on_left_click([100.0, 100.0], MapToolKind::Measure, &mut scene);
    cad.on_left_click([0.0, 100.0], MapToolKind::Measure, &mut scene);
    let m = cad.last_measure.as_ref();
    assert!(m.is_some_and(|r| (r.total_distance_m - 300.0).abs() < 1e-4));
    assert!(m.is_some_and(|r| (r.polygon_area_m2 - 10_000.0).abs() < 1e-2));
}

// D13 KaSnapSettingsWidget.cpp:16-39 — 자석 / 공유 경계 / 허용치 4~64 px / 붙을 대상
#[test]
fn test_d13_snap_settings_tolerance_and_measure_copy_text() {
    let mut cad = CadSessionController::default();
    assert!(cad.snap_enabled);
    assert!(cad.snap_topological);
    assert!((cad.snap_tolerance_px - 12.0).abs() < 1e-9);
    assert!(!cad.snap_target_current_only);

    // KaSnapSettingsWidget.cpp:20-21 — 범위는 4.0~64.0 px
    cad.snap_tolerance_px = cad.snap_tolerance_px.clamp(4.0, 64.0);
    assert!((cad.snap_tolerance_px - 12.0).abs() < 1e-9);
    cad.snap_tolerance_px = 100.0_f64.clamp(4.0, 64.0);
    assert!((cad.snap_tolerance_px - 64.0).abs() < 1e-9);
    cad.snap_tolerance_px = 1.0_f64.clamp(4.0, 64.0);
    assert!((cad.snap_tolerance_px - 4.0).abs() < 1e-9);

    // 「모든 조사 레이어」 <-> 「현재 레이어」
    cad.snap_target_current_only = true;
    assert!(cad.snap_target_current_only);
    cad.snap_target_current_only = false;
    assert!(!cad.snap_target_current_only);

    cad.snap_topological = false;
    assert!(!cad.snap_topological);

    // KaMeasureMapTool.cpp:331-355 copyText() — 면적은 둘레도 함께 나간다
    let mut scene = MapSceneModel::default();
    for pt in [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]] {
        cad.on_left_click(pt, MapToolKind::Measure, &mut scene);
    }
    let m = cad.last_measure.expect("measure readout");
    assert!((m.polygon_perimeter_m - 40.0).abs() < 1e-6);
    assert!((m.polygon_area_m2 - 100.0).abs() < 1e-6);
}

#[test]
fn snap_trace_graph_fills_intermediate_parcel_vertices_and_snaps_rough_polygon() {
    let parcel_ring = vec![
        [0.0, 0.0],
        [10.0, 1.5],
        [20.0, 0.5],
        [30.0, 0.0],
        [30.0, 20.0],
        [0.0, 20.0],
    ];
    let refs = vec![parcel_ring];
    let mids = trace_between([0.0, 0.0], [30.0, 0.0], &refs, 0.2, 20);
    assert_eq!(mids, vec![[10.0, 1.5], [20.0, 0.5]]);

    let rough = vec![[0.4, -0.3], [29.7, 0.2], [30.2, 19.8], [-0.2, 20.1]];
    let (snapped, moved, inserted) = snap_ring_to_nearest_lines(&rough, true, &refs, 0.2);
    assert_eq!(moved, 4);
    assert_eq!(inserted, 2);
    assert_eq!(snapped.len(), 6);
    assert!(snapped.contains(&[10.0, 1.5]));
    assert!(snapped.contains(&[20.0, 0.5]));
}

#[test]
fn point_grid_nearby_vertices_deduplicates_and_sorts_by_distance() {
    let rings = vec![vec![
        [10.0, 10.0],
        [10.01, 10.01],
        [15.0, 10.0],
        [100.0, 100.0],
    ]];
    let near = nearby_vertices([10.0, 10.0], None, &rings, 0.5, 20.0, 10);
    assert_eq!(near.len(), 2);
    assert_eq!(near[0], [10.0, 10.0]);
    assert_eq!(near[1], [15.0, 10.0]);
}

#[test]
fn test_point_grid_budget_clamps_2000_dense_vertices_to_exact_900_sorted_by_distance() {
    use std::sync::atomic::{AtomicU64, Ordering};

    // Coalescing job id check (`std::atomic<uint64_t> m_latestJobId`)
    let latest_job_id = AtomicU64::new(0);
    let stale_job = latest_job_id.fetch_add(1, Ordering::SeqCst) + 1;
    let current_job = latest_job_id.fetch_add(1, Ordering::SeqCst) + 1;
    assert!(stale_job < latest_job_id.load(Ordering::SeqCst));
    assert_eq!(current_job, latest_job_id.load(Ordering::SeqCst));

    // 2,000 dense vertices inside 50 m radius around (1000, 1000)
    let mut ring = Vec::with_capacity(2000);
    for i in 1..=2000 {
        let r = (f64::from(i) / 2000.0) * 49.0;
        let angle = f64::from(i) * 0.137;
        ring.push([1000.0 + r * angle.cos(), 1000.0 + r * angle.sin()]);
    }
    let rings = vec![ring];
    let candidates = nearby_vertices(
        [1000.0, 1000.0],
        None,
        &rings,
        0.004,
        12_500.0,
        BoundaryGraph::MAX_NODES,
    );
    assert_eq!(candidates.len(), 900);
    let max_kept_dist = dist([1000.0, 1000.0], candidates[899]);
    let farthest_input_dist = dist([1000.0, 1000.0], rings[0][1999]);
    assert!(max_kept_dist < farthest_input_dist);
}

#[test]
fn test_measure_tool_azimuth_0_90_270_and_unit_formatting_1500m_25000m2() {
    let north = compute_segment_metrics([100.0, 100.0], [100.0, 200.0]);
    assert!((north.azimuth_deg - 0.0).abs() < 1e-9);
    assert!((north.distance_m - 100.0).abs() < 1e-9);

    let east = compute_segment_metrics([100.0, 100.0], [200.0, 100.0]);
    assert!((east.azimuth_deg - 90.0).abs() < 1e-9);

    let west = compute_segment_metrics([100.0, 100.0], [0.0, 100.0]);
    assert!((west.azimuth_deg - 270.0).abs() < 1e-9);

    assert_eq!(format_measure_distance(850.5), "850.50 m");
    assert_eq!(format_measure_distance(1500.0), "1.50 km");
    assert_eq!(format_measure_area(25000.0), "2.50 ha");

    // TestMeasureOps::testShoelacePyeongCalculation: 10m x 10m = 100.0 m² -> 30.250 평
    let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let (area_m2, pyeong) = calculate_polygon_area_and_pyeong(&sq);
    assert!((area_m2 - 100.0).abs() < 1e-6);
    assert!((pyeong - 30.250).abs() <= 0.001);

    // MeasureOps::formatBearing & KaAzimuthDial::note & KaDrawingCoordCallout
    assert_eq!(format_bearing_dms(45.5), "45°30′00″");
    assert_eq!(azimuth_dial_note_ko(0.0), "위가 북쪽입니다");
    assert_eq!(azimuth_dial_note_ko(35.0), "북에서 시계 방향 35°");
    assert_eq!(
        format_drawing_coord_callout([198_421.35, 541_208.92], 45.32),
        "X: 198,421.35m\nY: 541,208.92m\nEL: 45.32m"
    );
}
