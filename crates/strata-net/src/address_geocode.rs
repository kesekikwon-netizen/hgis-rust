//! HTTP 호출과 Nominatim/VWorld 지오코딩 (C++ `executeLocationSearch`).
//!
//! `address_search` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::process::Command;
use serde_json::Value;
use super::address_parcel::parse_parcel_address;
use super::address_parcel::percent_encode;
use super::address_vworld_search::search_vworld_sync;

use super::*;

/// curl 로 GET — `vworld_client` 의 지적도·행정구역 다운로드도 같은 방식을 쓴다.
pub(crate) fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };
    let mut cmd = Command::new(curl_bin);
    cmd.args([
        "-sS",
        "--max-time",
        "15",
        "-A",
        "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)",
        "-H",
        "Accept: application/json",
        url,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("위치 검색 호출 실패: {e}"))?;
    if !output.status.success() {
        return Err("network_error".to_owned());
    }
    Ok(output.stdout)
}

pub(super) fn read_json_coord(val: Option<&Value>) -> Option<f64> {
    let v = val?;
    let n = if let Some(f) = v.as_f64() {
        f
    } else {
        let s = v.as_str()?;
        s.parse::<f64>().ok()?
    };
    n.is_finite().then_some(n)
}

pub(super) fn parcel_not_found_msg() -> String {
    "지번 주소를 찾지 못했습니다. 시·군·구, 법정동·리, 번지를 확인하세요. 예: 고아읍 봉한리 / 산 12-3"
        .to_owned()
}

pub(super) fn road_not_found_msg() -> String {
    "도로명주소를 찾지 못했습니다. 시·군·구, 도로명, 건물번호를 확인하세요. 예: 안동시 풍천면 하회종가길 40"
        .to_owned()
}

pub(super) fn search_nominatim_sync(query: &str) -> Result<Vec<LocationHit>, String> {
    let q_enc = percent_encode(query);
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={q_enc}&format=json&addressdetails=1&limit=12&countrycodes=kr&accept-language=ko"
    );
    let body = http_get_bytes(&url).map_err(|_| {
        "위치 검색 서버에 연결하지 못했습니다. 인터넷 연결을 확인하고 잠시 후 다시 검색하세요."
            .to_owned()
    })?;
    let doc: Value = serde_json::from_slice(&body).map_err(|_| {
        "위치 검색 서버가 올바른 결과를 보내지 않았습니다. 잠시 후 다시 검색하세요.".to_owned()
    })?;
    let Some(arr) = doc.as_array() else {
        return Err(
            "위치 검색 서버가 올바른 결과를 보내지 않았습니다. 잠시 후 다시 검색하세요.".to_owned(),
        );
    };
    let mut hits = Vec::new();
    for o in arr {
        let title = o
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if title.is_empty() {
            continue;
        }
        let detail = format!(
            "{} / {}",
            o.get("type").and_then(Value::as_str).unwrap_or(""),
            o.get("class").and_then(Value::as_str).unwrap_or("")
        );
        let Some(lat) = read_json_coord(o.get("lat")) else {
            continue;
        };
        let Some(lon) = read_json_coord(o.get("lon")) else {
            continue;
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            continue;
        }
        let bbox = o
            .get("boundingbox")
            .and_then(Value::as_array)
            .and_then(|bb| {
                if bb.len() != 4 {
                    return None;
                }
                let south = read_json_coord(bb.first())?;
                let north = read_json_coord(bb.get(1))?;
                let west = read_json_coord(bb.get(2))?;
                let east = read_json_coord(bb.get(3))?;
                if south >= -90.0
                    && north <= 90.0
                    && south < north
                    && west >= -180.0
                    && east <= 180.0
                    && west < east
                {
                    Some([west, south, east, north])
                } else {
                    None
                }
            });
        hits.push(LocationHit {
            title,
            detail,
            lon,
            lat,
            bbox,
            pnu: String::new(),
        });
    }
    if hits.is_empty() {
        Err("검색 결과 없음 (주소·지번·상호를 다시 입력)".to_owned())
    } else {
        Ok(hits)
    }
}

/// C++ `LocationSearch::search` / `LocationSearch::searchParcel` (`LocationSearch.cpp:227-470`) 1:1
pub fn execute_location_search(
    query: &str,
    parcel: bool,
    vworld_api_key: &str,
) -> Result<Vec<LocationHit>, String> {
    let q_trim = query.trim();
    if q_trim.is_empty() {
        return Err("주소·지번·지역·상호를 입력하세요".to_owned());
    }
    let key = vworld_api_key.trim();
    let road = !parcel && is_road_address(q_trim);
    if road {
        if key.is_empty() {
            return Err(
                "도로명주소 검색에는 VWorld API 키가 필요합니다. VWorld API 키 설정을 확인하세요."
                    .to_owned(),
            );
        }
        return search_vworld_sync(q_trim, false, true, key);
    }
    if parcel {
        let Some(parsed) = parse_parcel_address(q_trim) else {
            return Err(
                "지번 검색에는 시·군·구와 법정동·리, 번지가 필요합니다. 읍·면은 리까지 입력하세요. 예: 고아읍 봉한리 / 산 12-3"
                    .to_owned(),
            );
        };
        if key.is_empty() {
            return Err(
                "정확한 지번 검색에는 VWorld API 키가 필요합니다. VWorld API 키 설정을 확인하세요."
                    .to_owned(),
            );
        }
        let lot_fmt = if parsed.sub > 0 {
            format!(
                "{}{}-{}",
                if parsed.mountain { "산" } else { "" },
                parsed.main,
                parsed.sub
            )
        } else {
            format!("{}{}", if parsed.mountain { "산" } else { "" }, parsed.main)
        };
        let normalized_q = format!("{} {}", parsed.area.join(" "), lot_fmt);
        return search_vworld_sync(&normalized_q, true, false, key);
    }
    if !key.is_empty() {
        if let Ok(hits) = search_vworld_sync(q_trim, false, false, key) {
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
    }
    search_nominatim_sync(q_trim)
}
