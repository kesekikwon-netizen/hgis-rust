//! 지적 내려받기 작업자 — 키·계정 확인 → 시·군·구 → 로그인 → 자료 목록 → ZIP.
//!
//! C++ `CadastralPortal.cpp:204-341` 의 순서 그대로다. 예전 Rust 판은 로그인(`:264-292`)을
//! 건너뛰어 ZIP 대신 로그인 화면을 받았고, 5 km 범위 계산도 항상 실패했다.
//! `run_download` 는 이 파일 밖에서 부르지 않지만, `with_page` 는 시험이 직접 본다.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use strata_net::portal_session::{PortalSession, ZipDownloadError};
use strata_net::vworld_client::{self, VworldClient};
use strata_net::vworld_login::{vworld_portal_login, CADASTRAL_DATASET_URL};
use strata_secrets::accounts::{AccountKind, AccountSettings};
use strata_types::crs::{CrsCode, WorkCrs};

use super::cadastral_download_ui_paths::{
    cadastral_district_error_ko, cadastral_zip_size_error_ko, parse_sigungu_features,
    CadastralOutcome, CADASTRAL_CATALOG_MAX_PAGES, CADASTRAL_CATALOG_PAGE_SIZE,
};

const CANCELED: &str = "지적도 받기를 취소했습니다.";
/// C++ `CadastralPortal.cpp:216` — 아이디·비밀번호가 없을 때.
const NO_LOGIN_ACCOUNT: &str = "지적도 메뉴에서 VWorld 계정을 설정하세요.";
const NETWORK_ERROR: &str = "VWorld 통신에 실패했습니다. 인터넷 연결과 계정 설정을 확인하세요.";

/// 내려받기 작업자. `scope_work` 는 조사구역을 감싸는 범위(작업 좌표계,
/// `[min_x, min_y, max_x, max_y]`)이고, 여기에 5 km 를 더해 찾는다.
pub(crate) fn run_download(
    survey_dir: std::path::PathBuf,
    scope_work: [f64; 4],
    work_crs: WorkCrs,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<f32>>,
) -> CadastralOutcome {
    let mut outcome = CadastralOutcome::default();
    let report = |fraction: f32| {
        if let Ok(mut slot) = progress.lock() {
            *slot = fraction;
        }
    };
    let canceled = || cancel.load(Ordering::Relaxed);

    // C++ `:214` — API 키가 없으면 여기서 멈춘다.
    let key = strata_secrets::vworld::VworldSettings::load_api_key();
    let key = key.trim().to_owned();
    if key.is_empty() {
        outcome.error = VworldClient::NO_KEY_MESSAGE.to_owned();
        return outcome;
    }
    // C++ `:216` — 누리집 아이디·비밀번호(API 키와 다르다).
    let creds = AccountSettings::read_from_dir(
        &AccountSettings::user_config_dir(),
        AccountKind::VworldCadastral,
    );
    if !creds.is_ready() {
        outcome.error = NO_LOGIN_ACCOUNT.to_owned();
        outcome.account_missing = true;
        return outcome;
    }

    // C++ `:222-228` — 조사구역 + 5 km 를 WGS84 사각 범위로.
    let Some(wgs_box) = cadastral_work_box(scope_work, work_crs) else {
        outcome.error = "조사 주변 5km 범위를 만들지 못했습니다.".to_owned();
        return outcome;
    };
    report(0.02);
    let session = PortalSession::new(CADASTRAL_DATASET_URL);

    // C++ `:229-262` — 범위에 걸친 시·군·구 목록.
    let mut districts: Vec<vworld_client::CadastralDistrict> = Vec::new();
    for page in 1..=CADASTRAL_CATALOG_MAX_PAGES {
        if canceled() {
            outcome.error = CANCELED.to_owned();
            return outcome;
        }
        let url = with_page(&VworldClient::districts_url(&key, wgs_box), page);
        let Ok(text) = session.get_text(&url) else {
            outcome.error = cadastral_district_error_ko().to_owned();
            return outcome;
        };
        match parse_sigungu_features(text.as_bytes()) {
            Ok(rows) => {
                let before = districts.len();
                for row in rows {
                    if !districts.iter().any(|d| d.code == row.code) {
                        districts.push(row);
                    }
                }
                if districts.len() == before {
                    break; // 더 받을 쪽이 없다.
                }
            }
            Err(_) if page == 1 => {
                outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
                return outcome;
            }
            Err(_) => break,
        }
    }
    if districts.is_empty() {
        outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
        return outcome;
    }
    outcome.districts = districts.iter().map(|d| d.name.clone()).collect();
    report(0.08);

    // C++ `:264-292` — 쿠키를 받고 누리집에 로그인한다.
    if let Err(failure) = vworld_portal_login(&session, &creds.username, &creds.password) {
        outcome.error = failure.message;
        outcome.account_rejected = failure.account_rejected;
        return outcome;
    }
    report(0.12);

    // C++ `:293-303` — 자료 목록(ZIP 이름·크기·갱신일). 로그인한 세션으로 받는다.
    let mut resources: Vec<vworld_client::CadastralResource> = Vec::new();
    let mut finished = false;
    for page in 1..=CADASTRAL_CATALOG_MAX_PAGES {
        if canceled() {
            outcome.error = CANCELED.to_owned();
            return outcome;
        }
        let url = VworldClient::cadastral_dataset_page_url(page, CADASTRAL_CATALOG_PAGE_SIZE);
        let Ok(html) = session.get_text(&url) else {
            outcome.error = NETWORK_ERROR.to_owned();
            return outcome;
        };
        let rows = vworld_client::parse_cadastral_resources(&html);
        let short_page = rows.len() < CADASTRAL_CATALOG_PAGE_SIZE as usize;
        resources.extend(rows);
        if short_page {
            finished = true;
            break;
        }
    }
    if !finished {
        outcome.error = "VWorld 자료 목록이 끝나지 않아 내려받기를 중단했습니다.".to_owned();
        return outcome;
    }
    report(0.15);

    // C++ `:304-305` — 시·군·구마다 정확히 한 ZIP.
    let files = match vworld_client::select_cadastral_resources(&resources, &districts) {
        Ok(files) => files,
        Err(error) => {
            outcome.error = error;
            return outcome;
        }
    };
    if std::fs::create_dir_all(&survey_dir).is_err() {
        outcome.error = "조사 폴더에 지적도 저장 위치를 만들지 못했습니다.".to_owned();
        return outcome;
    }

    // C++ `:310-337` — ZIP 을 받는다(15 % → 65 %).
    let total = files.len().max(1);
    for (index, file) in files.iter().enumerate() {
        if canceled() {
            outcome.error = CANCELED.to_owned();
            return outcome;
        }
        if file.size_kb == 0 || file.size_kb > vworld_client::CADASTRAL_MAX_ZIP_KB {
            outcome.error = cadastral_zip_size_error_ko().to_owned();
            return outcome;
        }
        let path = survey_dir.join(vworld_client::cadastral_zip_file_name(file));
        // 이미 받은 파일은 ZIP 으로 열릴 때만 다시 받지 않는다(C++ `:317-319`).
        if !is_readable_zip(&path) {
            let url = VworldClient::cadastral_zip_url(&file.file_no);
            let limit = vworld_client::CADASTRAL_MAX_ZIP_KB * 1024;
            if let Err(why) = session.download_zip(&url, &path, limit, &cancel) {
                outcome.error = match why {
                    ZipDownloadError::Canceled => CANCELED,
                    ZipDownloadError::Timeout => vworld_client::CADASTRAL_TIMEOUT_MESSAGE,
                    ZipDownloadError::TooLarge => "지적도 서버 응답이 허용 크기를 넘었습니다.",
                    ZipDownloadError::NotZip => vworld_client::CADASTRAL_NOT_ZIP_MESSAGE,
                    ZipDownloadError::Disk => vworld_client::CADASTRAL_DISK_MESSAGE,
                    ZipDownloadError::Network => NETWORK_ERROR,
                }
                .to_owned();
                return outcome;
            }
        }
        outcome.downloaded.push(path.to_string_lossy().into_owned());
        report(0.15 + 0.5 * (index + 1) as f32 / total as f32);
    }
    outcome.ok = !outcome.downloaded.is_empty();
    if !outcome.ok {
        outcome.error = "조사 범위의 전체 시·군·구 목록을 확인하지 못했습니다.".to_owned();
    }
    outcome
}

