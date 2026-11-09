//! DEM 내려받기 작업 상태·스레드·결과 반영.
//!
//! `dem_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui;
use strata_net::dem_copernicus::{self, DemProgress, PreparedDem};

use crate::app_state::StrataAppState;

use super::dem_download_ui_paths::{
    dem_success_message_ko, dem_target_base_path, ensure_dem_folder, wgs84_extent_from_work,
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
            status: "조사구역 주변의 공개 DEM 30m 타일을 받아 한 장으로 붙입니다.".to_owned(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress_slot: Arc::new(Mutex::new(0.0)),
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
    let Some(extent) = wgs84_extent_from_work(app.home_map.viewport.world_bbox(), app.work_crs)
    else {
        state.status = dem_copernicus::DEM_EXTENT_INVALID_KO.to_owned();
        return;
    };
    let tiles = match dem_copernicus::tiles_for_extent(extent) {
        Ok(tiles) => tiles,
        Err(error) => {
            state.status = error.message_ko();
            return;
        }
    };
    state.tile_total = tiles.len();
    state.running = true;
    state.progress = 0.0;
    state.status = format!("DEM {}칸을 받는 중입니다.", tiles.len());
    let cancel = Arc::new(AtomicBool::new(false));
    let progress_slot = Arc::new(Mutex::new(0.0_f32));
    let (sender, receiver) = mpsc::channel();
    let thread_cancel = Arc::clone(&cancel);
    let thread_progress = Arc::clone(&progress_slot);
    std::thread::spawn(move || {
        let report = |fraction: f32| {
            if let Ok(mut slot) = thread_progress.lock() {
                *slot = fraction;
            }
        };
        let callback: &dyn Fn(f32) = &report;
        let progress = DemProgress {
            on_progress: Some(callback),
            is_cancelled: Some(&thread_cancel),
        };
        let result = dem_copernicus::prepare(extent, &target, &progress);
        let _ = sender.send(result);
    });
    state.cancel = cancel;
    state.progress_slot = progress_slot;
    state.receiver = Some(receiver);
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
    let message = dem_success_message_ko(result.downloaded_tiles, result.missing_tiles);
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
    let finished = state
        .receiver
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(result) = finished {
        state.running = false;
        state.receiver = None;
        state.progress = if result.is_ready() { 1.0 } else { 0.0 };
        state.status = if result.is_ready() {
            dem_success_message_ko(result.downloaded_tiles, result.missing_tiles)
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
