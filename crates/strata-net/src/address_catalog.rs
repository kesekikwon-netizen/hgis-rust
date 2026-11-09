//! 시·도/시군구/법정동 카탈로그 조회 (C++ `KoreaRegionCatalog`).
//!
//! `address_search` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::collections::BTreeMap;
use std::sync::OnceLock;
use serde_json::Value;
use super::address_types::KOREA_DONGS_JSON;

use super::*;

fn dong_table() -> &'static BTreeMap<String, Vec<String>> {
    static TABLE: OnceLock<BTreeMap<String, Vec<String>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = BTreeMap::new();
        if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(KOREA_DONGS_JSON) {
            for (k, v) in obj {
                if let Value::Array(arr) = v {
                    let mut list = Vec::new();
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            let d = s.trim();
                            if !d.is_empty() && !list.iter().any(|x| x == d) {
                                list.push(d.to_owned());
                            }
                        }
                    }
                    map.insert(k, list);
                }
            }
        }
        map
    })
}

/// C++ `KoreaRegionCatalog` (`KoreaRegionCatalog.cpp`) 1:1
#[derive(Debug, Clone, Copy)]
pub struct KoreaRegionCatalog;

impl KoreaRegionCatalog {
    #[must_use]
    pub fn all_sido() -> &'static [KoreaSido] {
        BUILTIN_KOREA_SIDO
    }

    #[must_use]
    pub fn canonical_sido(name: &str) -> String {
        let t = name.trim();
        if t.is_empty() {
            return String::new();
        }
        match t {
            "강원" | "강원도" => return "강원특별자치도".to_owned(),
            "전북" | "전라북도" | "전북도" => return "전북특별자치도".to_owned(),
            "제주" | "제주도" => return "제주특별자치도".to_owned(),
            _ => {}
        }
        for s in BUILTIN_KOREA_SIDO {
            if s.name == t || s.short_name == t {
                return s.name.to_owned();
            }
        }
        t.to_owned()
    }

    #[must_use]
    pub fn cities_of(sido_name: &str) -> Vec<String> {
        let canon = Self::canonical_sido(sido_name);
        for s in BUILTIN_KOREA_SIDO {
            if s.name == canon {
                return s.cities.iter().map(|&c| c.to_owned()).collect();
            }
        }
        Vec::new()
    }

    #[must_use]
    pub fn dongs_of(sido_name: &str, city_name: &str) -> Vec<String> {
        let city = city_name.trim();
        if city.is_empty() || city == "시·군·구" {
            return Vec::new();
        }
        let key = format!("{}|{}", Self::canonical_sido(sido_name), city);
        dong_table().get(&key).cloned().unwrap_or_default()
    }

    #[must_use]
    pub fn overview_bounds(sido: &str) -> Option<KoreaRegionBounds> {
        let b = match Self::canonical_sido(sido).as_str() {
            "서울특별시" => KoreaRegionBounds {
                west: 126.76,
                south: 37.42,
                east: 127.19,
                north: 37.71,
            },
            "부산광역시" => KoreaRegionBounds {
                west: 128.79,
                south: 35.03,
                east: 129.31,
                north: 35.39,
            },
            "대구광역시" => KoreaRegionBounds {
                west: 128.35,
                south: 35.60,
                east: 128.90,
                north: 36.33,
            },
            "인천광역시" => KoreaRegionBounds {
                west: 126.57,
                south: 37.33,
                east: 126.80,
                north: 37.64,
            },
            "광주광역시" => KoreaRegionBounds {
                west: 126.64,
                south: 35.05,
                east: 127.03,
                north: 35.26,
            },
            "대전광역시" => KoreaRegionBounds {
                west: 127.24,
                south: 36.18,
                east: 127.56,
                north: 36.51,
            },
            "울산광역시" => KoreaRegionBounds {
                west: 128.97,
                south: 35.32,
                east: 129.47,
                north: 35.73,
            },
            "세종특별자치시" => KoreaRegionBounds {
                west: 127.12,
                south: 36.40,
                east: 127.42,
                north: 36.74,
            },
            "경기도" => KoreaRegionBounds {
                west: 126.51,
                south: 36.89,
                east: 127.85,
                north: 38.29,
            },
            "강원특별자치도" => KoreaRegionBounds {
                west: 127.09,
                south: 37.02,
                east: 129.37,
                north: 38.62,
            },
            "충청북도" => KoreaRegionBounds {
                west: 127.27,
                south: 36.01,
                east: 128.66,
                north: 37.26,
            },
            "충청남도" => KoreaRegionBounds {
                west: 126.11,
                south: 35.97,
                east: 127.64,
                north: 37.07,
            },
            "전북특별자치도" => KoreaRegionBounds {
                west: 126.42,
                south: 35.29,
                east: 127.92,
                north: 36.16,
            },
            "전라남도" => KoreaRegionBounds {
                west: 126.15,
                south: 34.29,
                east: 127.79,
                north: 35.49,
            },
            "경상북도" => KoreaRegionBounds {
                west: 127.79,
                south: 35.56,
                east: 129.59,
                north: 37.15,
            },
            "경상남도" => KoreaRegionBounds {
                west: 127.57,
                south: 34.76,
                east: 129.22,
                north: 35.91,
            },
            "제주특별자치도" => KoreaRegionBounds {
                west: 126.16,
                south: 33.19,
                east: 126.95,
                north: 33.57,
            },
            _ => return None,
        };
        Some(b)
    }

    #[must_use]
    pub fn compose_address(sido: &str, city: &str, dong: &str, lot: &str) -> String {
        let mut parts = Vec::new();
        let s = Self::canonical_sido(sido);
        if !s.is_empty() {
            parts.push(s);
        }
        let c = city.trim();
        if !c.is_empty() && c != "시·군·구" {
            parts.push(c.to_owned());
        }
        let d = dong.trim();
        if !d.is_empty() && d != "동·읍·면" {
            parts.push(d.to_owned());
        }
        let l = lot.trim();
        if !l.is_empty() {
            parts.push(l.to_owned());
        }
        parts.join(" ")
    }
}
