//! `tile_fetcher` 시험 — 캐시·타일 인덱스·배경지도·고지도 레이어 명세.
//!
//! 원래 `tile_fetcher.rs` 안 `mod tests` 를 그대로 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn tile_dispatcher_synthesizes_backdrop_and_builds_heritage_webview_plan() {
    let rgba = AsyncTileDispatcher::synthesize_basemap_backdrop_rgba(
        BasemapSource::VworldSatellite,
        128,
        128,
    );
    assert_eq!(rgba.len(), 128 * 128 * 4);

    let plan = AsyncTileDispatcher::build_heritage_webview_plan(
        "archaeo_user",
        "secret_pw",
        "경상북도",
        "경주시",
    );
    assert_eq!(plan.portal_url, "https://intranet.gis-heritage.go.kr/");
    assert!(plan.injected_scripts.len() >= 3);
}

#[test]
fn test_tile_cache_lru_eviction_capacity_5_evicts_1_to_5_and_retains_6_to_10() {
    let mut cache = TileLruCache::new(5);
    for idx in 1..=10 {
        cache.insert(format!("tile_{idx}"), vec![idx as u8]);
    }
    assert_eq!(cache.len(), 5);
    assert_eq!(
        cache.keys(),
        vec!["tile_6", "tile_7", "tile_8", "tile_9", "tile_10"]
    );
    assert!(cache.get("tile_1").is_none());
    assert_eq!(cache.get("tile_6"), Some(vec![6]));
}

#[test]
fn test_gyeongju_cheomseongdae_tile_index_28145_12886_and_exponential_backoff_1000_2000_4000() {
    let (tx, ty) = lon_lat_to_tile_index(129.2190, 35.8347, 15);
    assert_eq!((tx, ty), (28145, 12886));

    assert_eq!(compute_tile_exponential_backoff_ms(0), Some(1000));
    assert_eq!(compute_tile_exponential_backoff_ms(1), Some(2000));
    assert_eq!(compute_tile_exponential_backoff_ms(2), Some(4000));
    assert_eq!(compute_tile_exponential_backoff_ms(3), None);
}

#[test]
fn test_reject_html_response_as_tile_and_prune_512mb_to_400mb() {
    let html_err = b"<HTML><HEAD><TITLE>Service Error</TITLE></HEAD><BODY></BODY></HTML>";
    assert!(!verify_tile_image_magic(html_err));
    let mut valid_png = Vec::from(HGIS_PNG_MAGIC_BYTES);
    valid_png.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    assert!(verify_tile_image_magic(&valid_png));
    assert_eq!(
        generate_tile_cache_key("vworld_base", 15, 28145, 12886),
        "vworld_base/15/28145/12886.png"
    );

    let chunk = 100 * 1_048_576_u64; // 100 MB each
    let mut disk_entries = vec![
        ("t1.png".to_owned(), chunk, 10),
        ("t2.png".to_owned(), chunk, 20),
        ("t3.png".to_owned(), chunk, 30),
        ("t4.png".to_owned(), chunk, 40),
        ("t5.png".to_owned(), chunk, 50),
        ("t6.png".to_owned(), chunk, 60),
    ]; // 600 MB > 512 MB
    let removed = prune_disk_cache_entries_lru(
        &mut disk_entries,
        HGIS_MAX_DISK_CACHE_BYTES,
        HGIS_PRUNE_TARGET_BYTES,
    );
    assert_eq!(removed, 2);
    let remaining_bytes: u64 = disk_entries.iter().map(|(_, sz, _)| *sz).sum();
    assert!(remaining_bytes <= HGIS_PRUNE_TARGET_BYTES);
}

#[test]
fn test_chosun_1919_tms_layer_url_building_and_thematic_wms() {
    // TestHistoricalMapOps::testChosun1919TmsLayerUrlBuilding
    let spec = build_historical_tms_layer_spec(HistoricalMapType::Chosun1919_1to50000, 0.75);
    assert_eq!(spec.provider_type, "wms");
    assert!((spec.opacity - 0.75).abs() < 1e-6);
    assert_eq!(spec.layer_title_ko, "1919 조선지형도 (1:50,000)");
    assert_eq!(spec.source_url, HGIS_CHOSUN1919_TMS_URL);

    let daedong = build_historical_tms_layer_spec(HistoricalMapType::Daedongyeojido, 0.80);
    assert_eq!(daedong.layer_title_ko, "대동여지도 (1861 목판본)");
    assert_eq!(daedong.source_url, HGIS_DAEDONG_TMS_URL);

    let soil = build_wms_thematic_layer_spec(ThematicLayerType::SoilDrainage, "EPSG:5187");
    assert_eq!(soil.layer_title_ko, "토양도 (배수등급)");
    assert!(soil.source_url.contains("crs=EPSG:5187"));
}
