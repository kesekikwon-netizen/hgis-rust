//! 영문 지번 질의 해석·시도 프리셋·VWorld 검색 URL/JSON.
//!
//! `address_search` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use serde_json::Value;
use strata_types::error::StrataResult;
use strata_types::error::StrataError;

use super::*;

/// C++ `KaRegionLocator.h` 지오코딩 오류 코드 (`GeocodeError`) 1:1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeocodeError {
    NetworkFailure,
    NotFound,
}

/// C++ `KaRegionLocator::parseAddressOrCadastral` 행정구역·지번 정규 파싱 결과 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCadastralQuery {
    pub sido: String,
    pub sigungu: String,
    pub eup_myeon_dong: String,
    pub ri_or_detail: String,
    pub jibun: String,
}

/// C++ `KaRegionLocator` 지번 검색 실패 시 안내 팝업 문구 1:1
#[must_use]
pub const fn format_geocode_error_ko(err: GeocodeError) -> &'static str {
    match err {
        GeocodeError::NotFound => {
            "해당 지번을 찾을 수 없습니다. 지번 표기를 확인하거나 지도에서 직접 위치를 클릭하세요."
        }
        GeocodeError::NetworkFailure => {
            "주소 검색 서버와 통신할 수 없습니다. 네트워크 연결을 확인하거나 지도에서 직접 위치를 클릭하세요."
        }
    }
}

/// C++ `KaRegionLocator` 행정구역·지번 문자열 파서 (`"충남 부여군 부여읍 쌍북리 123"` 등) 1:1
pub fn parse_cadastral_query_ko(query: &str) -> Option<ParsedCadastralQuery> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.len() < 4 {
        return None;
    }
    let sido = tokens[0].to_owned();
    let sigungu = tokens[1].to_owned();
    let eup_myeon_dong = tokens[2].to_owned();
    let (ri_or_detail, jibun) = if tokens.len() >= 5 {
        (tokens[3].to_owned(), tokens[4..].join(" "))
    } else {
        (String::new(), tokens[3].to_owned())
    };
    let has_digit = jibun.chars().any(|c| c.is_ascii_digit());
    if !has_digit {
        return None;
    }
    Some(ParsedCadastralQuery {
        sido,
        sigungu,
        eup_myeon_dong,
        ri_or_detail,
        jibun,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidoPreset {
    pub name_ko: &'static str,
    pub center_5187: [f64; 2],
}

pub const SIDO_PRESETS: [SidoPreset; 10] = [
    SidoPreset {
        name_ko: "경북 경주(월성·황룡사)",
        center_5187: [219_850.0, 360_120.0],
    },
    SidoPreset {
        name_ko: "서울(풍납토성·몽촌토성)",
        center_5187: [32_150.0, 547_300.0],
    },
    SidoPreset {
        name_ko: "충남 부여(관북리·부소산성)",
        center_5187: [-9_420.0, 408_950.0],
    },
    SidoPreset {
        name_ko: "충남 공주(공산성·송산리)",
        center_5187: [10_680.0, 428_510.0],
    },
    SidoPreset {
        name_ko: "전북 익산(왕궁리·미륵사지)",
        center_5187: [-5_120.0, 375_240.0],
    },
    SidoPreset {
        name_ko: "경남 김해(봉황동·대성동)",
        center_5187: [189_240.0, 293_110.0],
    },
    SidoPreset {
        name_ko: "경남 함안(말이산 고분군)",
        center_5187: [146_720.0, 297_850.0],
    },
    SidoPreset {
        name_ko: "대구·경산(임당동·조영동)",
        center_5187: [176_850.0, 359_400.0],
    },
    SidoPreset {
        name_ko: "전남 나주(복암리·반남)",
        center_5187: [-25_840.0, 267_900.0],
    },
    SidoPreset {
        name_ko: "강원 춘천(중도 유적)",
        center_5187: [81_400.0, 588_100.0],
    },
];

#[must_use]
pub const fn sido_presets() -> &'static [SidoPreset] {
    &SIDO_PRESETS
}

#[must_use]
pub fn build_vworld_search_url(query: &str, api_key: &str) -> String {
    format!(
        "https://api.vworld.kr/req/search?service=search&request=search&version=2.0\
         &crs=EPSG:4326&size=10&page=1&query={query}&type=address&category=parcel\
         &format=json&errorformat=json&key={api_key}"
    )
}

pub fn parse_vworld_search_json(json_bytes: &[u8]) -> StrataResult<Vec<AddressSearchHit>> {
    let root: Value = serde_json::from_slice(json_bytes)
        .map_err(|e| StrataError::Network(format!("VWorld 주소 검색 JSON 파싱 실패: {e}")))?;
    let status = root
        .pointer("/response/status")
        .and_then(Value::as_str)
        .unwrap_or("ERROR");
    if status != "OK" {
        return Ok(Vec::new());
    }
    let Some(items) = root
        .pointer("/response/result/items")
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::with_capacity(items.len());
    for item in items {
        let lon = item
            .pointer("/point/x")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok());
        let lat = item
            .pointer("/point/y")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok());
        let (Some(lon), Some(lat)) = (lon, lat) else {
            continue;
        };
        let parcel = item
            .pointer("/address/parcel")
            .and_then(Value::as_str)
            .unwrap_or("");
        let road = item
            .pointer("/address/road")
            .and_then(Value::as_str)
            .unwrap_or(parcel);
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(parcel)
            .to_owned();
        hits.push(AddressSearchHit {
            title,
            road_or_parcel: if !parcel.is_empty() {
                parcel.to_owned()
            } else {
                road.to_owned()
            },
            lon,
            lat,
        });
    }
    Ok(hits)
}
