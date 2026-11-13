//! F1 — 도면 지도의 좌표: 용지 점 ↔ 지상 좌표가 같은 변환, 가운데는 지도 화면 가운데(경주 기본점 없음).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::Pos2;

use super::*;

/// 좌표점 콜아웃은 그 점의 실제 지상 좌표(KaDrawingCoordCallout — 지도 칸 변환의 역).
#[test]
fn sheet_point_and_world_point_round_trip() {
    let mc = Pos2::new(400.0, 300.0);
    let center = [312_345.0, 451_234.0];
    let ppm = 2.5;
    let world = [312_400.0, 451_200.0];
    let px = world_to_sheet_px(mc, center, ppm, world);
    let back = sheet_px_to_world(mc, center, ppm, px);
    assert!((back[0] - world[0]).abs() < 0.5 && (back[1] - world[1]).abs() < 0.5, "{back:?}");
    assert_eq!(sheet_px_to_world(mc, center, ppm, mc), center);
}

/// KaDrawingStudio.cpp:188-195 studioMapExtent — 도면 지도는 지도 화면 범위(가운데)를 쓴다.
#[test]
fn sheet_map_centre_is_the_map_view_centre() {
    let mut app = crate::app_state::StrataAppState::new_startup();
    assert_eq!(app.sheet_map_center(), app.home_map.viewport.center_xy);
    app.home_map.viewport.center_xy = [250_000.0, 400_000.0];
    assert_eq!(app.sheet_map_center(), [250_000.0, 400_000.0]);
}
