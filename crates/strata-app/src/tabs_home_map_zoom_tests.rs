//! 「선택한 레이어로 이동」·「전체 지도 보기」 — C++ `LayerOps::zoomToLayerMax`(`BasemapOps.cpp:580-651`)·
//! `zoomToFullMax`(:822-829)·`clampCanvasToKorea`(:2510-2552)와 같은지.
//!
//! 기댓값: qgis-dev 4.3.0-Master·PROJ 9.8.1 을 화면 없이 띄워 798×598 지도 칸(작업 EPSG:5187)에서
//! demo.gpkg 사본의 표마다 zoomToLayerMax 를 옮겨 적은 순서(transformBoundingBox → 80 m 채움 →
//! ×1.15 → setExtent → zoomToFeatureExtent)로 돌린 뒤의 canvas.extent(). 옮겨 적은 것은 이 작업자다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_types::crs::WorkCrs;

use crate::tabs_home_map::HomeMapTabState;

fn demo_tables() -> Vec<strata_storage::survey_gpkg_read::SurveyFileTable> {
    let dir = tempfile::tempdir().expect("temp dir");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let copy = dir.path().join("demo.gpkg");
    std::fs::copy(src, &copy).expect("copy");
    strata_storage::survey_gpkg_read::read_survey_tables(&copy).expect("read")
}

fn map_798x598() -> HomeMapTabState {
    let mut m = HomeMapTabState::default();
    m.viewport.screen_size_px = [798.0, 598.0];
    m
}

fn extent(m: &HomeMapTabState) -> [f64; 4] {
    m.viewport.world_bbox()
}

fn assert_close(got: [f64; 4], want: [f64; 4], tol: f64) {
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() <= tol, "got {got:?} want {want:?}");
    }
}

/// 면·선·점 표: 표 좌표계(5186) 범위를 작업 좌표계(5187)로 옮긴 바깥 상자, 짧은 변은 80 m 로 채우고
/// ×1.15·×1.05 해 지도 칸에 맞춘다. 같은 PROJ(strata-engine)라 1 mm 안에서 같다.
#[test]
fn zoom_to_layer_matches_qgis_for_the_demo_tables() {
    let tables = demo_tables();
    let want = [
        ("survey_area", [21118.1512, 451840.7533, 21283.3455, 451964.5455]),
        ("feature_poly", [21114.0509, 451843.3060, 21242.9586, 451939.9060]),
        ("section_line", [21093.7555, 451829.7205, 21222.6632, 451926.3205]),
        ("control_points", [21110.6647, 451824.8653, 21239.5724, 451921.4653]),
    ];
    for (table, w) in want {
        let t = tables.iter().find(|t| t.table == table).expect("table");
        let ext = crate::survey_paint_layers::table_extent_in(t, WorkCrs::Epsg5187).expect("extent");
        let mut m = map_798x598();
        assert!(m.zoom_to_layer_max(Some(ext), false, WorkCrs::Epsg5187));
        assert_close(extent(&m), w, 0.001);
    }
}

/// 배경지도(래스터, 범위가 세계 전체)는 1:50,000 로 — 지금 보기가 동네 축척이면 그 가운데에서.
#[test]
fn zoom_to_a_basemap_layer_goes_to_one_to_fifty_thousand() {
    let mut m = map_798x598();
    m.viewport.center_xy = [21_200.0, 451_900.0];
    m.viewport.meters_per_pixel = 1.0;
    assert!(m.zoom_to_layer_max(None, true, WorkCrs::Epsg5187));
    assert!((m.viewport.meters_per_pixel - 50_000.0 * 0.0254 / 96.0).abs() < 1e-9);
    assert_eq!(m.viewport.center_xy, [21_200.0, 451_900.0]);
    // 범위가 없는 벡터 레이어는 아무 것도 하지 않는다(C++ 은 false).
    assert!(!m.zoom_to_layer_max(None, false, WorkCrs::Epsg5187));
}

/// 「전체 지도 보기」 = 한반도 범위(zoomToKorea) 뒤 clampCanvasToKorea — 그 범위 안이라 바뀌지 않는다.
#[test]
fn full_view_is_the_korea_view() {
    let mut a = map_798x598();
    a.zoom_to_full_max(WorkCrs::Epsg5187);
    let mut b = map_798x598();
    b.zoom_to_korea(WorkCrs::Epsg5187);
    assert_eq!(a.viewport.center_xy, b.viewport.center_xy);
    assert_eq!(a.viewport.meters_per_pixel, b.viewport.meters_per_pixel);
}

/// clampCanvasToKorea — 한반도보다 넓으면 줄이고, 가운데는 한반도 범위 안으로 끌어온다.
#[test]
fn clamp_keeps_the_view_inside_korea() {
    let kr = super::super::tabs_home_map_korea::korea_fill_extent(WorkCrs::Epsg5187).expect("korea");
    let mut m = map_798x598();
    m.viewport.center_xy = [-5_000_000.0, 9_000_000.0];
    m.viewport.meters_per_pixel = 50_000.0;
    m.clamp_to_korea(WorkCrs::Epsg5187);
    let [x0, y0, x1, y1] = extent(&m);
    assert!(x1 - x0 <= kr[2] - kr[0] + 1e-6 && y1 - y0 <= kr[3] - kr[1] + 1e-6, "zoomed in to fit Korea");
    let c = m.viewport.center_xy;
    assert!(c[0] >= kr[0] && c[0] <= kr[2] && c[1] >= kr[1] && c[1] <= kr[3], "centre pulled into Korea");
    // 한반도 안의 동네 보기는 그대로다.
    let mut n = map_798x598();
    n.viewport.center_xy = [21_200.0, 451_900.0];
    n.viewport.meters_per_pixel = 1.0;
    n.clamp_to_korea(WorkCrs::Epsg5187);
    assert_eq!((n.viewport.center_xy, n.viewport.meters_per_pixel), ([21_200.0, 451_900.0], 1.0));
}
