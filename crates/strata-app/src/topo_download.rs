//! The 「수치지형」 and 「DEM」 buttons: the NGII order for 5 km around the drawn survey area, run in the
//! background and shown as one line of the bottom gauge (docs/RUST_PLAN.md 2.8, 2.9, 4.3 F4-e). The C++
//! opens a small window for the sheet run (`MainWindowTopographic.cpp:18-72`, `KaTopographicBrowser`); here
//! there is none. The DEM run is this app's own (owner 2026-11-15: 「수치지형도와 같은 구역의 dem을 별도로
//! 받아야한다」): the C++ fetches a Copernicus DEM and takes the NGII one only as a file the user picks.

use std::path::{Path, PathBuf};

use strata_net::ngii_auto::{Account, Scope, DEFAULT_PURPOSE};
#[cfg(windows)]
use strata_net::ngii_auto::{Product, DEM_PAGE_SCRIPT, PAGE_SCRIPT};
#[cfg(any(windows, test))]
use strata_net::ngii_auto::Stage;
use strata_net::ngii_topographic::NgiiTopographicClient;
use strata_secrets::accounts::Credentials;
use strata_types::crs::WorkCrs;

#[cfg(any(windows, test))]
use crate::download_jobs::DownloadJobState;
use crate::download_jobs::StopCode;

/// 5 km around the survey area (`MainWindowDownloads.cpp:187`; owner 2026-11-15).
pub(crate) const TOPO_RADIUS_M: f64 = 5000.0;
/// The name of the sheet job in the gauge.
pub(crate) const TOPO_JOB_TITLE: &str = "수치지형도";

/// What a hidden order of the NGII portal is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderOf {
    /// 1:25,000 topographic sheets (DXF).
    Sheets,
    /// The public DEM of the 1:50,000 sheets that hold the same circle.
    Dem,
}

impl OrderOf {
    /// The name of the job in the gauge.
    pub(crate) const fn job_title(self) -> &'static str {
        match self {
            Self::Sheets => TOPO_JOB_TITLE,
            Self::Dem => "DEM",
        }
    }

    /// The folder of the survey folder the files go to.
    pub(crate) const fn folder(self) -> &'static str {
        match self {
            Self::Sheets => "지형도",
            Self::Dem => "DEM",
        }
    }

    /// What is fetched, as the object of a sentence.
    pub(crate) const fn object(self) -> &'static str {
        match self {
            Self::Sheets => "수치지형도를",
            Self::Dem => "DEM 을",
        }
    }

    /// The name of the run in the log of a scripted run.
    #[cfg(windows)]
    pub(crate) const fn log_name(self) -> &'static str {
        match self {
            Self::Sheets => "수치지형",
            Self::Dem => "DEM",
        }
    }

    #[cfg(windows)]
    pub(crate) const fn product(self) -> Product {
        match self {
            Self::Sheets => Product::Sheets,
            Self::Dem => Product::Dem,
        }
    }

    /// The page script that drives the portal for this order.
    #[cfg(windows)]
    pub(crate) const fn script(self) -> &'static str {
        match self {
            Self::Sheets => PAGE_SCRIPT,
            Self::Dem => DEM_PAGE_SCRIPT,
        }
    }
}

/// What is known when the button is pressed, without asking the site.
#[derive(Debug, Clone)]
pub(crate) struct TopoFacts<'a> {
    pub(crate) survey_path: Option<&'a Path>,
    /// Centre of the drawn survey area, work CRS metres.
    pub(crate) centre_work: Option<[f64; 2]>,
    pub(crate) work_crs: WorkCrs,
    pub(crate) online: bool,
}

/// A run that may start.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TopoRequest {
    pub(crate) scope: Scope,
    /// `<survey folder>/지형도/원본`.
    pub(crate) downloads: PathBuf,
    pub(crate) account: Account,
}

