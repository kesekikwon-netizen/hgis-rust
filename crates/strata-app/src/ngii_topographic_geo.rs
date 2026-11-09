//! 「수치지형」 도엽 번호 후보와 좌표 변환.
//!
//! `ngii_topographic_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 옮긴 함수의 이름·시그니처·동작은 그대로이고 `crate::ngii_topographic_ui::<이름>` 경로도 그대로다.

use strata_net::ngii_topographic::NgiiTopographicClient;
use strata_types::crs::{CrsCode, WorkCrs};

/// 근처 도엽 번호의 후보 — C++ `MainWindowDownloads.cpp:179-182` 는
/// `TopographicSheets::select` 결과를 쓰고, 비면 `359011` 을 넣는다.
#[must_use]
pub fn nearby_sheet_numbers(center_wgs84: [f64; 2]) -> Vec<String> {
    // 중심점 주변 아주 작은 상자를 훑어 그 점이 속한 도엽을 얻는다.
    let [lon, lat] = center_wgs84;
    if !lon.is_finite() || !lat.is_finite() {
        return vec!["359011".to_owned()];
    }
    // C++ `TopographicSheets.cpp:101-105` — 경도 124°~°132° 밖은 도엽 계산을 하지 않는다.
    if !(124.0..132.0).contains(&lon) || !(0.0..90.0).contains(&lat) {
        // C++ `MainWindowDownloads.cpp:182`
        return vec!["359011".to_owned()];
    }
    let numbers =
        NgiiTopographicClient::sheets_for_bbox_25k([lon - 1e-6, lat - 1e-6, lon + 1e-6, lat + 1e-6]);
    if numbers.is_empty() {
        // C++ `MainWindowDownloads.cpp:182`
        vec!["359011".to_owned()]
    } else {
        numbers
    }
}

/// 작업 좌표계 중심 → EPSG:5179(포털 질의 좌표계). C++ `MainWindowDownloads.cpp:183-186`.
#[must_use]
pub fn portal_center(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 2]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    if !center[0].is_finite() || !center[1].is_finite() {
        return None;
    }
    let out = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg5179)
        .transform_xy(center[0], center[1]);
    (out[0].is_finite() && out[1].is_finite()).then_some(out)
}

/// 작업 좌표계 중심 → 위경도(도엽 번호 후보 계산용)
#[must_use]
pub fn center_wgs84(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 2]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    if !center[0].is_finite() || !center[1].is_finite() {
        return None;
    }
    let out = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg4326)
        .transform_xy(center[0], center[1]);
    (out[0].is_finite() && out[1].is_finite()).then_some(out)
}
