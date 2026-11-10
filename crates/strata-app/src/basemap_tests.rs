//! 배경지도 타일 회귀 시험.
//!
//! C++ 은 QGIS 가 위성·지적 타일을 그려 주지만 Rust 는 직접 받아 칠해야 한다.
//! 사용자 지적(2026-11-09): 레이어 목록에는 「위성지도」가 켜져 있는데
//! 화면은 회색 단색이었다 — 타일을 그리는 코드가 아예 없었다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::basemap_fetch::{looks_like_image, tile_cache_path};
use crate::basemap_tiles::{basemap_zoom_for, lon_lat_to_tile, tile_lon_lat, tile_url, TileKind};

/// C++ `BasemapOps.cpp:1161-1175` 와 같은 URL 형식이어야 한다.
/// 위성은 WMTS `/Satellite/{z}/{y}/{x}.jpeg` — **z/y/x 순서**가 중요하다.
#[test]
fn test_vworld_tile_url_matches_cpp_form() {
    let sat = tile_url(TileKind::Satellite, "KEY123", 16, 55883, 25378);
    assert_eq!(
        sat,
        "https://api.vworld.kr/req/wmts/1.0.0/KEY123/Satellite/16/25378/55883.jpeg",
        "위성 타일 URL 이 C++ 과 달라졌다 (z/y/x 순서 주의)"
    );

    let base = tile_url(TileKind::Base, "KEY123", 16, 55883, 25378);
    assert_eq!(
        base,
        "https://api.vworld.kr/req/wmts/1.0.0/KEY123/Base/16/25378/55883.png"
    );
}

/// 키가 없으면 xdworld 공개 타일로 떨어져야 한다(C++ 과 같은 폴백).
#[test]
fn test_tile_url_falls_back_without_key() {
    let sat = tile_url(TileKind::Satellite, "", 16, 100, 200);
    assert!(
        sat.contains("xdworld.vworld.kr"),
        "키가 없으면 xdworld 폴백이어야 한다: {sat}"
    );
    assert!(sat.contains("/16/100/200.jpeg"));
}

/// 경위도 ↔ 타일 번호 왕복. 서울시청 근처.
#[test]
fn test_lon_lat_tile_roundtrip() {
    let (lon, lat) = (126.978, 37.5665);
    let (tx, ty) = lon_lat_to_tile(lon, lat, 16);
    let (lon2, lat2) = tile_lon_lat(16, tx, ty);
    // 타일 한 장은 약 0.0055도(경도)이므로 그 안에 있으면 맞다.
    assert!((lon2 - lon).abs() < 0.01, "경도 오차 큼: {lon2} vs {lon}");
    assert!((lat2 - lat).abs() < 0.01, "위도 오차 큼: {lat2} vs {lat}");
    // z=16 에서 서울은 x≈55883, y≈25378 근처다.
    assert!((55_800..55_950).contains(&tx), "타일 x 이상: {tx}");
    assert!((25_300..25_450).contains(&ty), "타일 y 이상: {ty}");
}

/// VWorld 는 키가 만료되면 HTTP 200 + XML 을 준다. 이미지가 아니면 거부해야 한다.
#[test]
fn test_xml_error_is_not_treated_as_image() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?><ExceptionReport><Exception>INCORRECT_KEY</Exception></ExceptionReport>"#;
    assert!(
        !looks_like_image(xml),
        "XML 오류를 이미지로 받아들이면 지도에 깨진 타일이 칠해진다"
    );
    // 진짜 JPEG/PNG 헤더는 통과해야 한다.
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0];
    jpeg.extend_from_slice(&[0u8; 32]);
    assert!(looks_like_image(&jpeg));
    let mut png = vec![0x89, b'P', b'N', b'G'];
    png.extend_from_slice(&[0u8; 32]);
    assert!(looks_like_image(&png));
}

