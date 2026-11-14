//! `tabs_home_map` 는 홈 지도 탭 상태와 실행취소 스택의 재수출 창구다.
//!
//! 원래 359줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::tabs_home_map::<이름>` 경로는 그대로 동작한다.

#[path = "tabs_home_map_korea.rs"]
mod tabs_home_map_korea;
#[path = "tabs_home_map_state.rs"]
mod tabs_home_map_state;
#[path = "tabs_home_map_undo.rs"]
mod tabs_home_map_undo;
#[path = "tabs_home_map_zoom.rs"]
mod tabs_home_map_zoom;

pub use tabs_home_map_state::*;
pub use tabs_home_map_undo::*;

/// Longitude of the map view extent centre in WGS84 (`EPSG:4326`), or `None` when no view
/// exists, the canvas is narrower than 40 px, the scale is outside `(0, 500_000]`, or the CRS
/// transform fails (`target/cpp-ref/src/app/MainWindowSession.cpp:131-144`).
#[must_use]
pub(crate) fn canvas_center_longitude(
    viewport: Option<&strata_canvas2d::viewport::ViewportTransform>,
    view_epsg: u32,
) -> Option<f64> {
    let viewport = viewport?;
    let width = viewport.screen_size_px[0];
    if !width.is_finite() || width < 40.0 {
        return None;
    }
    let scale = crate::app_state::StrataAppState::compute_representative_scale(
        viewport.meters_per_pixel,
        crate::app_state::StrataAppState::HGIS_STANDARD_DPI,
    )?;
    if scale <= 0.0 || scale > 500_000.0 {
        return None;
    }
    let [x0, y0, x1, y1] = viewport.world_bbox();
    let center = [(x0 + x1) * 0.5, (y0 + y1) * 0.5];
    if !center[0].is_finite() || !center[1].is_finite() {
        return None;
    }
    let to_wgs84 = strata_engine::Transformer::new(view_epsg, 4326).ok()?;
    let [lon, _lat] = to_wgs84.transform(center).ok()?;
    lon.is_finite().then_some(lon)
}

#[cfg(test)]
#[path = "tabs_home_map_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tabs_home_map_lon_tests.rs"]
mod lon_tests;
