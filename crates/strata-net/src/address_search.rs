use serde_json::Value;
use strata_types::error::{StrataError, StrataResult};

#[derive(Debug, Clone, PartialEq)]
pub struct AddressSearchHit {
    pub title: String,
    pub road_or_parcel: String,
    pub lon: f64,
    pub lat: f64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vworld_address_search_parser_extracts_parcel_coords_and_sido_presets_exist() -> StrataResult<()>
    {
        assert!(!sido_presets().is_empty());
        let sample = r#"{
            "response": {
                "status": "OK",
                "result": {
                    "items": [
                        {
                            "title": "경상북도 경주시 인왕동 76",
                            "address": { "parcel": "경상북도 경주시 인왕동 76" },
                            "point": { "x": "129.2247", "y": "35.8347" }
                        }
                    ]
                }
            }
        }"#;
        let hits = parse_vworld_search_json(sample.as_bytes())?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "경상북도 경주시 인왕동 76");
        assert!((hits[0].lon - 129.2247).abs() < 1e-4);
        assert!((hits[0].lat - 35.8347).abs() < 1e-4);
        Ok(())
    }
}
