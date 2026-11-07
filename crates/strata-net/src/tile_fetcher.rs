use std::collections::VecDeque;

use crate::heritage_intranet::HeritageIntranetFlow;
use crate::ngii_topographic::NgiiTopographicClient;
use crate::vworld_client::VworldClient;

/// C++ `TileFetcher.cpp` 동시 HTTP 요청 상한 (`maxConcurrentRequests = 6`)
pub const MAX_CONCURRENT_TILE_REQUESTS: usize = 6;

pub const HGIS_WEB_MERCATOR_MAX_LAT: f64 = 85.051_128_78;
pub const HGIS_TILE_MEM_CACHE_MAX_COUNT: usize = 256;
pub const HGIS_TILE_DISK_CACHE_MAX_BYTES: u64 = 524_288_000;
pub const HGIS_PNG_MAGIC_BYTES: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
pub const HGIS_JPEG_MAGIC_BYTES: [u8; 3] = [0xFF, 0xD8, 0xFF];
pub const HGIS_MAX_DISK_CACHE_BYTES: u64 = 536_870_912;
pub const HGIS_PRUNE_TARGET_BYTES: u64 = 419_430_400;
pub const HGIS_TILE_MAX_RETRIES: u32 = 3;
pub const HGIS_TILE_BACKOFF_BASE_MS: u64 = 1000;

/// C++ `KaTileDiskCache::generateTileKey` — `/{sourceId}/{z}/{x}/{y}.png` 상대 경로 키 생성
#[must_use]
pub fn generate_tile_cache_key(source_id: &str, z: u8, x: u32, y: u32) -> String {
    format!("{source_id}/{z}/{x}/{y}.png")
}

/// C++ `KaTileDiskCache::verifyTileImageMagic` — PNG 8바이트 매직(`\x89PNG\r\n\x1a\n`) 또는 JPEG 3바이트 매직(`\xFF\xD8\xFF`) 검사로 HTML 에러 페이지 저장 방어
#[must_use]
pub fn verify_tile_image_magic(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    data.starts_with(&HGIS_PNG_MAGIC_BYTES) || data.starts_with(&HGIS_JPEG_MAGIC_BYTES)
}

/// C++ `KaTileDiskCache::pruneDiskCacheIfNeeded` — `HGIS_MAX_DISK_CACHE_BYTES`(512MB) 초과 시 최종 접근 시각 오름차순으로 `HGIS_PRUNE_TARGET_BYTES`(400MB, 80%) 이하까지 삭제
pub fn prune_disk_cache_entries_lru(
    entries: &mut Vec<(String, u64, u64)>,
    max_bytes: u64,
    target_bytes: u64,
) -> usize {
    let mut total_bytes: u64 = entries.iter().map(|(_, size_bytes, _)| *size_bytes).sum();
    if total_bytes <= max_bytes {
        return 0;
    }
    entries.sort_by_key(|(_, _, last_access_ts)| *last_access_ts);
    let mut removed = 0_usize;
    while total_bytes > target_bytes && !entries.is_empty() {
        let (_, sz, _) = entries.remove(0);
        total_bytes = total_bytes.saturating_sub(sz);
        removed += 1;
    }
    removed
}

/// C++ `KaTileFetcher::lonLatToTileIndex` — 위도 `[-85.05112878°, +85.05112878°]` 클램핑 및 Web Mercator 슬리피 타일 인덱스 `(tx, ty)` 산출
#[must_use]
pub fn lon_lat_to_tile_index(lon_deg: f64, lat_deg: f64, zoom: u8) -> (u32, u32) {
    let z = zoom.min(19);
    let n = f64::from(1_u32 << z);
    let clamped_lat = lat_deg.clamp(-HGIS_WEB_MERCATOR_MAX_LAT, HGIS_WEB_MERCATOR_MAX_LAT);
    let lat_rad = clamped_lat.to_radians();
    let tx = (((lon_deg + 180.0) / 360.0) * n)
        .floor()
        .clamp(0.0, n - 1.0) as u32;
    let ty_f =
        ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0) * n;
    let ty = ty_f.floor().clamp(0.0, n - 1.0) as u32;
    (tx, ty)
}

