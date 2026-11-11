//! 「DEM」 받기의 뒤 작업 — 국토정보플랫폼의 공개DEM 을 수치지형도와 같은 통로로 받는다.
//!
//! 사용자 지시(2026-11-11): 「국토지리원에서 DEM을 받아야한다」「수치지형도를 받을때 쓰는 라인으로
//! DEM만 받아와야한다」. 웹 화면 도우미(`ka-hgis-web`)를 감춘 채 띄워 저장된 수치지형도 계정으로
//! 로그인 → 공개DEM 도엽 확인 → 공식 신청 → 파일 받기를 차례로 한다(`strata_net::ngii_portal_*`).
//! 한 번 받을 때마다 사용자 이름으로 공식 신청 1건이 접수된다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Map};
use strata_net::dem_copernicus::{DemStatus, PreparedDem};
use strata_net::ngii_portal_flow::{PortalFlow, Stage};
use strata_net::ngii_portal_runner::{
    dem_automation, PortalHelper, PortalInputs, PortalLogin, DEM_JS, POLL_INTERVAL, PORTAL_URL, TOPOGRAPHIC_JS,
};
use strata_secrets::accounts::{AccountKind, AccountSettings};

/// 신청서의 상세용도 기본 문구. C++ `KaTopographicBrowser.cpp:168`.
pub(crate) const DEFAULT_PURPOSE_KO: &str = "고고학 발굴조사 현장 도면 작성 및 지형 확인";
/// 사용목적 기본 분류: 공식 목록의 「제작 및 활용 → 교육, 학술, 연구」(2026-11-11 확인).
pub(crate) const DEFAULT_PURPOSE_CODE: &str = "CAAAC00006";
/// 자동 진행 전체에 주는 시간.
const TOTAL_LIMIT_SECS: u64 = 900;

/// 받기 한 번에 필요한 값.
pub(crate) struct NgiiDemJob {
    /// 받을 범위의 가운데(EPSG:5179)와 반경(m).
    pub(crate) center_5179: [f64; 2],
    pub(crate) radius_m: f64,
    /// 받은 파일을 둘 폴더(`<조사폴더>/DEM/원본`).
    pub(crate) folder: PathBuf,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) progress: Arc<Mutex<f32>>,
    pub(crate) status: Arc<Mutex<String>>,
}

fn failed(message: impl Into<String>) -> PreparedDem {
    PreparedDem { status: Some(DemStatus::Failed), error: message.into(), ..PreparedDem::default() }
}

/// 도우미 실행 파일: 앱 실행 파일 옆의 `ka-hgis-web.exe`.
fn helper_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let helper = exe.parent()?.join(if cfg!(windows) { "ka-hgis-web.exe" } else { "ka-hgis-web" });
    helper.is_file().then_some(helper)
}

/// 단계별 진행률(받는 중에는 파일 수로 채운다).
fn progress_of(flow: &PortalFlow) -> f32 {
    match flow.stage {
        Stage::Select => 0.05,
        Stage::OpenForm => 0.2,
        Stage::Form | Stage::Submit => 0.35,
        Stage::Order => 0.45,
        Stage::Files => 0.55,
        Stage::Download | Stage::Transfer => {
            0.6 + 0.4 * flow.received_files as f32 / flow.expected_files.max(1) as f32
        }
        Stage::Complete => 1.0,
        Stage::Stopped => 0.0,
    }
}

/// 받은 파일 가운데 지도에 올릴 표고 그림(GeoTIFF)을 찾는다. 압축 파일은 푼다.
fn find_raster(received: &[String], folder: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for path in received.iter().map(PathBuf::from) {
        let is_zip = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        if is_zip {
            let stem = path.file_stem().map_or_else(|| "dem".into(), |s| s.to_string_lossy().into_owned());
            files.extend(strata_storage::zip_extract::extract_zip(&path, &folder.join(stem)).unwrap_or_default());
        } else {
            files.push(path);
        }
    }
    files.into_iter().find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("tif") || e.eq_ignore_ascii_case("tiff")))
}

/// 뒤의 스레드에서 돈다. 끝나면 결과(성공이면 받은 파일 수, 올릴 그림 경로)를 돌려준다.
pub(crate) fn run_ngii_dem(job: &NgiiDemJob) -> PreparedDem {
    let say = |text: &str| {
        if let Ok(mut slot) = job.status.lock() {
            text.clone_into(&mut slot);
        }
    };
    let creds = AccountSettings::read_from_dir(&AccountSettings::user_config_dir(), AccountKind::NgiiTopographic);
    if !creds.is_ready() {
        return failed("더보기의 수치지형도 계정 설정에서 아이디와 비밀번호를 입력해 주세요.");
    }
    let Some(helper_path) = helper_exe() else {
        return failed("웹 화면 도우미(ka-hgis-web)를 찾지 못했습니다. 앱 설치 파일을 확인하세요.");
    };
    if std::fs::create_dir_all(&job.folder).is_err() {
        return failed("다운로드 폴더를 만들지 못했습니다. 저장 공간과 쓰기 권한을 확인하세요.");
    }
    // 전송 화면 설정용 스크립트는 파일로 넘긴다(끝나면 지운다).
    let init_script = job.folder.join(".ngii-transfer-setup.js");
    if std::fs::write(&init_script, TOPOGRAPHIC_JS).is_err() {
        return failed("다운로드 폴더에 쓰지 못했습니다. 쓰기 권한을 확인하세요.");
    }
    let result = drive(job, &helper_path, &init_script, creds.username, creds.password, &say);
    let _ = std::fs::remove_file(&init_script);
    result
}

