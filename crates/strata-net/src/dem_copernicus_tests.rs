//! `dem_copernicus` 시험 — 타일 이름·범위·진행 상태·안내 문구.
//!
//! 원래 `dem_copernicus.rs` 안 `mod tests` 를 그대로 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::AtomicBool;

use super::*;

// C++ `DemDownloadService.cpp:85-93` tileUri 규칙
#[test]
fn test_dem_tile_name_matches_cpp_tile_uri_rule() {
    assert_eq!(
        tile_name(35, 129),
        "Copernicus_DSM_COG_10_N35_00_E129_00_DEM"
    );
    assert_eq!(
        tile_name(35, 128),
        "Copernicus_DSM_COG_10_N35_00_E128_00_DEM"
    );
    // 적도·그리니치·남반구·서경(자릿수 0 채움 포함)
    assert_eq!(tile_name(0, 0), "Copernicus_DSM_COG_10_N00_00_E000_00_DEM");
    assert_eq!(
        tile_name(-5, -73),
        "Copernicus_DSM_COG_10_S05_00_W073_00_DEM"
    );
    assert_eq!(
        tile_name(-1, -9),
        "Copernicus_DSM_COG_10_S01_00_W009_00_DEM"
    );
}

#[test]
fn test_dem_tile_url_points_at_copernicus_bucket_with_plain_https_for_curl() {
    let tile = DemTile::new(35, 129);
    assert_eq!(
        tile.url(),
        "https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N35_00_E129_00_DEM/Copernicus_DSM_COG_10_N35_00_E129_00_DEM.tif"
    );
    // C++ :93 원문은 /vsicurl/ 접두사가 붙은 GDAL 가상 경로다.
    assert_eq!(tile.vsicurl_uri(), format!("/vsicurl/{}", tile.url()));
}

// C++ `DemDownloadService.cpp:138-142` floor(min) / ceil(max)-1
#[test]
fn test_dem_tile_range_uses_floor_min_and_ceil_max_minus_one() {
    let range = tile_range(Wgs84Extent::new(128.1, 34.2, 129.9, 35.7));
    assert_eq!(
        range,
        DemTileRange {
            lon_from: 128,
            lon_to: 129,
            lat_from: 34,
            lat_to: 35,
        }
    );
    assert_eq!(range.tile_count(), 4);

    // 정수 경계에 딱 맞으면 그 칸 하나만 덮는다.
    let single = tile_range(Wgs84Extent::new(129.0, 35.0, 130.0, 36.0));
    assert_eq!(
        single,
        DemTileRange {
            lon_from: 129,
            lon_to: 129,
            lat_from: 35,
            lat_to: 35,
        }
    );
    assert_eq!(single.tile_count(), 1);

    // 폭 0 범위는 C++ 에서 칸 수 0 → 실패 경로
    let empty_range = tile_range(Wgs84Extent::new(129.0, 35.0, 129.0, 35.0));
    assert_eq!(empty_range.tile_count(), 0);
}

// C++ `DemDownloadService.cpp:160-161` 위도 바깥 · 경도 안쪽 순회
#[test]
fn test_dem_tiles_for_extent_orders_lat_outer_lon_inner() {
    let tiles = tiles_for_extent(Wgs84Extent::new(128.1, 34.2, 129.9, 35.7))
        .expect("범위 4칸은 상한 안이다");
    let names: Vec<String> = tiles.iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        vec![
            "Copernicus_DSM_COG_10_N34_00_E128_00_DEM".to_owned(),
            "Copernicus_DSM_COG_10_N34_00_E129_00_DEM".to_owned(),
            "Copernicus_DSM_COG_10_N35_00_E128_00_DEM".to_owned(),
            "Copernicus_DSM_COG_10_N35_00_E129_00_DEM".to_owned(),
        ]
    );
}

// C++ `kMaximumTiles = 24` (DemDownloadService.cpp:25, :143-147)
#[test]
fn test_dem_maximum_tiles_is_24_and_over_limit_reports_cpp_message() {
    assert_eq!(DEM_MAXIMUM_TILES, 24);
    // 5x5 = 25칸 → 거절
    assert_eq!(
        tiles_for_extent(Wgs84Extent::new(126.1, 33.1, 130.9, 37.9)),
        Err(DemTileListError::TooWide)
    );
    assert_eq!(dem_too_wide_ko(), "DEM을 받을 범위가 너무 넓습니다. 한 번에 최대 24칸까지 받을 수 있습니다. 조사구역으로 확대하고 다시 내려받으세요.");
    // 4x6 = 24칸 → 허용
    let allowed = tiles_for_extent(Wgs84Extent::new(126.1, 33.1, 129.9, 38.9))
        .expect("24칸은 상한과 같으므로 허용");
    assert_eq!(allowed.len(), 24);
}

