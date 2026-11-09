//! GeoTIFF 내보내기 한도와 파일 이름 (`MapGeoTiffExport.cpp:94-103`).
//!
//! `dialogs_download_print.rs` 가 300줄을 넘어 이 파일로 옮겼다.


/// `MapGeoTiffExport.cpp:94-103` — GeoTIFF 크기는 화면 캔버스 출력 크기 그대로다.
/// 고정 DPI 로 다시 계산하지 않고, 상한만 넘는지 본다.
pub fn geotiff_canvas_size_px(screen_size_px: [f64; 2]) -> Result<(u32, u32), &'static str> {
    let (w, h) = (screen_size_px[0], screen_size_px[1]);
    if !w.is_finite() || !h.is_finite() || w < 1.0 || h < 1.0 {
        return Err("현재 지도의 좌표계 또는 표시 범위를 확인해 주세요.");
    }
    let (wi, hi) = (w.round() as u32, h.round() as u32);
    if wi == 0
        || hi == 0
        || wi > HGIS_GEOTIFF_MAX_SIDE_PX
        || hi > HGIS_GEOTIFF_MAX_SIDE_PX
        || u64::from(wi) * u64::from(hi) > HGIS_GEOTIFF_MAX_TOTAL_PX
    {
        return Err("지도 영상은 한 변 8,192픽셀, 총 3,200만 픽셀 이내여야 합니다.");
    }
    Ok((wi, hi))
}

/// `MainWindowExport.cpp:73-74` — 기본 파일 이름 `지도_EPSG_5187.tif` (`:` 를 `_` 로 바꾼다)
#[must_use]
pub fn geotiff_default_file_name(auth_id: &str) -> String {
    format!("지도_{}.tif", auth_id.replace(':', "_"))
}

/// `KaShellUi.cpp:48-50` — 지도 탭에서만 쓸 수 있는 까닭 (GeoTIFF 버튼·비활성 사유)
pub const MAP_TAB_ONLY_REASON: &str = "「지도」 탭에서 쓸 수 있습니다. 지도 탭으로 옮긴 뒤 누르세요.";

/// `MapGeoTiffExport.cpp:101-103` — 한 변 8,192픽셀, 총 3,200만 픽셀 상한
pub const HGIS_GEOTIFF_MAX_SIDE_PX: u32 = 8192;
pub const HGIS_GEOTIFF_MAX_TOTAL_PX: u64 = 32_000_000;
