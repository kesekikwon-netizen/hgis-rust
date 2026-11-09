//! DEM 타일 이름·주소·범위 계산 (C++ `tileUri` / `tileRange` / `gridAligned`).
//!
//! `dem_copernicus` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;

/// C++ `QgsRectangle` — WGS84(경위도) 범위. C++ `wgs84Extent` 인자와 같은 값.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wgs84Extent {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

impl Wgs84Extent {
    #[must_use]
    pub const fn new(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Self {
        Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        }
    }

    /// C++ `QgsRectangle::isEmpty()` — 최대값이 최소값보다 작으면 빈 범위
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.max_lon < self.min_lon || self.max_lat < self.min_lat
    }

    /// C++ `QgsRectangle::isFinite()`
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.min_lon.is_finite()
            && self.min_lat.is_finite()
            && self.max_lon.is_finite()
            && self.max_lat.is_finite()
    }

    /// C++ `DemDownloadService.cpp:127-129` 범위 검사 그대로
    #[must_use]
    pub fn is_valid_wgs84(self) -> bool {
        !self.is_empty()
            && self.is_finite()
            && self.min_lon >= -180.0
            && self.max_lon <= 180.0
            && self.min_lat >= -90.0
            && self.max_lat <= 90.0
    }
}

/// Copernicus 1°×1° 타일 한 칸 (남서쪽 모서리의 정수 도 단위)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemTile {
    pub lat: i32,
    pub lon: i32,
}

impl DemTile {
    #[must_use]
    pub const fn new(lat: i32, lon: i32) -> Self {
        Self { lat, lon }
    }

    /// C++ `tileUri()` 의 파일 이름 부분 (DemDownloadService.cpp:86-92)
    #[must_use]
    pub fn name(self) -> String {
        tile_name(self.lat, self.lon)
    }

    /// curl 로 받을 실제 https 주소 (C++ 의 `/vsicurl/` 접두사 없는 형태)
    #[must_use]
    pub fn url(self) -> String {
        tile_url(self.lat, self.lon)
    }

    /// C++ `tileUri()` 원문 (DemDownloadService.cpp:93) — GDAL 가상 파일 경로
    #[must_use]
    pub fn vsicurl_uri(self) -> String {
        format!("/vsicurl/{}", self.url())
    }
}

/// C++ `tileUri()` (DemDownloadService.cpp:85-93) 의 이름 규칙.
/// 예: lat 35, lon 129 → `Copernicus_DSM_COG_10_N35_00_E129_00_DEM`
#[must_use]
pub fn tile_name(lat: i32, lon: i32) -> String {
    let north = if lat >= 0 { 'N' } else { 'S' };
    let east = if lon >= 0 { 'E' } else { 'W' };
    format!(
        "Copernicus_DSM_COG_10_{north}{:02}_00_{east}{:03}_00_DEM",
        lat.abs(),
        lon.abs()
    )
}

/// C++ `DemDownloadService.cpp:93` 의 버킷 주소. `/vsicurl/` 접두사만 뺀 형태다.
#[must_use]
pub fn tile_url(lat: i32, lon: i32) -> String {
    let name = tile_name(lat, lon);
    format!("{COPERNICUS_DEM_BUCKET}/{name}/{name}.tif")
}

/// C++ `isMissingTile()` (DemDownloadService.cpp:96-100) — 404 만 "타일 없음"으로 본다
#[must_use]
pub fn is_missing_tile(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("http response code: 404")
        || lower.contains("http error code : 404")
        || lower.contains("404 not found")
}

/// C++ `gridAligned()` (DemDownloadService.cpp:102-105)
#[must_use]
pub fn grid_aligned(delta: f64, spacing: f64) -> bool {
    let cells = delta / spacing;
    (cells - cells.round()).abs() < 1e-5
}

/// C++ `DemDownloadService.cpp:138-142` 타일 순회 범위 (1도 격자)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemTileRange {
    pub lon_from: i32,
    pub lon_to: i32,
    pub lat_from: i32,
    pub lat_to: i32,
}

impl DemTileRange {
    /// C++ `(lonTo-lonFrom+1) * (latTo-latFrom+1)` — 음수면 빈 범위
    #[must_use]
    pub fn tile_count(self) -> i64 {
        let width = i64::from(self.lon_to) - i64::from(self.lon_from) + 1;
        let height = i64::from(self.lat_to) - i64::from(self.lat_from) + 1;
        width * height
    }
}

/// C++ `DemDownloadService.cpp:138-142` — `floor(min)` / `ceil(max) - 1`
#[must_use]
pub fn tile_range(extent: Wgs84Extent) -> DemTileRange {
    DemTileRange {
        lon_from: extent.min_lon.floor() as i32,
        lon_to: extent.max_lon.ceil() as i32 - 1,
        lat_from: extent.min_lat.floor() as i32,
        lat_to: extent.max_lat.ceil() as i32 - 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemTileListError {
    /// C++ `DemDownloadService.cpp:130`
    ExtentInvalid,
    /// C++ `DemDownloadService.cpp:143-147`
    TooWide,
}

impl DemTileListError {
    #[must_use]
    pub fn message_ko(self) -> String {
        match self {
            Self::ExtentInvalid => DEM_EXTENT_INVALID_KO.to_owned(),
            Self::TooWide => dem_too_wide_ko(),
        }
    }
}

/// 범위를 덮는 타일 목록. C++ 순회 순서 그대로 위도 바깥 · 경도 안쪽(:160-161).
///
/// C++ `DemDownloadService.cpp:143` 과 같이 1칸 이상 `kMaximumTiles` 이하여야 한다.
pub fn tiles_for_extent(extent: Wgs84Extent) -> Result<Vec<DemTile>, DemTileListError> {
    if !extent.is_valid_wgs84() {
        return Err(DemTileListError::ExtentInvalid);
    }
    let range = tile_range(extent);
    let count = range.tile_count();
    if count <= 0 || count > i64::try_from(DEM_MAXIMUM_TILES).unwrap_or(i64::MAX) {
        return Err(DemTileListError::TooWide);
    }
    let mut tiles = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
    for lat in range.lat_from..=range.lat_to {
        for lon in range.lon_from..=range.lon_to {
            tiles.push(DemTile::new(lat, lon));
        }
    }
    Ok(tiles)
}
