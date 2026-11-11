//! DEM 내려받기 작업 상태·스레드·결과 반영.
//!
//! `dem_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui;
use strata_net::dem_copernicus::PreparedDem;

use crate::dem_ngii_worker::{run_ngii_dem, NgiiDemJob};

use crate::app_state::StrataAppState;

use super::dem_download_ui_paths::{
    dem_target_base_path, ensure_dem_folder,
    DEM_FOLDER_CREATE_FAILED_KO, DEM_LEGEND_KEYS, DEM_NO_SURVEY_KO,
};

/// 창 그리기(`dem_download_ui_render`)가 같은 구조체의 필드를 읽고 써야 해서
/// 구조체와 필드를 `pub(crate)` 로 둔다 — 크레이트 밖으로는 나가지 않는다.
pub(crate) struct DemDownloadState {
    pub(crate) running: bool,
    pub(crate) progress: f32,
    pub(crate) status: String,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) progress_slot: Arc<Mutex<f32>>,
    /// 작업 스레드가 적는 지금 단계 안내.
    pub(crate) status_slot: Arc<Mutex<String>>,
    pub(crate) receiver: Option<mpsc::Receiver<PreparedDem>>,
    /// 완료된 결과. `storage_dir` 를 들고 있어야 받은 `dem.tif` 가 지워지지 않는다.
    pub(crate) result: Option<PreparedDem>,
    pub(crate) tile_total: usize,
}

impl DemDownloadState {
    fn idle() -> Self {
        Self {
            running: false,
            progress: 0.0,
            status: "국토정보플랫폼에서 화면 범위의 공개DEM 을 받습니다. 받을 때마다 저장된 수치지형도 계정으로 공식 신청 1건이 접수됩니다.".to_owned(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress_slot: Arc::new(Mutex::new(0.0)),
            status_slot: Arc::new(Mutex::new(String::new())),
            receiver: None,
            result: None,
            tile_total: 0,
        }
    }
}

pub(crate) fn dem_state() -> &'static Mutex<DemDownloadState> {
    static STATE: OnceLock<Mutex<DemDownloadState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(DemDownloadState::idle()))
}
pub(crate) fn start_download(app: &StrataAppState, state: &mut DemDownloadState) {
    if state.running {
        return;
    }
    state.result = None;
    let target = match dem_target_base_path(app.survey_path.as_deref()) {
        Some(target) => target,
        None => {
            state.status = DEM_NO_SURVEY_KO.to_owned();
            return;
        }
    };
    if !ensure_dem_folder(app.survey_path.as_deref()) {
        state.status = DEM_FOLDER_CREATE_FAILED_KO.to_owned();
        return;
    }
    // 받을 범위: 화면 가운데와 화면의 절반 크기(500 m ~ 5 km)를 국토정보플랫폼의 좌표(EPSG:5179)로.
    let vp = &app.home_map.viewport;
    let half_m = (vp.screen_size_px[0].max(vp.screen_size_px[1]) * 0.5 * vp.meters_per_pixel).clamp(500.0, 5_000.0);
    let work = match app.work_crs {
        strata_types::crs::WorkCrs::Epsg5186 => strata_types::crs::CrsCode::Epsg5186,
        strata_types::crs::WorkCrs::Epsg5187 => strata_types::crs::CrsCode::Epsg5187,
    };
    let to_portal = strata_geo::crs_transform::CrsTransformer::new(work, strata_types::crs::CrsCode::Epsg5179);
    let center_5179 = to_portal.transform_xy(vp.center_xy[0], vp.center_xy[1]);
    let folder = PathBuf::from(&target).parent().map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf);
    state.tile_total = 0;
    state.running = true;
    state.progress = 0.0;
    state.status = "국토정보플랫폼에 연결하는 중입니다.".to_owned();
    let cancel = Arc::new(AtomicBool::new(false));
    let progress_slot = Arc::new(Mutex::new(0.0_f32));
    let status_slot = Arc::new(Mutex::new(state.status.clone()));
    let (sender, receiver) = mpsc::channel();
    let job = NgiiDemJob {
        center_5179,
        radius_m: half_m,
        folder,
        topographic: false,
        cancel: Arc::clone(&cancel),
        progress: Arc::clone(&progress_slot),
        status: Arc::clone(&status_slot),
    };
    std::thread::spawn(move || {
        let _ = sender.send(run_ngii_dem(&job));
    });
    state.status_slot = status_slot;
    state.cancel = cancel;
    state.progress_slot = progress_slot;
    state.receiver = Some(receiver);
}

