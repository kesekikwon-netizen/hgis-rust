//! 지적 내려받기 창의 작업 상태·스레드 시작·결과 폴링.
//!
//! `cadastral_download_ui.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.
//! 창 그리기(`cadastral_download_ui_render`)가 상태 필드를 읽고 써야 해서
//! 구조체와 필드를 `pub(crate)` 로 둔다 — 크레이트 밖으로는 나가지 않는다.

use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex, OnceLock};

use eframe::egui;
use strata_net::vworld_client::VworldClient;

use crate::app_state::StrataAppState;

use super::cadastral_download_ui_paths::{cadastral_raw_dir, cadastral_success_message_ko, CadastralOutcome};
use super::cadastral_download_ui_worker::run_download;

/// 지적 내려받기 작업 상태
pub(crate) struct CadastralState {
    pub(crate) running: bool,
    pub(crate) progress: f32,
    pub(crate) status: String,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) progress_slot: Arc<Mutex<f32>>,
    pub(crate) receiver: Option<mpsc::Receiver<CadastralOutcome>>,
    /// 받은 ZIP 경로 (C++ `CadastralPortal.cpp:308` `sources`)
    pub(crate) downloaded: Vec<String>,
}

impl CadastralState {
    pub(crate) fn idle() -> Self {
        Self {
            running: false,
            progress: 0.0,
            status: "조사구역 경계에서 주변 5km의 시·군·구 지적도를 준비합니다.".to_owned(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress_slot: Arc::new(Mutex::new(0.0)),
            receiver: None,
            downloaded: Vec::new(),
        }
    }
}

/// MainWindowCadastral.cpp:114 — 조사구역이 없을 때.
pub(crate) const NO_SURVEY_AREA_MESSAGE: &str = "조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다.";

pub(crate) fn cadastral_state() -> &'static Mutex<CadastralState> {
    static STATE: OnceLock<Mutex<CadastralState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(CadastralState::idle()))
}

pub(crate) fn start_download(app: &StrataAppState, state: &mut CadastralState) {
    if state.running {
        return;
    }
    state.downloaded.clear();
    let Some(survey_path) = app.survey_path.as_deref() else {
        state.status = VworldClient::NO_SURVEY_MESSAGE.to_owned();
        return;
    };
    let Some(dir) = cadastral_raw_dir(Some(survey_path)) else {
        state.status = VworldClient::NO_SURVEY_MESSAGE.to_owned();
        return;
    };
    // MainWindowCadastral.cpp:93-114 — 범위는 조사구역 도형들(작업 좌표계), 없으면 받지 않는다.
    // C++ 은 도형들을 합쳐 5km 둘레를 만든 뒤 그 바깥 상자로 시·군·구를 찾는다(둘레의 바깥 상자 = 이 상자 ± 5km).
    let Some(area) = crate::survey_area_setup::survey_area_extent(app) else {
        state.status = NO_SURVEY_AREA_MESSAGE.to_owned();
        return;
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(Mutex::new(0.0_f32));
    let (sender, receiver) = mpsc::channel();
    state.running = true;
    state.progress = 0.0;
    state.status = "조사구역 경계에서 주변 5km의 시·군·구 지적도를 확인하고 있습니다.".to_owned();
    let thread_cancel = Arc::clone(&cancel);
    let thread_progress = Arc::clone(&progress);
    let work_crs = app.work_crs;
    std::thread::spawn(move || {
        let outcome = run_download(dir, area, work_crs, thread_cancel, thread_progress);
        let _ = sender.send(outcome);
    });
    state.cancel = cancel;
    state.progress_slot = progress;
    state.receiver = Some(receiver);
}

pub(crate) fn poll(app: &mut StrataAppState, ctx: &egui::Context) {
    let mut state = match cadastral_state().lock() {
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
    if let Some(outcome) = finished {
        state.running = false;
        state.receiver = None;
        state.downloaded = outcome.downloaded.clone();
        if outcome.ok {
            state.progress = 1.0;
            let message =
                cadastral_success_message_ko(outcome.downloaded.len(), outcome.districts.len());
            state.status = message.clone();
            app.status_bar_hint = message;
            // C++ `MainWindowCadastral.cpp:166-168` — 받은 지적도를 최상위 레이어로 올린다.
            app.home_map
                .real_survey
                .layer_visible
                .insert("지적도".to_owned(), true);
        } else {
            state.progress = 0.0;
            state.status = outcome.error;
        }
    }
    if state.running {
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
}
