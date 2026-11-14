//! The running side of the 「수치지형」 button: a press is checked, the hidden run is started inside the app's
//! window and fed every frame, and the gauge line follows it.

use std::path::PathBuf;
use std::sync::Arc;

use strata_net::ngii_auto::{AutoConfig, AutoEvent, Stage, PAGE_SCRIPT, PORTAL_HOME};
use strata_ngii_run::{NgiiRun, RunOptions};
use strata_secrets::accounts::{AccountKind, AccountSettings};

use super::{gauge_state, topo_request, RunFacts, TopoFacts, TopoRequest, TOPO_JOB_TITLE};
use crate::app_state::{MainTab, StrataAppState};
use crate::download_jobs::{DownloadJobHandle, DownloadJobState, StopCode};

/// The run of the button, if there is one. A copy of the app state carries none.
#[derive(Debug, Default)]
pub(crate) struct TopoDownload {
    /// What a press came to; the next frame, which has the app's window, starts the run or shows why not.
    pending: Option<Result<TopoRequest, (StopCode, String)>>,
    run: Option<NgiiRun>,
    job: Option<DownloadJobHandle>,
    shown: Option<DownloadJobState>,
    cancelled: bool,
    /// Data files of sheets the run has handed on, with the portal's record of each (for the map, F4-d).
    pub(crate) received: Vec<(PathBuf, serde_json::Map<String, serde_json::Value>)>,
}

impl Clone for TopoDownload {
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// Where the hidden browser keeps the working files of a session (removed when the session ends).
fn browser_data_parent() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("ka-hgis").join("webview")
}

fn report(job: &DownloadJobHandle, state: &DownloadJobState) {
    match state {
        DownloadJobState::Running { fraction, stage } => job.report_running(*fraction, stage.clone()),
        DownloadJobState::Done { result } => job.report_done(result.clone()),
        DownloadJobState::Stopped { code, reason } => job.report_stopped(*code, reason.clone()),
        DownloadJobState::Cancelled => job.report_cancelled(),
        DownloadJobState::Waiting | DownloadJobState::Recovering { .. } => {}
    }
}

impl StrataAppState {
    /// The ribbon button 「수치지형」 (`MainWindowRibbon.cpp:382-398` → `openTopographicDownload`).
    pub(crate) fn press_topographic_download(&mut self) {
        self.active_tab = MainTab::Map;
        if self.topo.run.is_some() || self.topo.pending.is_some() {
            self.show_status_message("수치지형도를 받고 있습니다. 아래 게이지에서 진행을 볼 수 있습니다.");
            return;
        }
        let facts = TopoFacts { survey_path: self.survey_path.as_deref(), centre_work: self.reference_circle_centre(), work_crs: self.work_crs, online: strata_net::network::is_allowed() };
        let stored = || AccountSettings::read_from_dir(&AccountSettings::user_config_dir(), AccountKind::NgiiTopographic);
        match topo_request(facts, stored) {
            Ok(request) => {
                self.show_status_message("수치지형도를 받기 시작합니다. 아래 게이지에서 진행을 볼 수 있습니다.");
                self.topo.pending = Some(Ok(request));
            }
            // What the user mends on the map is said in the status line, as the other buttons do.
            Err((StopCode::Dir1 | StopCode::Area1, reason)) => self.show_status_message(reason),
            // The rest also goes to the gauge, with its code.
            Err(refusal) => {
                self.show_status_message(refusal.1.clone());
                self.topo.pending = Some(Err(refusal));
            }
        }
    }

    /// Whether a press is still being worked on (the scripted screenshots wait for its end).
    pub(crate) fn topographic_download_busy(&self) -> bool {
        self.topo.run.is_some() || self.topo.pending.is_some()
    }

    fn begin_topographic_run(&mut self, frame: &eframe::Frame, request: TopoRequest, now: f64) {
        // MainWindowTopographic.cpp:25-33
        if std::fs::create_dir_all(&request.downloads).is_err() {
            let reason = "조사 폴더에 지형도 폴더를 만들지 못했습니다. 저장 위치의 쓰기 권한과 공간을 확인한 뒤 다시 열어 주세요.";
            self.download_jobs.add_job(TOPO_JOB_TITLE, DownloadJobState::Stopped { code: StopCode::Dir2, reason: reason.to_owned() }, now);
            return;
        }
        let first = DownloadJobState::Running { fraction: None, stage: Some("[1/6] 도엽 확인".to_owned()) };
        let (_, job) = self.download_jobs.add_job(TOPO_JOB_TITLE, first.clone(), now);
        let config = AutoConfig { portal_url: PORTAL_HOME.to_owned(), script: PAGE_SCRIPT.to_owned(), download_directory: request.downloads, account: request.account, test_mode: false };
        let options = RunOptions { data_parent: browser_data_parent(), config, network_allowed: Arc::new(strata_net::network::is_allowed) };
        match NgiiRun::start(frame, options) {
            Ok(mut run) => {
                run.prepare_sheets(request.scope);
                self.topo = TopoDownload { run: Some(run), job: Some(job), shown: Some(first), ..TopoDownload::default() };
            }
            Err(_) => job.report_stopped(StopCode::App1, "보이지 않는 브라우저를 시작하지 못했습니다. Windows 의 WebView2 구성 요소를 확인한 뒤 다시 눌러 주세요."),
        }
    }

    /// Every frame, with the app's window at hand.
    pub(crate) fn drive_topographic_download(&mut self, frame: &eframe::Frame, now: f64) {
        match self.topo.pending.take() {
            Some(Ok(request)) => self.begin_topographic_run(frame, request, now),
            Some(Err((code, reason))) => drop(self.download_jobs.add_job(TOPO_JOB_TITLE, DownloadJobState::Stopped { code, reason }, now)),
            None => {}
        }
        let Some(run) = self.topo.run.as_mut() else {
            return;
        };
        // The gauge's own cancel.
        if !self.topo.cancelled && self.topo.job.as_ref().is_some_and(DownloadJobHandle::is_cancelled) {
            self.topo.cancelled = true;
            run.stop();
        }
        for event in run.frame(frame) {
            if let AutoEvent::SheetDownloaded { path, record } = event {
                self.topo.received.push((path, record));
            }
        }
        let (sheets, cancelled) = (self.topo.received.len(), self.topo.cancelled);
        let state = run.with_state(|m| {
            let (_, received, total) = m.current_file();
            gauge_state(&RunFacts { stage: m.stage(), phase: m.phase(), attention: m.attention(), failure_phase: m.failure_phase(), status: m.status(), received, total, sheets, cancelled })
        });
        let over = matches!(run.stage(), Stage::Complete | Stage::Stopped);
        if self.topo.shown.as_ref() != Some(&state) {
            if let Some(job) = &self.topo.job {
                report(job, &state);
            }
            // A scripted run has nobody watching the gauge: its lines go to the log (phases and the
            // site's own words only; where the pages stand is named without query or form values).
            if self.snap.is_some() {
                eprintln!("[SNAP] 수치지형 {state:?} | {}", run.page_outline());
            }
            self.topo.shown = Some(state);
        }
        // The session is closed and its folder removed over the next frames; then the run is let go.
        if over && run.finished() {
            if self.snap.is_some() {
                let files: Vec<_> = self.topo.received.iter().map(|(path, _)| path.display().to_string()).collect();
                eprintln!("[SNAP] 수치지형 끝: 받은 도엽 자료 {} — {files:?}", files.len());
            }
            self.topo.run = None;
            self.topo.job = None;
        }
    }
}
