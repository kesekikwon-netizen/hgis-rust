//! VWorld 검색 응답 해석 (C++ `searchVworld`).
//!
//! `address_search` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::collections::BTreeSet;
use serde_json::Value;
use super::address_parcel::parse_parcel_address;
use super::address_parcel::same_parcel;
use super::address_parcel::percent_encode;
use super::address_geocode::http_get_bytes;
use super::address_geocode::read_json_coord;
use super::address_geocode::parcel_not_found_msg;
use super::address_geocode::road_not_found_msg;
use super::address_geocode::search_nominatim_sync;

use super::*;

pub(super) fn search_vworld_sync(
    query: &str,
    parcel: bool,
    road: bool,
    key: &str,
) -> Result<Vec<LocationHit>, String> {
    let size = if parcel { "1000" } else { "12" };
    let search_type = if parcel || road { "ADDRESS" } else { "place" };
    let category_param = if parcel {
        "&category=PARCEL"
    } else if road {
        "&category=ROAD"
    } else {
        ""
    };
    let q_enc = percent_encode(query);
    let key_enc = percent_encode(key);
    let url = format!(
        "https://api.vworld.kr/req/search?service=search&request=search&version=2.0&crs=EPSG:4326&size={size}&page=1&query={q_enc}&type={search_type}{category_param}&format=json&errorformat=json&key={key_enc}"
    );
    let body = match http_get_bytes(&url) {
        Ok(b) => b,
        Err(_) => {
            if parcel || road {
                return Err(if road {
                    "도로명 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 검색하세요."
                        .to_owned()
                } else {
                    "지번 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 검색하세요."
                        .to_owned()
                });
            }
            return search_nominatim_sync(query);
        }
    };
    let root: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let status = root
        .pointer("/response/status")
        .and_then(Value::as_str)
        .unwrap_or("ERROR");
    if status != "OK" {
        if parcel || road {
            return Err(if road {
                if status == "NOT_FOUND" {
                    road_not_found_msg()
                } else {
                    "도로명 검색 서버가 정상 결과를 보내지 않았습니다. VWorld API 키와 서비스 상태를 확인하세요."
                        .to_owned()
                }
            } else if status == "NOT_FOUND" {
                parcel_not_found_msg()
            } else {
                "지번 검색 서버가 정상 결과를 보내지 않았습니다. VWorld API 키와 서비스 상태를 확인하세요."
                    .to_owned()
            });
        }
        return search_nominatim_sync(query);
    }

    let Some(items) = root
        .pointer("/response/result/items")
        .and_then(Value::as_array)
    else {
        return Err(if parcel {
            parcel_not_found_msg()
        } else if road {
            road_not_found_msg()
        } else {
            "검색한 위치를 찾지 못했습니다. 주소·지번·장소 이름을 확인한 뒤 다시 검색하세요."
                .to_owned()
        });
    };

    if parcel {
        let count = root
            .pointer("/response/record/total")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            })
            .unwrap_or(0);
        let pages = root
            .pointer("/response/page/total")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            })
            .unwrap_or(1);
        if count > items.len() as i64 || pages > 1 {
            return Err(
                "지번 후보가 너무 많아 정확한 필지를 확인하지 못했습니다. 구·법정동·리까지 입력하세요."
                    .to_owned(),
            );
        }
    }

    let requested = if parcel {
        parse_parcel_address(query)
    } else {
        None
    };
    let mut hits = Vec::new();
    let mut seen_pnus = BTreeSet::new();

    for o in items {
        let addr = o.get("address");
        let road_str = addr
            .and_then(|a| a.get("road"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let parcel_str = addr
            .and_then(|a| a.get("parcel"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let mut title = o
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if road && !road_str.is_empty() {
            title = road_str.to_owned();
        }
        if title.is_empty() {
            title = road_str.to_owned();
        }
        let mut detail = parcel_str.to_owned();
        let mut pnu = String::new();

        if parcel {
            let (Some(req), Some(res_addr)) = (&requested, parse_parcel_address(&detail)) else {
                continue;
            };
            if !same_parcel(req, &res_addr) {
                continue;
            }
            title = detail.clone();
            pnu = o.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
            if pnu.len() != 19 || !pnu.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let expected_mtn = if res_addr.mountain { "2" } else { "1" };
            let pnu_main = pnu.get(11..15).and_then(|s| s.parse::<u32>().ok());
            let pnu_sub = pnu.get(15..19).and_then(|s| s.parse::<u32>().ok());
            if pnu.get(10..11) != Some(expected_mtn)
                || pnu_main != Some(res_addr.main)
                || pnu_sub != Some(res_addr.sub)
            {
                continue;
            }
            if seen_pnus.contains(&pnu) {
                continue;
            }
        }
        if detail.is_empty() {
            detail = road_str.to_owned();
        }
        let pt = o.get("point");
        let Some(lon) = read_json_coord(pt.and_then(|p| p.get("x"))) else {
            continue;
        };
        let Some(lat) = read_json_coord(pt.and_then(|p| p.get("y"))) else {
            continue;
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            continue;
        }
        if parcel && (!(124.0..=132.0).contains(&lon) || !(33.0..=39.0).contains(&lat)) {
            continue;
        }
        if parcel {
            seen_pnus.insert(pnu.clone());
        }
        if !title.is_empty() {
            hits.push(LocationHit {
                title,
                detail,
                lon,
                lat,
                bbox: None,
                pnu,
            });
        }
    }

    if parcel && hits.len() > 1 {
        return Err(
            "같은 지번에 서로 다른 필지 후보가 있습니다. 구·법정동·리까지 정확히 입력하세요."
                .to_owned(),
        );
    }
    if hits.is_empty() {
        Err(if parcel {
            parcel_not_found_msg()
        } else if road {
            road_not_found_msg()
        } else {
            "검색한 위치를 찾지 못했습니다. 주소·지번·장소 이름을 확인한 뒤 다시 검색하세요."
                .to_owned()
        })
    } else {
        Ok(hits)
    }
}
