//! The running side of a reference-map button: the press is checked, the map is fetched in a thread, the
//! bottom gauge follows it, and the legend row appears when the file is there
//! (`target/cpp-ref/src/app/MainWindowOverlay.cpp:221-236`, `:405-472`, `:529-680`).

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use strata_engine::Scope;
use strata_refmap::transfer::{Prepared, Request, Stopped, TransferFailure, Why};
use strata_types::crs::CrsCode;

use super::{kept_map, survey_scope, write_scope_note, RefMapKind, BUSY};
use crate::app_state::{MainTab, StrataAppState};
use crate::download_jobs::{DownloadJobHandle, DownloadJobState, StopCode};
use crate::ref_vector::data::LoadSpec;

/// Sends one request to a map service.
type Fetch = Arc<dyn Fn(&Request) -> Result<Vec<u8>, TransferFailure> + Send + Sync>;
/// Runs work off the UI thread.
type Runner = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// A press that passed its checks and waits for the next frame.
struct Wanted {
    kind: &'static RefMapKind,
    scope: Scope,
    requested: PathBuf,
}

struct Running {
    kind: &'static RefMapKind,
    job: DownloadJobHandle,
    result: Receiver<Result<Prepared, Stopped>>,
}

/// The reference-map fetches of the app: at most one runs.
pub(crate) struct RefMapJobs {
    wanted: Option<Wanted>,
    /// A press that cannot start, for the gauge: its job title, code and words.
    refused: Option<(&'static str, StopCode, &'static str)>,
    running: Option<Running>,
    fetch: Fetch,
    runner: Runner,
    online: fn() -> bool,
}

impl Default for RefMapJobs {
    fn default() -> Self {
        let fetch: Fetch = Arc::new(|request| {
            strata_net::reference_http::reference_get(&request.url, request.user_agent, request.idle_secs, request.total_secs).map_err(|failure| match failure {
                strata_net::reference_http::ReferenceFailure::Timeout => TransferFailure::Timeout,
                strata_net::reference_http::ReferenceFailure::Network => TransferFailure::Network,
            })
        });
        Self { wanted: None, refused: None, running: None, fetch, runner: Arc::new(|work| drop(std::thread::spawn(work))), online: strata_net::network::is_allowed }
    }
}

impl std::fmt::Debug for RefMapJobs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefMapJobs").field("running", &self.running.as_ref().map(|r| r.kind.job_title)).finish()
    }
}

/// A copy of the app state carries no fetch.
impl Clone for RefMapJobs {
    fn clone(&self) -> Self {
        Self { wanted: None, refused: None, running: None, fetch: Arc::clone(&self.fetch), runner: Arc::clone(&self.runner), online: self.online }
    }
}

impl RefMapJobs {
    /// Whether a press is still being worked on (the scripted screenshots wait for its end).
    pub(crate) fn is_busy(&self) -> bool {
        self.wanted.is_some() || self.refused.is_some() || self.running.is_some()
    }
}

/// The gauge code of each kind of stop (docs/RUST_PLAN.md 2.8: one fixed code for each reason).
fn stop_code(why: Why) -> StopCode {
    match why {
        Why::Storage => StopCode::Dir2,
        Why::TooWide => StopCode::Area2,
        Why::NoData => StopCode::None1,
        Why::Service => StopCode::Site1,
        Why::Data => StopCode::Data1,
    }
}

impl StrataAppState {
    fn load_spec(&self, kind: &RefMapKind, file: PathBuf) -> LoadSpec {
        LoadSpec { file, table: kind.table.to_owned(), work_epsg: CrsCode::from(self.work_crs).epsg_number(), look: kind.look.clone(), label: kind.label }
    }