/// 캐시 경로가 종류·줌·좌표별로 달라야 한다(같은 파일을 덮어쓰면 안 된다).
#[test]
fn test_cache_paths_are_distinct() {
    let a = tile_cache_path(TileKind::Satellite, 16, 100, 200);
    let b = tile_cache_path(TileKind::Satellite, 16, 100, 201);
    let c = tile_cache_path(TileKind::Satellite, 17, 100, 200);
    let d = tile_cache_path(TileKind::Base, 16, 100, 200);
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
    assert!(a.to_string_lossy().ends_with(".jpeg"));
    assert!(d.to_string_lossy().ends_with(".png"));
}

/// 줌 선택이 합리적이어야 한다 — 확대하면(작은 m/px) 더 높은 z.
#[test]
fn test_basemap_zoom_increases_when_zoomed_in() {
    let rect = eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::vec2(1000.0, 700.0),
    );
    let far = basemap_zoom_for(50.0, rect); // 넓게 보기
    let near = basemap_zoom_for(0.5, rect); // 확대
    assert!(
        near > far,
        "확대하면 줌 레벨이 커져야 한다: near={near} far={far}"
    );
    assert!((5..=19).contains(&far));
    assert!((5..=19).contains(&near));
}

/// C++ `BasemapOps.cpp:1383` — 지적선은 1:200 ~ 1:80,000 에서만 그린다.
/// 넓게 보면 필지선이 뭉개지고 타일 요청만 폭증한다.
#[test]
fn test_cadastral_scale_range_matches_cpp() {
    use crate::basemap_cadastral::{
        cadastral_target_scale, cadastral_visible_at_scale, CADASTRAL_DEFAULT_SCALE_DENOM,
    };
    assert!(cadastral_visible_at_scale(5_000.0), "1:5000 은 보여야 한다");
    assert!(cadastral_visible_at_scale(25_000.0));
    assert!(cadastral_visible_at_scale(80_000.0), "경계값은 포함");
    assert!(!cadastral_visible_at_scale(80_001.0), "1:80000 초과는 숨김");
    assert!(!cadastral_visible_at_scale(200_000.0), "넓게 보면 숨김");
    assert!(!cadastral_visible_at_scale(150.0), "1:200 미만은 숨김");
    assert!(!cadastral_visible_at_scale(f64::NAN));
    // 범위를 벗어나면 C++ 처럼 1:25,000 으로 당긴다.
    assert_eq!(
        cadastral_target_scale(200_000.0),
        Some(CADASTRAL_DEFAULT_SCALE_DENOM)
    );
    assert_eq!(CADASTRAL_DEFAULT_SCALE_DENOM, 25_000.0);
    assert_eq!(cadastral_target_scale(5_000.0), None);
}

/// 지적 격자 칸이 1km 이고, 화면 이동에도 같은 칸을 재사용해야 한다.
/// 화면 전체를 한 장으로 받으면 1px 흔들림에도 매번 새로 받아 화면이 빈다.
#[test]
fn test_cadastral_grid_cells_are_reused() {
    use crate::basemap_cadastral::{cadastral_cell_bbox, cadastral_grid_range, CADASTRAL_CELL_M};

    // 거의 같은 두 영역은 같은 칸 범위를 내야 한다.
    let a = [1_000_100.0, 2_000_100.0, 1_001_900.0, 2_001_900.0];
    let b = [1_000_140.0, 2_000_130.0, 1_001_860.0, 2_001_870.0];
    assert_eq!(cadastral_grid_range(a), cadastral_grid_range(b));

    // 칸 경계는 정확히 격자 배수여야 한다.
    let bb = cadastral_cell_bbox(12, -3);
    assert!((bb[0] - 12.0 * CADASTRAL_CELL_M).abs() < 1e-6);
    assert!((bb[1] + 3.0 * CADASTRAL_CELL_M).abs() < 1e-6);
    assert!((bb[2] - 13.0 * CADASTRAL_CELL_M).abs() < 1e-6);
    assert!((bb[3] + 2.0 * CADASTRAL_CELL_M).abs() < 1e-6);

    // 넓게 벌어진 영역은 다른 범위여야 한다.
    let far = [1_010_000.0, 2_010_000.0, 1_011_000.0, 2_011_000.0];
    assert_ne!(cadastral_grid_range(a), cadastral_grid_range(far));
}