/// 리본의 「DEM」 단추에서 바로 받기를 시작한다.
///
/// 받기는 사용자 이름으로 공식 신청을 내는 일이므로, 사람이 단추를 누른 때에만 한다:
/// 시험과 화면 캡처 실행(`--snap`)에서는 시작하지 않는다(캡처에서 꼭 필요하면
/// `KA_HGIS_SNAP_REAL_ORDER=1`).
pub(crate) fn start_from_ribbon(app: &StrataAppState) {
    let automated = cfg!(test) || (app.snap.is_some() && std::env::var_os("KA_HGIS_SNAP_REAL_ORDER").is_none());
    if automated {
        return;
    }
    if let Ok(mut state) = dem_state().lock() {
        start_download(app, &mut state);
    }
}

/// 받기가 끝났을 때의 안내.
fn ngii_dem_done_message(result: &PreparedDem) -> String {
    let mut text = format!("국토정보플랫폼에서 공개DEM 파일 {}개를 받았습니다.", result.downloaded_tiles);
    for warning in &result.warnings {
        text.push(' ');
        text.push_str(warning);
    }
    text
}

pub(crate) fn apply_result(app: &mut StrataAppState, state: &mut DemDownloadState) {
    let Some(result) = state.result.as_ref() else {
        return;
    };
    if !result.is_ready() {
        return;
    }
    // C++ `MainWindowDownloads.cpp:69-96` — 받은 래스터를 참조 지도로 올린다.
    for key in DEM_LEGEND_KEYS {
        app.home_map
            .real_survey
            .layer_visible
            .insert(key.to_owned(), true);
    }
    let mut message = ngii_dem_done_message(result);
    // 받은 표고를 화면 주변만 읽어 색띠 + 음영 그림으로 올린다.
    if let Some(tif) = result.raster_uri.clone() {
        if let Err(why) = app.load_dem_image(&tif) {
            message = format!("{message} 다만 지도에 그리지는 못했습니다: {why}");
        }
    }
    app.status_bar_hint = message.clone();
    app.home_map.draw_guide_message = Some(message);
}

pub(crate) fn poll(app: &mut StrataAppState, ctx: &egui::Context) {
    let mut state = match dem_state().lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    let shared_progress = state.progress_slot.lock().map(|slot| *slot).ok();
    if let Some(progress) = shared_progress {
        state.progress = progress;
    }
    if state.running {
        let said = state.status_slot.lock().map(|slot| slot.clone()).unwrap_or_default();
        if !said.is_empty() {
            state.status = said;
        }
    }
    let finished = state
        .receiver
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(result) = finished {
        state.running = false;
        state.receiver = None;
        state.progress = if result.is_ready() { 1.0 } else { 0.0 };
        state.status = if result.is_ready() {
            ngii_dem_done_message(&result)
        } else {
            result.error.clone()
        };
        state.result = Some(result);
        apply_result(app, &mut state);
    }
    if state.running {
        // 진행률은 작업 스레드가 채운다. 창이 멈추지 않도록 다음 프레임을 예약한다.
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
}
/// 화면에 올라온 DEM 타일 파일 경로 (완료 뒤 읽기용)
#[must_use]
pub fn current_dem_raster_path() -> Option<PathBuf> {
    dem_state()
        .lock()
        .ok()
        .and_then(|state| state.result.as_ref().and_then(|r| r.raster_uri.clone()))
}