/// 파일이 있고 ZIP 머리(`PK\x03\x04`)로 시작하는지.
fn is_readable_zip(path: &std::path::Path) -> bool {
    let mut head = [0_u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head))
        .is_ok()
        && &head == b"PK\x03\x04"
}

/// C++ `CadastralPortal.cpp:222-227` — 조사구역 범위에 5 km 를 더해 WGS84 사각 범위로.
pub(crate) fn cadastral_work_box(scope_work: [f64; 4], work_crs: WorkCrs) -> Option<[f64; 4]> {
    let src = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    let r = strata_net::ngii_topographic::NgiiTopographicClient::SCOPE_RADIUS_M;
    let box_work = vworld_client::cadastral_search_box(scope_work, r)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 예전에는 버퍼 0 을 넘겨 항상 `None` 이 나왔다(「조사 주변 5km 범위를 만들지 못했습니다」).
    #[test]
    fn the_search_box_is_the_survey_extent_plus_five_kilometres_in_wgs84() {
        // 조사구역 100 m × 100 m (EPSG:5186, 원점 127°E 38°N 근처에서 남쪽으로 150 km).
        let b = cadastral_work_box([200_000.0, 450_000.0, 200_100.0, 450_100.0], WorkCrs::Epsg5186)
            .expect("범위가 만들어져야 한다");
        // 가로·세로 10.1 km 쯤: 경도 약 0.113°, 위도 약 0.091°.
        assert!((b[2] - b[0] - 0.113).abs() < 0.005, "{b:?}");
        assert!((b[3] - b[1] - 0.091).abs() < 0.005, "{b:?}");
        // 조사구역 중심(127.0006°E, 36.6488°N 부근)을 품는다.
        assert!(b[0] < 127.0006 && 127.0006 < b[2] && b[1] < 36.6488 && 36.6488 < b[3], "{b:?}");
    }
}
