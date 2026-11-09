//! 타일 디스크·메모리 캐시 상한과 타일 인덱스 규칙 (C++ `TileFetcher.cpp`, `KaTileDiskCache`).
//!
//! `tile_fetcher` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use std::collections::VecDeque;

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
