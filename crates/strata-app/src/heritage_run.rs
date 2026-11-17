//! The 「유산」 button (`target/cpp-ref/src/app/MainWindow.cpp:3071-3225` fetchNearbyHeritage): the heritage
//! data of the city of the survey becomes read-only rows under 참조 지도, one for every shapefile of a kind,
//! holding the sites within 5 km of the survey area. `heritage_reuse.rs` reads recent archives;
//! `heritage_download.rs` owns the hidden runner, imports and the single bottom gauge.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

use chrono::NaiveDateTime;
use strata_engine::Scope;
use strata_refmap::heritage::{HeritageDataset, DATASETS, NEAR_M};
use strata_refmap::heritage_load::{DatasetLoad, LoadFailure};

use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobHandle, DownloadJobState, StopCode};
use crate::survey_area_setup::is_survey_area_key;

#[path = "heritage_resolve.rs"]
pub(crate) mod resolve;
#[path = "heritage_reuse.rs"]
pub(crate) mod reuse;
#[path = "heritage_run_layers.rs"]
mod layers;
#[path = "heritage_download.rs"]
pub(crate) mod download;
#[cfg(test)]
#[path = "heritage_run_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "heritage_lifecycle_tests.rs"]
pub(crate) mod lifecycle_tests;

/// The rows of the button: `ref_heritage:<key of the kind>:<title>`.
pub(crate) const KEY_PREFIX: &str = "ref_heritage:";
pub(crate) const HERITAGE_JOB_TITLE: &str = "주변유적";
/// Why a press is stopped (`MainWindow.cpp:3082-3099`).
const NO_AREA_LAYER: &str = "조사구역 레이어가 없습니다. 조사구역을 먼저 그리세요.";
const EMPTY_AREA: &str = "조사구역이 비어 있습니다. 조사구역을 먼저 그리세요. 그린 구역으로 어느 시·군인지 판정합니다.";
const NO_SURVEY: &str = "조사를 먼저 열거나 저장하세요. 받은 자료는 그 조사폴더 안에만 둡니다.";
/// The end of a run (`:3143`).
const DONE: &str = "주변유적 자료 처리를 마쳤습니다. 받은 자료는 「참조 지도」에 있습니다.";
/// This app's own words, until the run on the intranet is built: the C++ receives these kinds there.
const NOT_RECEIVED: &str = "받아 둔 자료가 없는 종류";

/// What the worker tells the window.
pub(crate) enum Step {
    Kind(&'static HeritageDataset, Result<DatasetLoad, LoadFailure>),
    /// Every kind is through; `missing` names the kinds that were not there to use again.
    End { missing: Vec<&'static str> },
}

/// A run of the button and what it waits for.
#[derive(Default)]
pub(crate) struct HeritageRun {
    /// Where the received archives are kept; `None` is the app's own data folder.
    pub(crate) archive_root: Option<PathBuf>,
    pub(crate) refused: Option<(StopCode, String)>,
    pub(crate) wanted: Option<reuse::Wanted>,
    pub(crate) running: Option<(Receiver<Step>, DownloadJobHandle)>,
    /// The question about the city of the survey area, while it is open.
    pub(crate) resolving: Option<Receiver<Result<Vec<u8>, strata_refmap::transfer::TransferFailure>>>,
    /// Why no city could be given, for the dialog.
    pub(crate) region_note: Option<String>,
    /// The city the dialog last asked about, and when its newest usable data was received.
    pub(crate) asked: Option<((String, String), Option<NaiveDateTime>)>,
    /// The hidden-browser run, once a 「받기」 press has started one
    /// (`heritage_run_start.rs`). `None` until then, and again when it ends.
    #[cfg(windows)]
    pub(crate) browser_run: Option<strata_heritage_run::HeritageRunHandle>,
    pub(crate) download: Option<download::Download>,
    /// Capture the real pending fixture run, without waiting for its end (HER-CODEX-UI1).
    pub(crate) snap_capture_pending: bool,
}

impl std::fmt::Debug for HeritageRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HeritageRun")
    }
}

/// A copy of the app state does not take the run with it.
impl Clone for HeritageRun {
    fn clone(&self) -> Self {
        Self { archive_root: self.archive_root.clone(), ..Self::default() }
    }
}

impl HeritageRun {
    /// Whether a press is still being worked on (the scripted screenshots wait for its end).
    pub(crate) fn is_busy(&self) -> bool {
        self.refused.is_some() || self.wanted.is_some() || self.running.is_some() || self.resolving.is_some() || self.download.is_some() || self.browser_busy()
    }

    pub(crate) fn browser_busy(&self) -> bool {
        #[cfg(windows)]
        { self.browser_run.is_some() }
        #[cfg(not(windows))]
        { false }
    }
}

/// The kind a row of the button belongs to.
pub(crate) fn dataset_of_key(key: &str) -> Option<&'static HeritageDataset> {
    let kind = key.strip_prefix(KEY_PREFIX)?.split(':').next()?;
    DATASETS.iter().find(|dataset| dataset.key == kind)
}

/// The colour of the kind a row of the button belongs to.
pub(crate) fn colour_of_key(key: &str) -> Option<[u8; 3]> {
    dataset_of_key(key).map(|dataset| dataset.colour)
}

