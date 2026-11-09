//! 주소·지명 검색 결과 자료형과 법정동 자료 경로.
//!
//! `address_search` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


pub(super) const KOREA_DONGS_JSON: &str = include_str!("../../../data/korea_dongs.json");

#[derive(Debug, Clone, PartialEq)]
pub struct AddressSearchHit {
    pub title: String,
    pub road_or_parcel: String,
    pub lon: f64,
    pub lat: f64,
}

/// C++ `LocationHit` (`LocationSearch.h:12-20`) 1:1
#[derive(Debug, Clone, PartialEq)]
pub struct LocationHit {
    pub title: String,
    pub detail: String,
    pub lon: f64,
    pub lat: f64,
    /// `[west, south, east, north]` in EPSG:4326 when available
    pub bbox: Option<[f64; 4]>,
    /// 19-digit PNU only populated for exact parcel-address result
    pub pnu: String,
}

/// C++ `KoreaRegionBounds` (`KoreaRegionCatalog.h:9-14`) 1:1
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KoreaRegionBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// C++ `KoreaSido` (`KoreaRegionCatalog.h:16-20`) 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KoreaSido {
    pub name: &'static str,
    pub short_name: &'static str,
    pub cities: &'static [&'static str],
}