fn drive(
    job: &NgiiDemJob,
    helper_path: &Path,
    init_script: &Path,
    id: String,
    password: String,
    say: &dyn Fn(&str),
) -> PreparedDem {
    let mut scope = Map::new();
    scope.insert("x".to_owned(), json!(job.center_5179[0].round()));
    scope.insert("y".to_owned(), json!(job.center_5179[1].round()));
    scope.insert("radius".to_owned(), json!(job.radius_m.round()));
    scope.insert("numbers".to_owned(), json!([]));
    scope.insert("cachedNumbers".to_owned(), json!([]));
    scope.insert("purposeCode".to_owned(), json!(DEFAULT_PURPOSE_CODE));
    let mut flow = PortalFlow::new(scope, 1);
    let inputs = PortalInputs {
        automation_js: dem_automation(TOPOGRAPHIC_JS, DEM_JS),
        login: PortalLogin { id, password, attempt: 1 },
        birth_date: String::new(),
        purpose: DEFAULT_PURPOSE_KO.to_owned(),
    };
    let mut helper = match PortalHelper::start(helper_path, &job.folder, true, Some(init_script)) {
        Ok(helper) => helper,
        Err(why) => return failed(why),
    };
    if let Err(why) = helper.open(PORTAL_URL) {
        return failed(why);
    }
    let started = std::time::Instant::now();
    while !matches!(flow.stage, Stage::Complete | Stage::Stopped) {
        if job.cancel.load(Ordering::Relaxed) {
            return PreparedDem {
                status: Some(DemStatus::Cancelled),
                error: "자동 진행을 중지했습니다. 이미 신청한 자료는 공식 신청 내역에서 확인할 수 있습니다.".to_owned(),
                ..PreparedDem::default()
            };
        }
        if started.elapsed().as_secs() > TOTAL_LIMIT_SECS {
            return failed("국토정보플랫폼의 응답이 너무 오래 걸립니다. 공식 신청 내역을 확인한 뒤 다시 시도하세요.");
        }
        if let Err(why) = helper.step(&mut flow, &inputs) {
            return failed(why);
        }
        if let Ok(mut slot) = job.progress.lock() {
            *slot = progress_of(&flow);
        }
        say(&if flow.message.is_empty() {
            format!("국토정보플랫폼 · {}", flow.phase)
        } else {
            format!("국토정보플랫폼 · {} — {}", flow.phase, flow.message)
        });
        std::thread::sleep(POLL_INTERVAL);
    }
    if let Some(why) = flow.failed {
        return failed(why);
    }
    if helper.received.is_empty() {
        return failed(if flow.message.is_empty() { "받은 파일이 없습니다.".to_owned() } else { flow.message });
    }
    let raster_uri = find_raster(&helper.received, &job.folder);
    let warnings = if raster_uri.is_some() {
        Vec::new()
    } else {
        vec!["받은 파일에서 지도에 올릴 표고 그림(GeoTIFF)을 찾지 못했습니다. 파일은 조사 폴더의 DEM/원본 에 있습니다.".to_owned()]
    };
    PreparedDem {
        status: Some(DemStatus::Ready),
        raster_uri,
        downloaded_tiles: helper.received.len(),
        warnings,
        ..PreparedDem::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_rises_with_the_stages_and_with_received_files() {
        let mut flow = PortalFlow::new(Map::new(), 1);
        let mut last = -1.0;
        for stage in [Stage::Select, Stage::OpenForm, Stage::Form, Stage::Order, Stage::Files, Stage::Transfer] {
            flow.stage = stage;
            let now = progress_of(&flow);
            assert!(now > last, "{stage:?}");
            last = now;
        }
        flow.expected_files = 2;
        flow.received_files = 1;
        assert!((progress_of(&flow) - 0.8).abs() < 1e-6);
        flow.stage = Stage::Complete;
        assert!((progress_of(&flow) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_map_picture_is_looked_for_among_received_files() {
        let dir = std::env::temp_dir().join(format!("strata-dem-ngii-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("폴더");
        let tif = dir.join("36705.TIF");
        std::fs::write(&tif, b"x").expect("쓰기");
        let text = dir.join("36705.txt").to_string_lossy().into_owned();
        let found = find_raster(&[text.clone(), tif.to_string_lossy().into_owned()], &dir);
        assert_eq!(found, Some(tif));
        assert_eq!(find_raster(&[text], &dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