/// C++ `KaTileFetcher` HTTP 429/503 지수 백오프 재시도 대기 시간(`1000ms -> 2000ms -> 4000ms`, 최대 3회)
#[must_use]
pub const fn compute_tile_exponential_backoff_ms(retry_attempt: u32) -> Option<u64> {
    if retry_attempt >= HGIS_TILE_MAX_RETRIES {
        return None;
    }
    Some(HGIS_TILE_BACKOFF_BASE_MS << retry_attempt)
}

/// C++ `ThreadSafeLruCache<Key, Value>` 메모리 타일 캐시 (LRU 퇴출) 1:1
#[derive(Debug, Clone)]
pub struct TileLruCache {
    capacity: usize,
    entries: VecDeque<(String, Vec<u8>)>,
}

impl TileLruCache {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::with_capacity(capacity.max(1)),
        }
    }

    pub fn insert(&mut self, key: String, tile_bytes: Vec<u8>) {
        if let Some(pos) = self.entries.iter().position(|(k, _)| *k == key) {
            self.entries.remove(pos);
        } else if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((key, tile_bytes));
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        let pos = self.entries.iter().position(|(k, _)| k == key)?;
        let (k, v) = self.entries.remove(pos)?;
        let out = v.clone();
        self.entries.push_back((k, v));
        Some(out)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasemapSource {
    VworldSatellite,
    VworldBase,
    VworldCadastralWms,
    Daedongyeojido,
    History1919,
}

impl BasemapSource {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::VworldSatellite,
            Self::VworldBase,
            Self::VworldCadastralWms,
            Self::Daedongyeojido,
            Self::History1919,
        ]
    }

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::VworldSatellite => "VWorld 영상지도(위성)",
            Self::VworldBase => "VWorld 기본도",
            Self::VworldCadastralWms => "연속지적도 (VWorld WMS)",
            Self::Daedongyeojido => "대동여지도 (1861)",
            Self::History1919 => "1919 근대 오만분지일 지형도",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebViewAutomationPlan {
    pub portal_url: String,
    pub referer: String,
    pub injected_scripts: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AsyncTileDispatcher {
    pub active_source: Option<BasemapSource>,
}

impl AsyncTileDispatcher {
    #[must_use]
    pub fn synthesize_basemap_backdrop_rgba(
        source: BasemapSource,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let w = width.max(16) as usize;
        let h = height.max(16) as usize;
        let mut buf = vec![0_u8; w * h * 4];
        let base_rgb: [u8; 3] = match source {
            BasemapSource::VworldSatellite => [52, 74, 58],
            BasemapSource::VworldBase => [242, 238, 229],
            BasemapSource::VworldCadastralWms => [248, 244, 236],
            BasemapSource::Daedongyeojido => [232, 218, 190],
            BasemapSource::History1919 => [224, 214, 196],
        };
        for y in 0..h {
            for x in 0..w {
                let grid_line = (x % 32 == 0) || (y % 32 == 0);
                let c = if grid_line {
                    [
                        base_rgb[0].saturating_sub(16),
                        base_rgb[1].saturating_sub(16),
                        base_rgb[2].saturating_sub(16),
                        255,
                    ]
                } else {
                    [base_rgb[0], base_rgb[1], base_rgb[2], 255]
                };
                let off = (y * w + x) * 4;
                buf[off..off + 4].copy_from_slice(&c);
            }
        }
        buf
    }

    #[must_use]
    pub fn build_heritage_webview_plan(
        username: &str,
        password: &str,
        _sido: &str,
        _sigungu: &str,
    ) -> WebViewAutomationPlan {
        WebViewAutomationPlan {
            portal_url: HeritageIntranetFlow::PORTAL_URL.to_owned(),
            referer: VworldClient::REFERER_HEADER.to_owned(),
            injected_scripts: vec![
                HeritageIntranetFlow::login_script(username, password),
                HeritageIntranetFlow::open_download_page_script().to_owned(),
                HeritageIntranetFlow::search_script(),
            ],
        }
    }

    #[must_use]
    pub fn build_ngii_webview_plan(sheet_code: &str) -> WebViewAutomationPlan {
        let _ = NgiiTopographicClient;
        WebViewAutomationPlan {
            portal_url: "https://map.ngii.go.kr/mn/mainPage.do".to_owned(),
            referer: "https://map.ngii.go.kr".to_owned(),
            injected_scripts: vec![format!("window.__ka_ngii_sheet={sheet_code:?};")],
        }
    }
}

#[cfg(test)]
mod tests {
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
}

/// 1919 조선지형도 TMS URL 템플릿 (C++ `HGIS_CHOSUN1919_TMS_URL`)
pub const HGIS_CHOSUN1919_TMS_URL: &str =
    "type=xyz&url=http://hgis.local/tiles/1919/{z}/{x}/{-y}.png";
/// 대동여지도 TMS URL 템플릿 (C++ `HGIS_DAEDONG_TMS_URL`)
pub const HGIS_DAEDONG_TMS_URL: &str =
    "type=xyz&url=http://hgis.local/tiles/daedong/{z}/{x}/{-y}.png";
/// WMS 최대 연결 대기 시간 (5,000 ms, C++ `HGIS_WMS_TIMEOUT_MS`)
pub const HGIS_WMS_TIMEOUT_MS: u64 = 5000;

/// C++ `HistoricalMapType` 고지도 종류 1:1 대응
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalMapType {
    Chosun1919_1to50000,
    Daedongyeojido,
    Kwangbok1945,
    Cadastral1910Original,
}

/// C++ `ThematicLayerType` 주제도(토양·지질·수계) 종류 1:1 대응
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThematicLayerType {
    SoilDrainage,
    Geology1to50k,
    HydroNetworkWaterway,
}

/// C++ `HistoricalMapOps::loadHistoricalTileMap` 및 `SoilGeologyHydroLayerOps::loadWmsThematicLayer` 반환 레이어 명세
#[derive(Debug, Clone, PartialEq)]
pub struct RasterThematicLayerSpec {
    pub provider_type: &'static str,
    pub layer_title_ko: &'static str,
    pub source_url: String,
    pub opacity: f64,
}

/// 고지도 TMS 레이어 명세 생성 (C++ `HistoricalMapOps::loadHistoricalTileMap` 1:1 대응)
#[must_use]
pub fn build_historical_tms_layer_spec(
    map_type: HistoricalMapType,
    opacity: f64,
) -> RasterThematicLayerSpec {
    let clamped_opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        0.8
    };
    let (title, url) = match map_type {
        HistoricalMapType::Chosun1919_1to50000 => {
            ("1919 조선지형도 (1:50,000)", HGIS_CHOSUN1919_TMS_URL)
        }
        HistoricalMapType::Daedongyeojido => ("대동여지도 (1861 목판본)", HGIS_DAEDONG_TMS_URL),
        HistoricalMapType::Kwangbok1945 => (
            "1945 해방 직후 지형도",
            "type=xyz&url=http://hgis.local/tiles/1945/{z}/{x}/{-y}.png",
        ),
        HistoricalMapType::Cadastral1910Original => (
            "1910년대 토지조사사업 지적원도",
            "type=xyz&url=http://hgis.local/tiles/1910_cad/{z}/{x}/{-y}.png",
        ),
    };
    RasterThematicLayerSpec {
        provider_type: "wms",
        layer_title_ko: title,
        source_url: url.to_owned(),
        opacity: clamped_opacity,
    }
}

/// 토양·지질·수계 OGC WMS 1.3.0 레이어 명세 생성 (C++ `SoilGeologyHydroLayerOps::loadWmsThematicLayer` 1:1 대응)
#[must_use]
pub fn build_wms_thematic_layer_spec(
    layer_type: ThematicLayerType,
    crs_auth_id: &str,
) -> RasterThematicLayerSpec {
    let crs = if crs_auth_id.trim().is_empty() {
        "EPSG:5186"
    } else {
        crs_auth_id.trim()
    };
    let (title, layer_id) = match layer_type {
        ThematicLayerType::SoilDrainage => ("토양도 (배수등급)", "soil_drainage"),
        ThematicLayerType::Geology1to50k => ("수치지질도 (1:50,000)", "kigam_geology_50k"),
        ThematicLayerType::HydroNetworkWaterway => ("국가하천·수계망", "wamis_hydro_network"),
    };
    RasterThematicLayerSpec {
        provider_type: "wms",
        layer_title_ko: title,
        source_url: format!(
            "contextualWMSLegend=0&crs={crs}&format=image/png&layers={layer_id}&styles=default&transparent=true&version=1.3.0"
        ),
        opacity: 0.75,
    }
}
