use std::collections::VecDeque;

use crate::heritage_intranet::HeritageIntranetFlow;
use crate::ngii_topographic::NgiiTopographicClient;
use crate::vworld_client::VworldClient;

/// C++ `TileFetcher.cpp` 동시 HTTP 요청 상한 (`maxConcurrentRequests = 6`)
pub const MAX_CONCURRENT_TILE_REQUESTS: usize = 6;

pub const HGIS_WEB_MERCATOR_MAX_LAT: f64 = 85.051_128_78;
pub const HGIS_TILE_MEM_CACHE_MAX_COUNT: usize = 256;
pub const HGIS_TILE_DISK_CACHE_MAX_BYTES: u64 = 524_288_000;
pub const HGIS_TILE_MAX_RETRIES: u32 = 3;
pub const HGIS_TILE_BACKOFF_BASE_MS: u64 = 1000;

/// C++ `KaTileFetcher::lonLatToTileIndex` — 위도 `[-85.05112878°, +85.05112878°]` 클램핑 및 Web Mercator 슬리피 타일 인덱스 `(tx, ty)` 산출
#[must_use]
pub fn lon_lat_to_tile_index(lon_deg: f64, lat_deg: f64, zoom: u8) -> (u32, u32) {
    let z = zoom.min(19);
    let n = f64::from(1_u32 << z);
    let clamped_lat = lat_deg.clamp(-HGIS_WEB_MERCATOR_MAX_LAT, HGIS_WEB_MERCATOR_MAX_LAT);
    let lat_rad = clamped_lat.to_radians();
    let tx = (((lon_deg + 180.0) / 360.0) * n).floor().clamp(0.0, n - 1.0) as u32;
    let ty_f = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0) * n;
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
}
