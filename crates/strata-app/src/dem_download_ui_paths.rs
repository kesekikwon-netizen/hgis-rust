//! DEM 내려받기의 저장 위치·범위 변환·안내 문구 (C++ `MainWindowDownloads.cpp`, `DemDownloadService.cpp` 1:1).
//!
//! `dem_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use std::path::Path;

use strata_geo::crs_transform::{tm_to_lonlat, TmParams};
use strata_net::dem_copernicus::{self, dem_target_file_name, Wgs84Extent, DEM_RASTER_FILE_NAME};
use strata_types::crs::{CrsCode, WorkCrs};

/// C++ `MainWindowDownloads.cpp:134-135` — 조사 파일 옆 `DEM/원본` 폴더에 받는다.
pub const DEM_FOLDER_NAME: &str = "DEM";
pub const DEM_RAW_FOLDER_NAME: &str = "원본";

/// C++ `MainWindowDownloads.cpp:130-131` — 조사가 없을 때 안내
pub const DEM_NO_SURVEY_KO: &str = "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다.";
/// C++ `MainWindowDownloads.cpp:137-138` — 폴더를 못 만들 때
pub const DEM_FOLDER_CREATE_FAILED_KO: &str = "조사 폴더 안에 DEM 폴더를 만들지 못했습니다.";

/// C++ `BasemapDsm.h:21` 범례 이름 (Rust 범례는 줄여 쓴다 — `gui_ribbon_bar.rs:168-171`)
pub(crate) const DEM_LEGEND_KEYS: [&str; 2] = ["DEM", "지표모델(DSM) 30m · Coperni..."];
/// C++ `MainWindowDownloads.cpp:134-135` — 조사 폴더 아래 `DEM/원본/dem.tif`.
/// 조사 파일이 없으면 `None` (C++ :129-133 은 창을 열지 않고 안내만 한다).
#[must_use]
pub fn dem_target_base_path(survey_path: Option<&Path>) -> Option<String> {
    let profile = survey_path?;
    let parent = profile.parent().unwrap_or_else(|| Path::new("."));
    let target = parent
        .join(DEM_FOLDER_NAME)
        .join(DEM_RAW_FOLDER_NAME)
        .join(DEM_RASTER_FILE_NAME);
    let text = target.to_string_lossy().into_owned();
    // C++ `DemDownloadService.cpp:133-136` 저장 위치 검사와 같은 규칙을 미리 본다.
    dem_target_file_name(&text).map(|_| text)
}

/// C++ `MainWindowDownloads.cpp:136` `QDir().mkpath(rawDir)` — 조사 폴더 안에
/// `DEM/원본` 을 만든다. 실패하면 `false`.
#[must_use]
pub fn ensure_dem_folder(survey_path: Option<&Path>) -> bool {
    let Some(profile) = survey_path else {
        return false;
    };
    let parent = profile.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent.join(DEM_FOLDER_NAME).join(DEM_RAW_FOLDER_NAME)).is_ok()
}

/// C++ `MainWindowDownloads.cpp:176-187` — 작업 좌표계 화면 범위를 EPSG:4326 으로 옮긴
/// 네 모서리의 바깥 범위. `QgsCoordinateTransform::transformBoundingBox` 와 같다.
#[must_use]
pub fn wgs84_extent_from_work(bbox: [f64; 4], work_crs: WorkCrs) -> Option<Wgs84Extent> {
    let params = TmParams::for_crs(match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    })?;
    let [min_x, min_y, max_x, max_y] = bbox;
    if ![min_x, min_y, max_x, max_y].iter().all(|v| v.is_finite()) || max_x <= min_x || max_y <= min_y
    {
        return None;
    }
    let corners = [
        tm_to_lonlat(min_x, min_y, &params),
        tm_to_lonlat(min_x, max_y, &params),
        tm_to_lonlat(max_x, min_y, &params),
        tm_to_lonlat(max_x, max_y, &params),
    ];
    let mut min_lon = f64::INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    for [lon, lat] in corners {
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
    }
    let extent = Wgs84Extent::new(min_lon, min_lat, max_lon, max_lat);
    extent.is_valid_wgs84().then_some(extent)
}

/// 완료 안내 문구. C++ `DemDownloadService.cpp:211` 의 경고와 성공 보고를 합친 형태.
#[must_use]
pub fn dem_success_message_ko(downloaded: usize, missing: usize) -> String {
    let mut message = format!("DEM {downloaded}칸을 받아 한 장으로 붙였습니다.");
    if missing > 0 {
        message.push(' ');
        message.push_str(&dem_copernicus::dem_missing_tiles_warning_ko(missing));
    }
    message
}
