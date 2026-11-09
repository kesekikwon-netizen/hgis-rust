//! 지적 내려받기의 저장 위치·안내 문구·시군구 파싱·좌표 변환.
//!
//! C++ `MainWindowCadastral.cpp:121`, `CadastralPortal.cpp:232-258` 1:1.
//! `cadastral_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use strata_types::crs::{CrsCode, WorkCrs};
use strata_net::vworld_client;

/// C++ `CadastralPortal.cpp:232-236` — 시·군·구 목록 한 쪽에 100건
pub(crate) const CADASTRAL_CATALOG_PAGE_SIZE: u32 = 100;
/// C++ `CadastralPortal.cpp:231` — 자료 목록 최대 10쪽
pub(crate) const CADASTRAL_CATALOG_MAX_PAGES: u32 = 10;

/// 지적 내려받기 결과
#[derive(Debug, Clone, Default)]
pub struct CadastralOutcome {
    pub ok: bool,
    pub downloaded: Vec<String>,
    pub districts: Vec<String>,
    pub error: String,
}

/// C++ `MainWindowCadastral.cpp:121` — 조사 폴더 안 `지적도/원본`
#[must_use]
pub fn cadastral_raw_dir(survey_path: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    let profile = survey_path?;
    let parent = profile.parent().unwrap_or_else(|| std::path::Path::new("."));
    Some(
        parent
            .join(vworld_client::CADASTRAL_FOLDER_NAME)
            .join(vworld_client::CADASTRAL_RAW_FOLDER_NAME),
    )
}

/// C++ `CadastralPortal.cpp:312` — ZIP 크기가 자동 수신 범위를 벗어났을 때
#[must_use]
pub fn cadastral_zip_size_error_ko() -> &'static str {
    "해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다."
}

/// 지적이 실제로 하는 일을 한 줄로 알린다 (C++ `MainWindowCadastral.cpp:168` 완료 안내 참고)
#[must_use]
pub fn cadastral_success_message_ko(zip_count: usize, district_count: usize) -> String {
    format!(
        "조사 주변 5km 지적도를 추가했습니다. 시·군·구 {district_count}곳의 원본 ZIP {zip_count}개를 받았습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다."
    )
}

/// 시·군·구 목록 파싱 — `LT_C_ADSIGG_INFO` GetFeature 응답(C++ `CadastralPortal.cpp:249-258`)
pub fn parse_sigungu_features(body: &[u8]) -> Result<Vec<vworld_client::CadastralDistrict>, String> {
    let root: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| cadastral_district_error_ko().to_owned())?;
    let features = root
        .get("response")
        .and_then(|r| r.get("result"))
        .and_then(|r| r.get("featureCollection"))
        .and_then(|c| c.get("features"));
    let Some(features) = features.and_then(serde_json::Value::as_array) else {
        return Err(cadastral_district_error_ko().to_owned());
    };
    let mut out: Vec<vworld_client::CadastralDistrict> = Vec::new();
    for feature in features {
        let props = feature.get("properties").unwrap_or(&serde_json::Value::Null);
        let code = match props.get("sig_cd") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let name = props
            .get("full_nm")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned();
        // C++ `:255` — 5자리 코드와 이름이 모두 있어야 한다.
        if code.len() != 5 || name.is_empty() {
            return Err(cadastral_district_error_ko().to_owned());
        }
        if !out.iter().any(|d| d.code == code) {
            out.push(vworld_client::CadastralDistrict { code, name });
        }
    }
    if out.is_empty() {
        return Err(cadastral_district_error_ko().to_owned());
    }
    Ok(out)
}

/// C++ `CadastralPortal.cpp:242` — 시·군·구 경계를 확인하지 못했을 때
#[must_use]
pub fn cadastral_district_error_ko() -> &'static str {
    "VWorld에서 시·군·구 경계를 확인하지 못했습니다. API 키와 연결을 확인하세요."
}

/// 작업 좌표계 중심을 EPSG:5179(공식 신청 좌표계)로 옮긴다 — C++ `MainWindowDownloads.cpp:183-186`
#[must_use]
pub fn work_to_portal_xy(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 2]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    if !center[0].is_finite() || !center[1].is_finite() {
        return None;
    }
    let out = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg5179)
        .transform_xy(center[0], center[1]);
    (out[0].is_finite() && out[1].is_finite()).then_some(out)
}