/// Checks a press in the order the C++ does, then what the plan asks to know before the site is touched.
/// `Err` is why nothing starts: a code for the gauge and the words. The stored account is read last, and
/// only when nothing else stands in the way.
pub(crate) fn topo_request(of: OrderOf, facts: TopoFacts<'_>, stored_account: impl FnOnce() -> Credentials) -> Result<TopoRequest, (StopCode, String)> {
    // MainWindowTopographic.cpp:20-24
    let Some(folder) = crate::survey_folder::survey_folder(facts.survey_path) else {
        return Err((StopCode::Dir1, format!("먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 {} 폴더에 저장합니다.", of.folder())));
    };
    let needs_area = || (StopCode::Area1, format!("조사구역을 먼저 그려 주세요. 조사구역에서 반경 5km 안의 {} 받습니다.", of.object()));
    let centre = facts.centre_work.filter(|c| c[0].is_finite() && c[1].is_finite()).ok_or_else(needs_area)?;
    // MainWindowDownloads.cpp:183-186 — the portal is asked in EPSG:5179.
    let portal = crate::ngii_topographic_ui::portal_center(centre, facts.work_crs).ok_or_else(needs_area)?;
    let lon_lat = crate::ngii_topographic_ui::center_wgs84(centre, facts.work_crs).ok_or_else(needs_area)?;
    if !facts.online {
        return Err((StopCode::Site1, "인터넷 접속이 꺼져 있어 국토정보플랫폼에 연결할 수 없습니다.".to_owned()));
    }
    let account = stored_account();
    if !account.is_ready() {
        // The words of the C++ page script when the login page finds no account (ngii-topographic.js:196).
        return Err((StopCode::Acc1, "더보기의 수치지형도 계정 설정에서 아이디와 비밀번호를 입력해 주세요.".to_owned()));
    }
    // The sheets the circle may touch are only candidates: the portal answers with its own list
    // (KaTopographicBrowser.cpp:845-849). Degrees for 5 km at this latitude.
    let d_lat = TOPO_RADIUS_M / 111_320.0;
    let d_lon = d_lat / lon_lat[1].to_radians().cos().max(0.2);
    let mut numbers = match of {
        OrderOf::Sheets => NgiiTopographicClient::sheets_for_bbox_25k([lon_lat[0] - d_lon, lon_lat[1] - d_lat, lon_lat[0] + d_lon, lon_lat[1] + d_lat]),
        // The portal's own list says which 1:50,000 sheets hold the circle.
        OrderOf::Dem => Vec::new(),
    };
    if numbers.is_empty() && of == OrderOf::Sheets {
        numbers = crate::ngii_topographic_ui::nearby_sheet_numbers(lon_lat);
    }
    // What the survey has is named to the portal, which then leaves it out of the order
    // (the C++ `received` list, KaTopographicScopePanel.cpp:627-632).
    let cached_numbers = match of {
        OrderOf::Sheets => strata_topo::library::Library::open(&folder.join(of.folder())).numbers(),
        OrderOf::Dem => crate::topo_download::dem::received_numbers(&folder.join(of.folder())),
    };
    Ok(TopoRequest {
        scope: Scope { x: portal[0], y: portal[1], radius: TOPO_RADIUS_M, numbers, cached_numbers },
        downloads: folder.join(of.folder()).join("원본"),
        // The C++ sends no birth date of its own (its field is hidden and emptied, KaTopographicBrowser.cpp:251,
        // 267): the site's form carries the member's, and the page script says so when it does not.
        account: Account { login_id: account.username, login_password: account.password, birth_date: String::new(), purpose: DEFAULT_PURPOSE.to_owned() },
    })
}

/// A run as the gauge needs to see it. The hidden session is Windows-only; the gauge tests construct it.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunFacts<'a> {
    pub(crate) stage: Stage,
    pub(crate) phase: &'a str,
    pub(crate) attention: &'a str,
    pub(crate) failure_phase: &'a str,
    pub(crate) status: &'a str,
    pub(crate) received: u64,
    pub(crate) total: Option<u64>,
    /// Sheets handed on so far.
    pub(crate) sheets: usize,
    /// The user stopped the run.
    pub(crate) cancelled: bool,
}

/// The phases in the order of the C++ stage text `[n/6]` (`KaTopographicBrowser.cpp:411-414`).
#[cfg(any(windows, test))]
const PHASES: [&str; 6] = ["도엽 확인", "로그인", "신청", "파일 준비", "다운로드", "지도에 올리기"];

/// The gauge line for a run.
#[cfg(any(windows, test))]
pub(crate) fn gauge_state(run: &RunFacts<'_>) -> DownloadJobState {
    match run.stage {
        Stage::Complete => {
            let result = if run.sheets == 0 && !run.status.is_empty() { run.status.to_owned() } else { format!("도엽 {}장을 받았습니다.", run.sheets) };
            DownloadJobState::Done { result }
        }
        Stage::Stopped if run.cancelled || run.attention.is_empty() => DownloadJobState::Cancelled,
        Stage::Stopped => {
            let code = match run.failure_phase {
                // Stopped without having failed: the portal has no sheet for this place.
                "" => StopCode::None1,
                "로그인" => StopCode::Acc2,
                _ => StopCode::Site1,
            };
            DownloadJobState::Stopped { code, reason: run.attention.to_owned() }
        }
        _ => {
            let fraction = match (run.stage, run.total) {
                (Stage::Transfer, Some(total)) if total > 0 => Some((run.received as f64 / total as f64).clamp(0.0, 1.0) as f32),
                _ => None,
            };
            let stage = if run.attention.is_empty() {
                let at = PHASES.iter().position(|p| *p == run.phase).map_or(6, |i| i + 1);
                format!("[{at}/6] {}", run.phase)
            } else {
                run.attention.to_owned()
            };
            DownloadJobState::Running { fraction, stage: Some(stage) }
        }
    }
}

#[path = "topo_download_run.rs"]
mod run;
pub(crate) use run::TopoDownload;
#[path = "topo_post.rs"]
pub(crate) mod post;
#[path = "dem_post.rs"]
pub(crate) mod dem;

#[cfg(test)]
#[path = "topo_download_tests.rs"]
mod tests;