    /// The ribbon button of a reference map. With its row in the legend the press hides and shows the row
    /// (`toggleExistingOverlay`, `MainWindowOverlay.cpp:221-236`); without it the map is fetched, or a map
    /// kept for this very area is put on the map.
    pub(crate) fn press_reference_map(&mut self, kind: &'static RefMapKind) {
        self.active_tab = MainTab::Map;
        if let Some(name) = self.left_layers.entries.iter().find(|e| e.layer_key == kind.key).map(|e| e.label_ko.clone()) {
            let shown = self.home_map.real_survey.is_layer_visible(&name);
            self.home_map.real_survey.layer_visible.insert(name, !shown);
            self.show_status_message(format!("{}{}", kind.title, if shown { "를 껐습니다." } else { "를 다시 켰습니다." }));
            return;
        }
        if self.ref_maps.is_busy() {
            self.show_status_message(BUSY);
            return;
        }
        // MainWindowDownloads.cpp:93-103, MainWindowOverlay.cpp:509.
        let Some(folder) = crate::survey_folder::survey_folder(self.survey_path.as_deref()) else {
            self.show_status_message(kind.needs_survey);
            return;
        };
        let scope = match survey_scope(self, kind.radius_m) {
            Ok(Some(scope)) => scope,
            Ok(None) => return self.show_status_message(kind.needs_area),
            Err(reason) => return self.show_status_message(reason),
        };
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let stored = scope.reprojected(work_epsg, kind.stored_epsg).and_then(|s| Ok((s.bbox()?, s)));
        let Ok((bbox, scope)) = stored else {
            return self.show_status_message(super::AREA_CRS_UNKNOWN);
        };
        if let Some(file) = kept_map(&folder, kind, bbox) {
            self.show_reference_vector_layer(kind.key, kind.title, self.load_spec(kind, file));
            self.show_status_message(kind.kept);
            return;
        }
        if !(self.ref_maps.online)() {
            // What the user cannot mend on the map also goes to the gauge, with its code.
            self.show_status_message(kind.offline);
            self.ref_maps.refused = Some((kind.job_title, StopCode::Site1, kind.offline));
            return;
        }
        self.show_status_message(kind.fetching);
        self.ref_maps.wanted = Some(Wanted { kind, scope, requested: folder.join(kind.file_name) });
    }

    /// Every frame: starts a wanted fetch, and takes the result of a running one.
    pub(crate) fn drive_reference_maps(&mut self, ctx: &eframe::egui::Context, now: f64) {
        if let Some((title, code, reason)) = self.ref_maps.refused.take() {
            drop(self.download_jobs.add_job(title, DownloadJobState::Stopped { code, reason: reason.to_owned() }, now));
        }
        if let Some(Wanted { kind, scope, requested }) = self.ref_maps.wanted.take() {
            let first = DownloadJobState::Running { fraction: None, stage: Some(kind.fetching.to_owned()) };
            let (_, job) = self.download_jobs.add_job(kind.job_title, first, now);
            let (send, result) = channel();
            let (fetch, worker_job, wake) = (Arc::clone(&self.ref_maps.fetch), job.clone(), ctx.clone());
            (self.ref_maps.runner)(Box::new(move || {
                let outcome = (kind.prepare)(&scope, &requested, &|request| fetch(request), &|| worker_job.is_cancelled()).and_then(|prepared| {
                    // The note says for which area the map was fetched; without it the map would be fetched again.
                    let noted = scope.bbox().ok().and_then(|bbox| write_scope_note(&prepared.folder, kind.radius_m, bbox).ok());
                    noted.map(|()| prepared).ok_or_else(|| Stopped::Failed(Why::Storage, strata_refmap::transfer::NOT_SAVED.to_owned()))
                });
                let _ = send.send(outcome);
                wake.request_repaint();
            }));
            self.ref_maps.running = Some(Running { kind, job, result });
        }
        let Some(running) = self.ref_maps.running.as_ref() else {
            return;
        };
        let outcome = match running.result.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return,
            // The worker ended without an answer: the gauge turns the job into APP-1 by itself.
            Err(TryRecvError::Disconnected) => {
                self.ref_maps.running = None;
                return;
            }
        };
        let Some(Running { kind, job, .. }) = self.ref_maps.running.take() else {
            return;
        };
        if self.snap.is_some() {
            eprintln!("[SNAP] {} {:?}", kind.job_title, outcome.as_ref().map(|p| (p.rows, &p.warnings, p.gpkg.display().to_string())));
        }
        match outcome {
            Ok(prepared) => {
                self.show_reference_vector_layer(kind.key, kind.title, self.load_spec(kind, prepared.gpkg));
                job.report_done(kind.added);
                // What the C++ shows in a notice 「… 확인 사항」 (`MainWindowOverlay.cpp:674`) goes to the status line.
                self.show_status_message(if prepared.warnings.is_empty() { kind.added.to_owned() } else { prepared.warnings.join(" ") });
            }
            Err(Stopped::Cancelled(_)) => {
                job.report_cancelled();
                self.show_status_message(kind.cancelled);
            }
            Err(Stopped::Failed(why, reason)) => job.report_stopped(stop_code(why), reason),
        }
    }
}

#[cfg(test)]
impl RefMapJobs {
    /// Answers come from `fetch`, the network counts as on or off, and work runs inside the call that asks.
    pub(crate) fn for_test(fetch: Fetch, online: fn() -> bool) -> Self {
        Self { wanted: None, refused: None, running: None, fetch, runner: Arc::new(|work| work()), online }
    }

    /// Work is handed to `runner` instead of a thread.
    pub(crate) fn set_runner(&mut self, runner: Runner) {
        self.runner = runner;
    }
}
