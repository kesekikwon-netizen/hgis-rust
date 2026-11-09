//! 지적 내려받기 작업자 — 키 확인 → 시·군·구 → 자료 목록 → ZIP.
//!
//! C++ `CadastralPortal.cpp:204-341` 1:1. `cadastral_download_ui.rs` 가 분할한 파일이다.
//! `run_download` 는 이 파일 밖에서 부르지 않지만, `with_page` 는 시험이 직접 본다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use strata_net::vworld_client::{self, VworldClient};
use strata_types::crs::{CrsCode, WorkCrs};

use super::cadastral_download_ui_paths::{
    cadastral_district_error_ko, cadastral_zip_size_error_ko, parse_sigungu_features,
    CadastralOutcome, CADASTRAL_CATALOG_MAX_PAGES, CADASTRAL_CATALOG_PAGE_SIZE,
};

/// 내려받기 작업자 — 키 확인 → 시·군·구 → 자료 목록 → ZIP
pub(crate) fn run_download(
    survey_dir: std::path::PathBuf,
    center_work: [f64; 2],
    work_crs: WorkCrs,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<f32>>,
) -> CadastralOutcome {
    let mut outcome = CadastralOutcome::default();
    let report = |fraction: f32, progress: &Arc<Mutex<f32>>| {
        if let Ok(mut slot) = progress.lock() {
            *slot = fraction;
        }
    };

    // C++ `CadastralPortal.cpp:214` — API 키가 없으면 여기서 멈춘다.
    let key = strata_secrets::vworld::VworldSettings::load_api_key();
    let key = key.trim().to_owned();
    if key.is_empty() {
        outcome.error = VworldClient::NO_KEY_MESSAGE.to_owned();
        return outcome;
    }
    if cancel.load(Ordering::Relaxed) {
        outcome.error = "지적도 받기를 취소했습니다.".to_owned();
        return outcome;
    }

    // C++ `:222-228` — 조사구역 + 5km 를 WGS84 사각 범위로
    let Some(work_box) = cadastral_work_box(center_work, work_crs) else {
        outcome.error = "조사 주변 5km 범위를 만들지 못했습니다.".to_owned();
        return outcome;
    };
    report(0.05, &progress);

    // C++ `:231-262` — 범위에 걸친 시·군·구 목록
    let mut districts: Vec<vworld_client::CadastralDistrict> = Vec::new();
    for page in 1..=CADASTRAL_CATALOG_MAX_PAGES {
        if cancel.load(Ordering::Relaxed) {
            outcome.error = "지적도 받기를 취소했습니다.".to_owned();
            return outcome;
        }
        let url = VworldClient::districts_url(&key, work_box);
        let url = with_page(&url, page);
        let Ok(bytes) = vworld_client::fetch_bytes(&url) else {
            outcome.error = cadastral_district_error_ko().to_owned();
            return outcome;
        };
        match parse_sigungu_features(&bytes) {
            Ok(rows) => {
                let before = districts.len();
                for row in rows {
                    if !districts.iter().any(|d| d.code == row.code) {
                        districts.push(row);
                    }
                }
                // C++ `:260` — 더 받을 쪽이 없으면 끝
                if districts.len() == before {
                    break;
                }
            }
            Err(_) if page == 1 => {
                // 첫 쪽에서 아무 시·군·구도 못 받으면 C++ 처럼 오류로 끝낸다.
                outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
                return outcome;
            }
            Err(_) => break,
        }
        report(0.05 + 0.15 * f32::from(u8::try_from(page).unwrap_or(1)) / 10.0, &progress);
    }
    if districts.is_empty() {
        outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
        return outcome;
    }
    outcome.districts = districts.iter().map(|d| d.name.clone()).collect();
    report(0.25, &progress);

    // C++ `:293-303` — 자료 목록(ZIP 이름·크기·갱신일)
    let mut resources: Vec<vworld_client::CadastralResource> = Vec::new();
    for page in 1..=CADASTRAL_CATALOG_MAX_PAGES {
        if cancel.load(Ordering::Relaxed) {
            outcome.error = "지적도 받기를 취소했습니다.".to_owned();
            return outcome;
        }
        let Ok(html) = vworld_client::fetch_cadastral_catalog_page(page, CADASTRAL_CATALOG_PAGE_SIZE)
        else {
            // 로그인 없이 받은 화면에는 ZIP 목록이 없다 — C++ `:264-292` 가 로그인을
            // 먼저 하기 때문이다. 그 사유를 C++ 문구로 알린다.
            outcome.error = VworldClient::NO_ACCOUNT_MESSAGE.to_owned();
            return outcome;
        };
        let rows = vworld_client::parse_cadastral_resources(&html);
        let short_page = rows.len() < CADASTRAL_CATALOG_PAGE_SIZE as usize;
        resources.extend(rows);
        if short_page {
            break;
        }
    }
    if resources.is_empty() {
        outcome.error = VworldClient::NO_ACCOUNT_MESSAGE.to_owned();
        return outcome;
    }
    report(0.4, &progress);

    // C++ `:304-305` — 시·군·구마다 정확히 한 ZIP
    let files = match vworld_client::select_cadastral_resources(&resources, &districts) {
        Ok(files) => files,
        Err(error) => {
            outcome.error = error;
            return outcome;
        }
    };
    // C++ `:306-307`
    if std::fs::create_dir_all(&survey_dir).is_err() {
        outcome.error = "조사 폴더에 지적도 저장 위치를 만들지 못했습니다.".to_owned();
        return outcome;
    }
    // C++ `:310-337`
    let total = files.len().max(1);
    for (index, file) in files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            outcome.error = "지적도 받기를 취소했습니다.".to_owned();
            return outcome;
        }
        if file.size_kb == 0 || file.size_kb > vworld_client::CADASTRAL_MAX_ZIP_KB {
            outcome.error = cadastral_zip_size_error_ko().to_owned();
            return outcome;
        }
        let path = survey_dir.join(vworld_client::cadastral_zip_file_name(file));
        if !path.is_file()
            && vworld_client::download_cadastral_zip(&file.file_no, &path).is_err()
        {
            outcome.error = vworld_client::CADASTRAL_NOT_ZIP_MESSAGE.to_owned();
            return outcome;
        }
        outcome.downloaded.push(path.to_string_lossy().into_owned());
        report(0.4 + 0.6 * (index + 1) as f32 / total as f32, &progress);
    }
    outcome.ok = !outcome.downloaded.is_empty();
    if !outcome.ok {
        outcome.error = VworldClient::NO_ACCOUNT_MESSAGE.to_owned();
    }
    outcome
}