impl StrataAppState {
    /// What a run needs before anything else: the survey area grown by 5 km (`:3082-3099`).
    pub(crate) fn heritage_scope(&self) -> Result<Scope, (StopCode, String)> {
        if !self.survey_tables.iter().any(|table| is_survey_area_key(&table.table)) {
            return Err((StopCode::Area1, NO_AREA_LAYER.to_owned()));
        }
        let scope = match crate::ref_vector::map::survey_scope(self, NEAR_M) {
            Ok(Some(scope)) => scope,
            Ok(None) => return Err((StopCode::Area1, EMPTY_AREA.to_owned())),
            Err(reason) => return Err((StopCode::Area1, reason)),
        };
        if crate::survey_folder::survey_folder(self.survey_path.as_deref()).is_none() {
            return Err((StopCode::Dir1, NO_SURVEY.to_owned()));
        }
        Ok(scope)
    }

    /// The press: with a survey area in an open survey, the dialog asks for the city.
    pub(crate) fn press_heritage(&mut self) {
        if self.heritage.download.is_some() || self.heritage.browser_busy() || self.heritage.running.is_some() || self.heritage.wanted.is_some() {
            return;
        }
        match self.heritage_scope() {
            Ok(_) => self.begin_heritage_region(),
            // The reason stands on the status line at once and on the gauge with its code.
            Err((code, reason)) => {
                self.show_status_message(reason.clone());
                self.heritage.refused = Some((code, reason));
            }
        }
    }

    /// Every frame: what a press or the dialog wished for becomes a line of the gauge.
    pub(crate) fn drive_heritage_wishes(&mut self, now: f64) {
        if let Some((code, reason)) = self.heritage.refused.take() {
            drop(self.download_jobs.add_job(HERITAGE_JOB_TITLE, DownloadJobState::Stopped { code, reason }, now));
        }
        if let Some(wanted) = self.heritage.wanted.take() {
            self.begin_heritage_run(&wanted, now);
        }
    }

    /// Every frame: the wishes, and the words of a running worker.
    ///
    /// `frame` is needed only when a 「받기」 press has to open the browser session, which a test
    /// cannot do; [`Self::drive_heritage_no_window`] drives the testable app state.
    pub(crate) fn drive_heritage(&mut self, ctx: &eframe::egui::Context, frame: &eframe::Frame, now: f64) {
        self.drive_heritage_region(ctx);
        self.drive_heritage_wishes(now);
        self.drive_heritage_download(ctx);
        // A 「받기」 press asked for a run; the window handle exists only here
        // (`heritage_run_start.rs`).
        if self.heritage_run_requested {
            self.heritage_run_requested = false;
            crate::heritage_run_start::begin_run_in_window(self, frame);
        }
        self.drive_heritage_steps(ctx);
        #[cfg(windows)]
        self.drive_heritage_browser(frame);
        self.drive_heritage_download(ctx);
    }

    /// Everything the frame loop does except opening a browser session: a test calls this,
    /// because a test has no window to give a hidden view.
    #[cfg(test)]
    pub(crate) fn drive_heritage_no_window(&mut self, ctx: &eframe::egui::Context, now: f64) {
        self.drive_heritage_region(ctx);
        self.drive_heritage_wishes(now);
        self.drive_heritage_steps(ctx);
        self.drive_heritage_download(ctx);
    }

    /// Feed the hidden session and consume only explicit engine facts (HER-API1).
    #[cfg(windows)]
    pub(crate) fn drive_heritage_browser(&mut self, frame: &eframe::Frame) {
        let Some(run) = self.heritage.browser_run.as_mut() else { return; };
        let events = run.frame(frame);
        let finished = run.finished();
        for event in events { self.take_heritage_event(event); }
        if finished {
            self.heritage.browser_run = None;
            self.heritage_browser_ended();
        }
    }

    /// Reads what the worker thread sent, until it has nothing left.
    fn drive_heritage_steps(&mut self, ctx: &eframe::egui::Context) {
        loop {
            let Some((steps, job)) = self.heritage.running.as_ref() else {
                return;
            };
            let job = job.clone();
            if job.is_cancelled() {
                job.report_cancelled();
                self.heritage.running = None;
                return;
            }
            match steps.try_recv() {
                Ok(step) => {
                    if self.take_heritage_step(&job, step) {
                        self.heritage.running = None;
                    }
                }
                Err(TryRecvError::Empty) => {
                    // Nothing wakes the window while the thread works.
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                    return;
                }
                // The worker ended without its last word: the gauge turns the job into APP-1 by itself.
                Err(TryRecvError::Disconnected) => self.heritage.running = None,
            }
        }
    }

    /// A run belongs to the survey it was started for: when another survey takes its place it is told to
    /// stop and its words are not waited for.
    pub(crate) fn drop_heritage_run(&mut self) {
        self.heritage_run_requested = false;
        self.cancel_heritage_download();
        (self.heritage.wanted, self.heritage.refused, self.heritage.resolving) = (None, None, None);
        if let Some((_, job)) = self.heritage.running.take() {
            self.download_jobs.request_cancel(job.id());
            job.report_cancelled();
        }
    }
}
