//! The running side of the 「수치지형」 and 「DEM」 buttons: a press is checked, the hidden run is started
//! inside the app's window and fed every frame, and the gauge line follows it.

use std::path::PathBuf;
use std::sync::Arc;

use strata_net::ngii_auto::{AutoConfig, AutoEvent, Stage, PORTAL_HOME};
use strata_ngii_run::{NgiiRun, RunOptions};
use strata_secrets::accounts::{AccountKind, AccountSettings};

use super::post::TopoPost;
use super::{gauge_state, topo_request, OrderOf, RunFacts, TopoFacts, TopoRequest};
use crate::app_state::{MainTab, StrataAppState};
use crate::download_jobs::{DownloadJobHandle, DownloadJobState, StopCode};

/// The run of one of the two buttons, if there is one. A copy of the app state carries none.
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
    /// The work on what the run left: listing, placing, merging (topo_post.rs).
    pub(crate) post: Option<TopoPost>,
}

#[cfg(test)]
#[path = "topo_download_run_tests.rs"]
mod tests;

impl Clone for TopoDownload {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl TopoDownload {
    fn is_busy(&self) -> bool {
        self.run.is_some() || self.pending.is_some() || self.post.is_some()
    }

    /// The order is of a survey that is no longer open: a press that waits is forgotten, a run is told to
    /// stop (it then ends as a cancelled one, whose files stay in the folder of its own survey and go on no
    /// map), and the work after a run is handed back to be told the same.
    pub(crate) fn abandon(&mut self) -> Option<TopoPost> {
        self.pending = None;
        if let (false, Some(run)) = (self.cancelled, self.run.as_mut()) {
            self.cancelled = true;
            run.stop();
        }
        self.post.take()
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
    fn order_mut(&mut self, of: OrderOf) -> &mut TopoDownload {
        match of {
            OrderOf::Sheets => &mut self.topo,
            OrderOf::Dem => &mut self.dem,
        }
    }

    /// The ribbon button 「수치지형」 and the first row of its menu: the order runs hidden, and what it
    /// receives goes on the map (topo_post.rs). A sheet the survey has is not ordered again.
    pub(crate) fn open_topographic_download(&mut self) {
        self.press_ngii_order(OrderOf::Sheets);
    }

    /// The order itself (`MainWindowRibbon.cpp:382-398` → `openTopographicDownload`).
    pub(crate) fn press_ngii_order(&mut self, of: OrderOf) {
        self.active_tab = MainTab::Map;
        // One hidden browser at a time: the two orders use the same account and the same portal session.
        if let Some(busy) = [OrderOf::Sheets, OrderOf::Dem].into_iter().find(|of| self.order_mut(*of).is_busy()) {
            self.show_status_message(format!("{} 받고 있습니다. 아래 게이지에서 진행을 볼 수 있습니다.", busy.object()));
            return;
        }
        let facts = TopoFacts { survey_path: self.survey_path.as_deref(), centre_work: self.reference_circle_centre(), work_crs: self.work_crs, online: strata_net::network::is_allowed() };
        let stored = || AccountSettings::read_from_dir(&AccountSettings::user_config_dir(), AccountKind::NgiiTopographic);
        match topo_request(of, facts, stored) {
            Ok(request) => {
                self.show_status_message(format!("{} 받기 시작합니다. 아래 게이지에서 진행을 볼 수 있습니다.", of.object()));
                self.order_mut(of).pending = Some(Ok(request));
            }
            // What the user mends on the map is said in the status line, as the other buttons do.
            Err((StopCode::Dir1 | StopCode::Area1, reason)) => self.show_status_message(reason),
            // The rest also goes to the gauge, with its code.
            Err(refusal) => {
                self.show_status_message(refusal.1.clone());
                self.order_mut(of).pending = Some(Err(refusal));
            }
        }
    }

    /// Whether a press is still being worked on (the scripted screenshots wait for its end).
    pub(crate) fn topographic_download_busy(&self) -> bool {
        self.topo.is_busy() || self.dem.is_busy()
    }

    fn begin_ngii_run(&mut self, slot: &mut TopoDownload, of: OrderOf, frame: &eframe::Frame, request: TopoRequest, now: f64) {
        // MainWindowTopographic.cpp:25-33
        if std::fs::create_dir_all(&request.downloads).is_err() {
            let reason = format!("조사 폴더에 {} 폴더를 만들지 못했습니다. 저장 위치의 쓰기 권한과 공간을 확인한 뒤 다시 열어 주세요.", of.folder());
            self.download_jobs.add_job(of.job_title(), DownloadJobState::Stopped { code: StopCode::Dir2, reason }, now);
            return;
        }
        let first = DownloadJobState::Running { fraction: None, stage: Some("[1/6] 도엽 확인".to_owned()) };
        let (_, job) = self.download_jobs.add_job(of.job_title(), first.clone(), now);
        let config = AutoConfig { portal_url: PORTAL_HOME.to_owned(), script: of.script().to_owned(), download_directory: request.downloads, account: request.account, product: of.product(), test_mode: false, diagnostics: self.snap.is_some() };
        let options = RunOptions { data_parent: browser_data_parent(), config, network_allowed: Arc::new(strata_net::network::is_allowed) };
        match NgiiRun::start(frame, options) {
            Ok(mut run) => {
                run.prepare_sheets(request.scope);
                *slot = TopoDownload { run: Some(run), job: Some(job), shown: Some(first), ..TopoDownload::default() };
            }
            Err(_) => job.report_stopped(StopCode::App1, "보이지 않는 브라우저를 시작하지 못했습니다. Windows 의 WebView2 구성 요소를 확인한 뒤 다시 눌러 주세요."),
        }
    }

    /// Every frame, with the app's window at hand.
    pub(crate) fn drive_topographic_download(&mut self, frame: &eframe::Frame, now: f64) {
        for of in [OrderOf::Sheets, OrderOf::Dem] {
            // The slot is taken out while it is worked on, so that the rest of the app state stays at hand.
            let mut slot = std::mem::take(self.order_mut(of));
            self.drive_ngii_order(&mut slot, of, frame, now);
            *self.order_mut(of) = slot;
        }
    }

    fn drive_ngii_order(&mut self, slot: &mut TopoDownload, of: OrderOf, frame: &eframe::Frame, now: f64) {
        match slot.pending.take() {
            Some(Ok(request)) => self.begin_ngii_run(slot, of, frame, request, now),
            Some(Err((code, reason))) => drop(self.download_jobs.add_job(of.job_title(), DownloadJobState::Stopped { code, reason }, now)),
            None => {}
        }
        let Some(run) = slot.run.as_mut() else {
            return;
        };
        // The gauge's own cancel.
        if !slot.cancelled && slot.job.as_ref().is_some_and(DownloadJobHandle::is_cancelled) {
            slot.cancelled = true;
            run.stop();
        }
        for event in run.frame(frame) {
            if let AutoEvent::SheetDownloaded { path, record } = event {
                slot.received.push((path, record));
            }
        }
        let (sheets, cancelled) = (slot.received.len(), slot.cancelled);
        let state = run.with_state(|m| {
            let (_, received, total) = m.current_file();
            gauge_state(&RunFacts { stage: m.stage(), phase: m.phase(), attention: m.attention(), failure_phase: m.failure_phase(), status: m.status(), received, total, sheets, cancelled })
        });
        // A run that came to its end goes on to its sixth phase: what was received is put on the map.
        let goes_on = run.stage() == Stage::Complete && !cancelled;
        let state = if goes_on { DownloadJobState::Running { fraction: None, stage: Some("[6/6] 지도에 올리기".to_owned()) } } else { state };
        let over = matches!(run.stage(), Stage::Complete | Stage::Stopped);
        if slot.shown.as_ref() != Some(&state) {
            if let Some(job) = &slot.job {
                report(job, &state);
            }
            // A scripted run has nobody watching the gauge: its lines go to the log (phases and the
            // site's own words only; where the pages stand is named without query or form values).
            if self.snap.is_some() {
                eprintln!("[SNAP] {} {state:?} | {} | {}", of.log_name(), run.page_outline(), run.with_state(|m| m.last_diagnostic().to_owned()));
            }
            slot.shown = Some(state);
        }
        // The session is closed and its folder removed over the next frames; then the run is let go.
        if over && run.finished() {
            if self.snap.is_some() {
                let files: Vec<_> = slot.received.iter().map(|(path, _)| path.display().to_string()).collect();
                eprintln!("[SNAP] {} 끝: 받은 도엽 자료 {} — {files:?}", of.log_name(), files.len());
                eprintln!("[SNAP] {} 경과: {}", of.log_name(), run.with_state(|m| m.answer_trace().join(" | ")));
                eprintln!("[SNAP] {} 브라우저: {}", of.log_name(), run.reported().join(" | "));
            }
            let portal: Vec<_> = run.with_state(|m| m.portal_records().iter().filter_map(serde_json::Value::as_object).cloned().collect());
            slot.run = None;
            let (job, received) = (slot.job.take(), std::mem::take(&mut slot.received));
            if let (true, Some(job)) = (goes_on, job) {
                slot.post = match of {
                    OrderOf::Sheets => self.begin_topographic_post(received, portal, job),
                    OrderOf::Dem => self.begin_dem_post(received, job),
                };
            }
        }
    }
}
