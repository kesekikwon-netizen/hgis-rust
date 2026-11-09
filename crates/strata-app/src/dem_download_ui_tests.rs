//! `dem_download_ui` 시험 — 저장 위치 규칙·화면 범위 변환·안내 문구.
//!
//! 원래 `dem_download_ui.rs` 안 `mod tests` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 는 이 파일의 부모를 가리키므로
//! `crate::dem_download_ui::*` 로 명시한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_net::dem_copernicus;
use strata_types::crs::WorkCrs;

use crate::dem_download_ui::*;

// C++ `MainWindowDownloads.cpp:134-135` — 조사 폴더 옆 `DEM/원본/dem.tif`
#[test]
fn test_dem_target_base_path_follows_cpp_dem_raw_folder_rule() {
    let survey = Path::new(r"C:\조사\경주_월성북편.gpkg");
    let target = dem_target_base_path(Some(survey)).expect("조사 파일이 있으면 경로가 나온다");
    assert!(target.ends_with(r"DEM\원본\dem.tif"), "{target}");
    assert!(target.starts_with(r"C:\조사\"), "{target}");

    // 조사가 없으면 C++ :129-133 처럼 받지 않는다.
    assert_eq!(dem_target_base_path(None), None);
}

// C++ `MainWindowDownloads.cpp:176-187` — 화면 범위를 위경도로 옮긴다
#[test]
fn test_dem_wgs84_extent_matches_gyeongju_work_area() {
    // 경주 월성북편 일대 EPSG:5187 대략 범위
    let extent = wgs84_extent_from_work(
        [219_000.0, 360_400.0, 220_500.0, 361_400.0],
        WorkCrs::Epsg5187,
    )
    .expect("5187 범위는 위경도로 바꿀 수 있다");
    assert!(extent.is_valid_wgs84());
    // 동부원점(129°E) 부근, 북위 35°대
    assert!((128.9..129.4).contains(&extent.min_lon), "{extent:?}");
    assert!((128.9..129.5).contains(&extent.max_lon), "{extent:?}");
    assert!((35.5..36.0).contains(&extent.min_lat), "{extent:?}");
    assert!((35.6..36.1).contains(&extent.max_lat), "{extent:?}");
    // 이 범위는 1~4칸이라 상한(24) 안이다.
    let tiles = dem_copernicus::tiles_for_extent(extent).expect("칸 수가 상한 안이다");
    assert!(!tiles.is_empty());
    assert!(tiles.len() <= dem_copernicus::DEM_MAXIMUM_TILES);

    // 뒤집히거나 비어 있는 범위는 변환하지 않는다.
    assert!(wgs84_extent_from_work([1.0, 2.0, 0.0, 0.0], WorkCrs::Epsg5187).is_none());
    assert!(wgs84_extent_from_work([f64::NAN, 0.0, 1.0, 1.0], WorkCrs::Epsg5186).is_none());
}

// C++ `DemDownloadService.cpp:211` — 빠진 칸이 있으면 경고를 붙인다
#[test]
fn test_dem_success_message_reports_tiles_and_missing_cells() {
    assert_eq!(
        dem_success_message_ko(4, 0),
        "DEM 4칸을 받아 한 장으로 붙였습니다."
    );
    assert_eq!(
        dem_success_message_ko(2, 2),
        "DEM 2칸을 받아 한 장으로 붙였습니다. 요청 범위 중 2칸은 바다이거나 DEM이 제공되지 않아 빈 영역으로 남습니다."
    );
}

// C++ 문구는 원문 그대로여야 한다 (`MainWindowDownloads.cpp:130-138`)
#[test]
fn test_dem_folder_messages_match_cpp_text() {
    assert_eq!(
        DEM_NO_SURVEY_KO,
        "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다."
    );
    assert_eq!(
        DEM_FOLDER_CREATE_FAILED_KO,
        "조사 폴더 안에 DEM 폴더를 만들지 못했습니다."
    );
    assert_eq!(DEM_FOLDER_NAME, "DEM");
    assert_eq!(DEM_RAW_FOLDER_NAME, "원본");
}