// C++ `isMissingTile()` (DemDownloadService.cpp:96-100)
#[test]
fn test_dem_is_missing_tile_only_treats_404_as_missing() {
    assert!(is_missing_tile("HTTP response code: 404"));
    assert!(is_missing_tile("HTTP error code : 404"));
    assert!(is_missing_tile("curl: (22) 404 Not Found"));
    assert!(is_missing_tile("HTTP RESPONSE CODE: 404"));
    assert!(!is_missing_tile("HTTP response code: 500"));
    assert!(!is_missing_tile("Could not resolve host"));
    assert!(!is_missing_tile("Operation timed out after 15001 ms"));
}

// C++ `gridAligned()` (DemDownloadService.cpp:102-105)
#[test]
fn test_dem_grid_aligned_tolerates_one_part_in_hundred_thousand() {
    assert!(grid_aligned(3600.0, 1.0 / 3600.0));
    assert!(grid_aligned(1.0, 1.0 / 3600.0));
    assert!(!grid_aligned(0.000_5, 1.0 / 3600.0));
}

// C++ `DemDownloadService.cpp:127-132` 범위 검사
#[test]
fn test_dem_extent_validation_matches_cpp_limits() {
    assert!(Wgs84Extent::new(128.0, 34.0, 129.0, 35.0).is_valid_wgs84());
    assert!(!Wgs84Extent::new(-181.0, 34.0, 129.0, 35.0).is_valid_wgs84());
    assert!(!Wgs84Extent::new(128.0, 34.0, 181.0, 35.0).is_valid_wgs84());
    assert!(!Wgs84Extent::new(128.0, -91.0, 129.0, 35.0).is_valid_wgs84());
    assert!(!Wgs84Extent::new(128.0, 34.0, 129.0, 91.0).is_valid_wgs84());
    assert!(!Wgs84Extent::new(f64::NAN, 34.0, 129.0, 35.0).is_valid_wgs84());
    // 뒤집힌 범위는 C++ `QgsRectangle::isEmpty()`
    assert!(!Wgs84Extent::new(129.0, 35.0, 128.0, 34.0).is_valid_wgs84());
    assert_eq!(
        tiles_for_extent(Wgs84Extent::new(129.0, 35.0, 128.0, 34.0)),
        Err(DemTileListError::ExtentInvalid)
    );
}

// C++ `DemDownloadService.cpp:133-136` 저장 위치 검사 (`QFileInfo::fileName()`)
#[test]
fn test_dem_target_file_name_matches_qfileinfo_filename() {
    assert_eq!(
        dem_target_file_name(r"C:\조사\DEM\원본\dem.tif"),
        Some("dem.tif".to_owned())
    );
    assert_eq!(
        dem_target_file_name("/tmp/조사/dem.tif"),
        Some("dem.tif".to_owned())
    );
    assert_eq!(dem_target_file_name("dem.tif"), Some("dem.tif".to_owned()));
    assert_eq!(dem_target_file_name(r"C:\조사\DEM\원본\"), None);
    assert_eq!(dem_target_file_name("/tmp/조사/"), None);
    assert_eq!(dem_target_file_name(""), None);
    assert_eq!(dem_target_file_name("   "), None);
}

// 네트워크를 부르지 않는 실패 경로 — C++ :127-147 이 다운로드보다 먼저 끝난다.
#[test]
fn test_dem_prepare_rejects_bad_extent_and_missing_folder_without_network() {
    let bad = prepare(
        Wgs84Extent::new(200.0, 34.0, 201.0, 35.0),
        r"C:\조사\DEM\원본\dem.tif",
        &DemProgress::none(),
    );
    assert_eq!(bad.status, Some(DemStatus::Failed));
    assert_eq!(bad.error, DEM_EXTENT_INVALID_KO);
    assert!(bad.raster_uri.is_none());

    let no_storage = prepare(
        Wgs84Extent::new(129.0, 35.0, 130.0, 36.0),
        "   ",
        &DemProgress::none(),
    );
    assert_eq!(no_storage.error, DEM_NO_STORAGE_KO);

    let too_wide = prepare(
        Wgs84Extent::new(126.1, 33.1, 130.9, 37.9),
        r"C:\조사\DEM\원본\dem.tif",
        &DemProgress::none(),
    );
    assert_eq!(too_wide.error, dem_too_wide_ko());
}

// 취소는 어떤 호출도 하기 전에 끝난다 (C++ :126)
#[test]
fn test_dem_prepare_returns_cancelled_when_flag_is_already_set() {
    let cancel = AtomicBool::new(true);
    let result = prepare(
        Wgs84Extent::new(129.0, 35.0, 130.0, 36.0),
        r"C:\조사\DEM\원본\dem.tif",
        &DemProgress {
            on_progress: None,
            is_cancelled: Some(&cancel),
        },
    );
    assert_eq!(result.status, Some(DemStatus::Cancelled));
    assert_eq!(result.error, DEM_CANCELLED_KO);
}

#[test]
fn test_dem_missing_warning_matches_cpp_text() {
    assert_eq!(
        dem_missing_tiles_warning_ko(2),
        "요청 범위 중 2칸은 바다이거나 DEM이 제공되지 않아 빈 영역으로 남습니다."
    );
}
