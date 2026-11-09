//! 행정경계 조회 (C++ `fetchAdminBoundary` WFS/JSON 경로).
//!
//! `vworld_client` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use super::vworld_wms::percent_encode;
use super::vworld_wkt::geometry_to_wkt;

use super::*;
use crate::address_search::http_get_bytes;

/// C++ `AdminBoundaryService.cpp:137` — 키 인증 실패
pub const ADMIN_KEY_FAILED_MESSAGE: &str =
    "행정구역 서비스의 인증에 실패했습니다. 리본의 더보기 → API 키 입력에서 VWorld API 키를 확인한 뒤 다시 불러오세요.";

/// C++ `AdminBoundaryService.cpp:139` — 경계를 찾지 못함
pub const ADMIN_NOT_FOUND_MESSAGE: &str =
    "선택한 읍면동 경계를 찾지 못했습니다. 시·군·구와 읍·면·동을 확인한 뒤 다시 불러오세요.";

/// C++ `AdminBoundaryService.cpp:141` — 그 밖의 응답 오류
pub const ADMIN_BAD_RESPONSE_MESSAGE: &str =
    "행정구역 서버가 올바른 경계를 보내지 않았습니다. 잠시 후 다시 불러오세요.";

impl VworldClient {
    /// C++ `AdminBoundaryService.cpp:50-70` `buildGetFeatureUrl` — 읍면동 경계 GetFeature.
    /// `attrFilter` 는 `full_nm:like:시도 시군구 읍면동`. 값이 비면 파라미터를 넣지 않는다.
    #[must_use]
    pub fn admin_boundary_url(api_key: &str, sido: &str, city: &str, dong: &str) -> String {
        let filter = Self::attr_filter(sido, city, dong);
        let mut url = format!(
            "https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature\
             &format=json&size=10&page=1&geometry=true&attribute=true&crs=EPSG:3857\
             &data={}",
            Self::ADMIN_BOUNDARY_DATASET
        );
        if !filter.is_empty() {
            url.push_str(&format!("&attrFilter={}", percent_encode(&filter)));
        }
        url.push_str(&format!("&key={}", api_key.trim()));
        url
    }

    /// C++ `AdminBoundaryService.cpp:40-48` `attrFilter`
    #[must_use]
    pub fn attr_filter(sido: &str, city: &str, dong: &str) -> String {
        let parts: Vec<&str> = [sido, city, dong]
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            String::new()
        } else {
            format!("full_nm:like:{}", parts.join(" "))
        }
    }

    /// C++ `AdminBoundaryService.cpp:72-81` `normalizeCrsName`
    #[must_use]
    pub fn normalize_crs_name(raw: &str) -> String {
        if raw.contains("5186") {
            "EPSG:5186".to_owned()
        } else if raw.contains("5187") {
            "EPSG:5187".to_owned()
        } else if raw.contains("4326") {
            "EPSG:4326".to_owned()
        } else if raw.contains("5179") {
            "EPSG:5179".to_owned()
        } else if raw.contains("3857") || raw.contains("900913") {
            "EPSG:3857".to_owned()
        } else if raw.starts_with("EPSG:") {
            raw.to_owned()
        } else {
            "EPSG:3857".to_owned()
        }
    }

    /// C++ `AdminBoundaryService.cpp:133-142` — 오류 분류별 안내문
    #[must_use]
    pub fn admin_boundary_error(status: &str, error_code: &str) -> Option<&'static str> {
        if status == "OK" {
            return None;
        }
        if error_code.to_ascii_uppercase().contains("KEY") {
            Some(ADMIN_KEY_FAILED_MESSAGE)
        } else if status == "NOT_FOUND" {
            Some(ADMIN_NOT_FOUND_MESSAGE)
        } else {
            Some(ADMIN_BAD_RESPONSE_MESSAGE)
        }
    }

    /// C++ `AdminBoundaryService.h:26` `kDataset`
    pub const ADMIN_BOUNDARY_DATASET: &'static str = "LT_C_ADEMD_INFO";

    /// VWorld 행정구역(`LT_C_ADSIGG_INFO`) WFS 2.0 `GetFeature` URL 생성
    /// (C++ `HeritageNearbyRegions::districtsUrl` 1:1 대응)
    #[must_use]
    pub fn districts_url(api_key: &str, box_wgs84: [f64; 4]) -> String {
        format!(
            "https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature&format=json\
             &data=LT_C_ADSIGG_INFO&size=100&page=1&crs=EPSG:4326&geometry=true&attribute=true\
             &geomFilter=BOX({:.8},{:.8},{:.8},{:.8})&key={}",
            box_wgs84[0],
            box_wgs84[1],
            box_wgs84[2],
            box_wgs84[3],
            api_key.trim()
        )
    }
}