/// C++ `CadastralPortal.cpp:222-227` — 중심 ± 5km 를 WGS84 사각 범위로.
/// 정확한 값은 조사구역 경계가 있어야 나오므로, 중심 기준 반경으로 잡는다.
fn cadastral_work_box(center: [f64; 2], work_crs: WorkCrs) -> Option<[f64; 4]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    let r = strata_net::ngii_topographic::NgiiTopographicClient::SCOPE_RADIUS_M;
    let box_work = vworld_client::cadastral_search_box(
        [center[0] - r, center[1] - r, center[0] + r, center[1] + r],
        0.0,
    )?;
    let to_wgs = strata_geo::crs_transform::CrsTransformer::new(src, CrsCode::Epsg4326);
    let corners = [
        to_wgs.transform_xy(box_work[0], box_work[1]),
        to_wgs.transform_xy(box_work[0], box_work[3]),
        to_wgs.transform_xy(box_work[2], box_work[1]),
        to_wgs.transform_xy(box_work[2], box_work[3]),
    ];
    let mut out = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for [lon, lat] in corners {
        if !lon.is_finite() || !lat.is_finite() {
            return None;
        }
        out[0] = out[0].min(lon);
        out[1] = out[1].min(lat);
        out[2] = out[2].max(lon);
        out[3] = out[3].max(lat);
    }
    Some(out)
}

/// C++ `CadastralPortal.cpp:235` — `QUrlQuery page` 를 붙인다.
pub(crate) fn with_page(url: &str, page: u32) -> String {
    // `districts_url` 은 `page=1` 을 이미 넣고 있다.
    url.replacen("&page=1&", &format!("&page={page}&"), 1)
}
