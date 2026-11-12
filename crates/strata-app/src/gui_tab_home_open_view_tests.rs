//! 「열기」 뒤 지도 범위·상태줄·범례 모양 시험 — 실제 C++ 앱 화면(1600×861, 150%)과 코드에 맞춘다.
//!
//! 지도: C++ 은 조사 데이터만 연 뒤 applyStartupMap -> ensureStartupViewReady 가
//! `LayerOps::zoomToKorea`(`BasemapOps.cpp:2559-2591`)로 한반도 전체를 보인다. 기대값은
//! GDAL/PROJ(gdaltransform EPSG:4326 -> 5187·5186)로 구한 한반도 네 모서리에서 계산했다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::Color32;
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;
use crate::gui_layer_panel::gui_layer_panel_row_spec::layer_rows;
use crate::gui_layer_panel::{
    UX_LEGEND_ALT_ROW, UX_LEGEND_BAND_HEIGHT, UX_LEGEND_BAND_INK, UX_LEGEND_BAND_RULE,
    UX_LEGEND_ROW_HEIGHT, UX_LEGEND_SWATCH_FRAME, UX_LEGEND_SWATCH_SIZE,
};

fn opened(work_crs: WorkCrs, screen: [f64; 2]) -> (StrataAppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let path = dir.path().join("demo.gpkg");
    std::fs::copy(src, &path).expect("copy demo.gpkg");
    let mut app = StrataAppState::new_startup();
    app.work_crs = work_crs;
    app.home_map.viewport.screen_size_px = screen;
    app.open_survey_gpkg(&path);
    (app, dir)
}

fn assert_view(app: &StrataAppState, center: [f64; 2], mpp: f64) {
    let v = &app.home_map.viewport;
    assert!((v.center_xy[0] - center[0]).abs() < 1.0, "center x {:?} != {center:?}", v.center_xy);
    assert!((v.center_xy[1] - center[1]).abs() < 1.0, "center y {:?} != {center:?}", v.center_xy);
    assert!((v.meters_per_pixel - mpp).abs() < 0.01, "mpp {} != {mpp}", v.meters_per_pixel);
}

/// 5187: 한반도 안쪽 범위 x -187136.965~458062.997, y 54259.536~770815.654. 1000×600 화면은
/// 그 폭에 맞추고(645.2 m/px), 세로로 긴 600×1000 은 높이에 맞춘다(716.6 m/px). 5186 도 같은 방식.
#[test]
fn layers_only_open_shows_the_cpp_korea_view_for_the_canvas_size() {
    let (mut app, _dir) = opened(WorkCrs::Epsg5187, [1000.0, 600.0]);
    assert_view(&app, [135_463.016, 412_537.595], 645.199_962);

    // 지도 칸 크기는 첫 지도 화면에서 정해진다 — 그때 한 번 다시 맞춘다(C++ ensureStartupViewReady).
    app.home_map.viewport.screen_size_px = [600.0, 1000.0];
    app.home_map.apply_pending_korea_view(app.work_crs);
    assert_view(&app, [135_463.016, 412_537.595], 716.556_118);
    app.home_map.viewport.screen_size_px = [1000.0, 600.0];
    app.home_map.apply_pending_korea_view(app.work_crs);
    assert_view(&app, [135_463.016, 412_537.595], 716.556_118);

    let (app_5186, _dir2) = opened(WorkCrs::Epsg5186, [1000.0, 600.0]);
    assert_view(&app_5186, [307_562.666, 412_939.300], 645.218_760);
}

/// C++ 은 조사 데이터만 연 뒤 상태줄에 아무 말도 띄우지 않는다(기본 안내 「위성과 지적도를…」도 없음).
#[test]
fn layers_only_open_leaves_the_status_line_empty() {
    let (app, _dir) = opened(WorkCrs::Epsg5187, [1000.0, 600.0]);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(""));
}

/// 범례 모양은 C++ 화면을 잰 값: 줄 22·구역 띠 16(`KaLayerSectionDelegate.h:21`), 견본 테두리 상자
/// 20×14, 교차 줄 (240,238,230), 띠 윗줄 (135,134,127), 띠 제목 (137,137,137), 견본 테두리 (20,20,19).
/// 구역 띠는 따로 한 줄이 아니라 그 구역 첫 레이어 행 위에 붙는다.
#[test]
fn legend_layout_follows_the_cpp_measurements_and_bands_sit_on_the_first_row() {
    assert_eq!(UX_LEGEND_ROW_HEIGHT, 22.0);
    assert_eq!(UX_LEGEND_BAND_HEIGHT, 16.0);
    assert_eq!(UX_LEGEND_SWATCH_SIZE, [20.0, 14.0]);
    assert_eq!(UX_LEGEND_ALT_ROW, Color32::from_rgb(240, 238, 230));
    assert_eq!(UX_LEGEND_BAND_RULE, Color32::from_rgb(135, 134, 127));
    assert_eq!(UX_LEGEND_BAND_INK, Color32::from_rgb(137, 137, 137));
    assert_eq!(UX_LEGEND_SWATCH_FRAME, Color32::from_rgb(20, 20, 19));

    let (app, _dir) = opened(WorkCrs::Epsg5187, [1000.0, 600.0]);
    let c = Color32::BLACK;
    let rows = layer_rows(&app.left_layers, c, c);
    assert_eq!(rows.len(), app.left_layers.entries.len(), "no separate header rows");
    let bands: Vec<(String, &str)> = rows
        .iter()
        .filter(|r| !r.section.is_empty())
        .map(|r| (r.name.clone(), r.section))
        .collect();
    assert_eq!(bands[0], ("시굴격자".to_owned(), "조사 데이터"));
}