/// C++ `AdminBoundaryService.cpp:186-230` `fetchEmd` — 읍면동 경계 WKT 를 받아 온다.
/// 키가 없으면 C++ `:193` 안내문 그대로 실패한다.
pub fn fetch_admin_boundary_wkt(
    sid: &str,
    city: &str,
    dong: &str,
) -> Result<AdminBoundaryParsed, String> {
    let key = strata_secrets::vworld::VworldSettings::load_api_key();
    let key = key.trim();
    if key.is_empty() {
        return Err(ADMIN_KEY_MISSING_MESSAGE.to_owned());
    }
    if city.trim().is_empty() || dong.trim().is_empty() {
        return Err("시·군·구와 읍·면·동을 모두 고르세요".to_owned());
    }
    let bytes = http_get_bytes(&VworldClient::admin_boundary_url(key, sid, city, dong))
        .map_err(|_| ADMIN_CONNECT_MESSAGE.to_owned())?;
    let mut hint = city.trim().to_owned();
    if !sid.trim().is_empty() {
        hint = format!("{} {hint}", sid.trim());
    }
    parse_admin_boundary(&bytes, &hint)
}
/// 기존 `address_search.rs:758` 의 curl GET 을 그대로 쓰는 공개 진입점.
/// VWorld·국토정보플랫폼 공개 조회가 모두 이 한 경로를 지난다(새 크레이트 없음).
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    http_get_bytes(url).map_err(|_| ADMIN_CONNECT_MESSAGE.to_owned())
}

/// 읍면동 경계 파싱 결과(C++ `AdminBoundaryParse`)
#[derive(Debug, Clone, PartialEq)]
pub struct AdminBoundaryParsed {
    pub title: String,
    pub emd_code: String,
    pub wkt: String,
    pub crs_auth_id: String,
}

/// C++ `AdminBoundaryService.cpp:127-184` `parseGetFeature`
pub fn parse_admin_boundary(body: &[u8], city_hint: &str) -> Result<AdminBoundaryParsed, String> {
    use serde_json::Value;
    let root: Value =
        serde_json::from_slice(body).map_err(|_| ADMIN_BAD_RESPONSE_MESSAGE.to_owned())?;
    let resp = root.get("response").unwrap_or(&Value::Null);
    let status = resp.get("status").and_then(Value::as_str).unwrap_or("");
    if status != "OK" {
        let code = resp
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(VworldClient::admin_boundary_error(status, code)
            .unwrap_or(ADMIN_BAD_RESPONSE_MESSAGE)
            .to_owned());
    }
    let fc = resp
        .get("result")
        .and_then(|r| r.get("featureCollection"))
        .ok_or_else(|| ADMIN_NOT_FOUND_MESSAGE.to_owned())?;
    let crs_auth_id = VworldClient::normalize_crs_name(
        fc.get("crs")
            .and_then(|c| c.get("properties"))
            .and_then(|p| p.get("name"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    let features = fc
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| ADMIN_NOT_FOUND_MESSAGE.to_owned())?;
    let hint = city_hint.trim();
    let mut chosen = None;
    for feature in features {
        let full = feature
            .get("properties")
            .and_then(|p| p.get("full_nm"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if !hint.is_empty() && !full.contains(hint) {
            continue;
        }
        chosen = Some(feature);
        break;
    }
    let chosen = chosen
        .or_else(|| features.first())
        .ok_or_else(|| ADMIN_NOT_FOUND_MESSAGE.to_owned())?;
    let props = chosen.get("properties").unwrap_or(&Value::Null);
    let mut title = props
        .get("full_nm")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    if title.is_empty() {
        title = props
            .get("emd_kor_nm")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
    }
    let emd_code = match props.get("emd_cd") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    let wkt = geometry_to_wkt(chosen.get("geometry").unwrap_or(&Value::Null));
    if wkt.is_empty() {
        return Err("행정구역 서버가 경계 좌표를 보내지 않았습니다. 잠시 후 다시 불러오세요.".to_owned());
    }
    Ok(AdminBoundaryParsed {
        title,
        emd_code,
        wkt,
        crs_auth_id,
    })
}
/// C++ `AdminBoundaryService.cpp:193` — 키가 없을 때(행정구역 경로 문구)
pub const ADMIN_KEY_MISSING_MESSAGE: &str =
    "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣은 뒤 다시 누르세요.";

/// C++ `AdminBoundaryService.cpp:218` — 행정구역 서버 연결 실패
pub const ADMIN_CONNECT_MESSAGE: &str =
    "행정구역 서버에 연결하지 못했습니다. 인터넷 연결을 확인하고 잠시 후 다시 불러오세요.";
