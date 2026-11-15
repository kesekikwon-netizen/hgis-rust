//! After the 「수치지형」 run: the received sheets are listed, placed and merged off the UI thread, and the
//! merged lines become the legend row 「수치지형도」 (C++ `KaTopographicScopePanel.cpp` prepareSheets and
//! `KaTopographicImportDialog.cpp` publishPrepared `:871-995`; the phase is the sixth of the run's gauge
//! line, 「지도에 올리기」).

use std::sync::mpsc::{channel, Receiver, TryRecvError};

use serde_json::{Map, Value};
use strata_topo::merge::MergeStop;
use strata_topo::post::{post_process, Posted, Received};
use strata_types::crs::CrsCode;

use super::{TOPO_JOB_TITLE, TOPO_RADIUS_M};
use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobHandle, StopCode};
use crate::ref_vector::data::{LoadSpec, Look};

#[cfg(test)]
#[path = "topo_post_tests.rs"]
mod tests;

/// `layer_key` and title of the legend row (`KaTopographicImportDialog.cpp:936`).
pub(crate) const TOPO_KEY: &str = "ref_topo";
pub(crate) const TOPO_TITLE: &str = "수치지형도";
/// The end words of the run (docs/RUST_PLAN.md 4.3 F4).
pub(crate) const TOPO_READY: &str = "수치지형도를 준비했습니다.";
/// This app's own words: the C++ has no sentence for sheets that hold nothing near the survey.
const NO_LINES: &str = "조사구역 5km 안에 지도에 올릴 수치지형도 선이 없습니다.";

/// The work on what a run left, while it runs.
pub(crate) struct TopoPost {
    result: Receiver<Result<Posted, MergeStop>>,
    job: DownloadJobHandle,
}

impl std::fmt::Debug for TopoPost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TopoPost")
    }
}

impl StrataAppState {
    /// The outcome of the work: the row, the gauge line and the status line.
    pub(crate) fn finish_topographic_post(&mut self, job: &DownloadJobHandle, outcome: Result<Posted, MergeStop>) {
        if self.snap.is_some() {
            eprintln!("[SNAP] {TOPO_JOB_TITLE} 올리기 {outcome:?}");
        }
        let posted = match outcome {
            Ok(posted) => posted,
            Err(MergeStop::Cancelled) => return job.report_cancelled(),
            Err(MergeStop::Failed(reason)) => return job.report_stopped(StopCode::Data1, reason),
        };
        // `<도엽> · 검토 필요: <이유>` (KaTopographicScopePanel.cpp:247-249).
        let review: Vec<String> = posted.review.iter().map(|(num, reason)| format!("{num} · 검토 필요: {reason}")).collect();
        let Some(file) = posted.merged else {
            if review.is_empty() {
                job.report_done(NO_LINES);
                self.show_status_message(NO_LINES);
            } else {
                self.show_status_message(review.join(" "));
                job.report_stopped(StopCode::Data1, review.join(" "));
            }
            return;
        };
        // One read-only line layer under 참조 지도: grey 128, 0.2 mm, no labels (`:936-975`).
        let spec = LoadSpec { file, table: strata_topo::merge::TABLE.to_owned(), work_epsg: CrsCode::from(self.work_crs).epsg_number(), look: Look::Lines { stroke: [128, 128, 128, 255], width_mm: 0.2 }, label: None };
        self.show_reference_vector_layer(TOPO_KEY, TOPO_TITLE, spec);
        job.report_done(TOPO_READY);
        let mut said = vec![format!("{TOPO_READY} 도엽 {}장을 지도에 올렸습니다.", posted.placed.len())];
        said.extend(review);
        said.extend(posted.warnings);
        self.show_status_message(said.join(" "));
    }

    /// Starts the work on what a run left, in a thread: the sheets go into the survey's 지형도 folder list,
    /// every sheet of the folder is placed, and the lines within 5 km of the survey area are merged.
    pub(crate) fn begin_topographic_post(&mut self, received: Vec<(std::path::PathBuf, Map<String, Value>)>, portal: Vec<Map<String, Value>>, job: DownloadJobHandle) -> Option<TopoPost> {
        let Some(root) = crate::survey_folder::survey_folder(self.survey_path.as_deref()).map(|folder| folder.join("지형도")) else {
            job.report_stopped(StopCode::Dir1, "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 지형도 폴더에 저장합니다.");
            return None;
        };
        let Ok(Some(scope)) = crate::ref_vector::map::survey_scope(self, TOPO_RADIUS_M) else {
            job.report_stopped(StopCode::Area1, "조사구역을 먼저 그려 주세요. 조사구역에서 반경 5km 안의 수치지형도를 받습니다.");
            return None;
        };
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_millis());
        let received: Vec<Received> = received.into_iter().map(|(path, record)| Received { path, record }).collect();
        let (send, result) = channel();
        let worker_job = job.clone();
        std::thread::spawn(move || {
            let _ = send.send(post_process(&root, &received, &portal, &scope, work_epsg, stamp, &|| worker_job.is_cancelled()));
        });
        Some(TopoPost { result, job })
    }

    /// Every frame: takes the outcome of the work when it is there.
    pub(crate) fn drive_topographic_post(&mut self, ctx: &eframe::egui::Context) {
        let Some(post) = self.topo.post.as_ref() else {
            return;
        };
        let outcome = match post.result.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => {
                // Nothing wakes the window while the thread works.
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
                return;
            }
            // The worker ended without an answer: the gauge turns the job into APP-1 by itself.
            Err(TryRecvError::Disconnected) => {
                self.topo.post = None;
                return;
            }
        };
        if let Some(post) = self.topo.post.take() {
            self.finish_topographic_post(&post.job, outcome);
        }
    }

    /// An order and the work after it belong to the survey they were started for: when another survey
    /// takes its place they are told to stop and their outcome is not waited for.
    pub(crate) fn drop_topographic_post(&mut self) {
        for post in [self.topo.abandon(), self.dem.abandon()].into_iter().flatten() {
            self.download_jobs.request_cancel(post.job.id());
            post.job.report_cancelled();
        }
    }
}
