//! Pure download job types, stop codes, and thread-safe worker handles
//! (`docs/RUST_PLAN.md` §2.8; slim bar precedent in `target/cpp-ref/src/app/KaProgressPlate.cpp:15-24`
//! and background download note in `target/cpp-ref/src/app/KaDownloadUi.cpp:51-55`).

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Weak};

#[path = "download_jobs_list.rs"]
mod download_jobs_list;
pub use download_jobs_list::DownloadJobList;

/// Seconds a `Done` job remains visible before leaving the list.
pub const DONE_LINGER_SECS: f64 = 8.0;
/// Seconds a `Cancelled` job remains visible before leaving the list.
pub const CANCELLED_LINGER_SECS: f64 = 6.0;
/// Stop reason shown when a worker exits or panics without reporting a final state (order DL1b).
pub const APP1_STOP_REASON: &str = "받기 작업이 중간에 끝났습니다. 다시 시도하세요.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownloadJobId(pub u64);

/// User-actionable stop codes (`docs/RUST_PLAN.md` §2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCode {
    /// Target or survey directory is missing (`DIR-1`).
    Dir1,
    /// Target directory cannot be written or disk space is insufficient (`DIR-2`).
    Dir2,
    /// Required survey area or map extent is missing or invalid (`AREA-1`).
    Area1,
    /// Requested area or tile count exceeds the service limit (`AREA-2`).
    Area2,
    /// Required API key is not configured (`KEY-1`).
    Key1,
    /// API key was rejected or lacks service permission (`KEY-2`).
    Key2,
    /// Required portal account credentials are not configured (`ACC-1`).
    Acc1,
    /// Portal login was rejected or requires manual account action (`ACC-2`).
    Acc2,
    /// Upstream site layout or flow changed so automation cannot proceed (`SITE-1`).
    Site1,
    /// Downloaded dataset is unreadable or fails format verification (`DATA-1`).
    Data1,
    /// Informational: no data exists for the requested area (`NONE-1`).
    None1,
    /// Worker ended (dropped handle or panic) before reporting a final state (`APP-1`).
    App1,
}

impl StopCode {
    pub const ALL: [Self; 12] = [
        Self::Dir1,
        Self::Dir2,
        Self::Area1,
        Self::Area2,
        Self::Key1,
        Self::Key2,
        Self::Acc1,
        Self::Acc2,
        Self::Site1,
        Self::Data1,
        Self::None1,
        Self::App1,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dir1 => "DIR-1",
            Self::Dir2 => "DIR-2",
            Self::Area1 => "AREA-1",
            Self::Area2 => "AREA-2",
            Self::Key1 => "KEY-1",
            Self::Key2 => "KEY-2",
            Self::Acc1 => "ACC-1",
            Self::Acc2 => "ACC-2",
            Self::Site1 => "SITE-1",
            Self::Data1 => "DATA-1",
            Self::None1 => "NONE-1",
            Self::App1 => "APP-1",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == code)
    }

    /// `NONE-1` is shown as information rather than a failure.
    #[must_use]
    pub const fn is_info(self) -> bool {
        matches!(self, Self::None1)
    }
}

impl fmt::Display for StopCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadJobState {
    Waiting,
    Running { fraction: Option<f32>, stage: Option<String> },
    Recovering { attempt: u32, reason: String, next_attempt_at: f64 },
    Done { result: String },
    Stopped { code: StopCode, reason: String },
    Cancelled,
}

#[derive(Debug, Clone)]
pub enum DownloadJobUpdate {
    Waiting,
    Running { fraction: Option<f32>, stage: Option<String> },
    Recovering { attempt: u32, reason: String, next_attempt_at: f64 },
    Done { result: String },
    Stopped { code: StopCode, reason: String },
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub id: DownloadJobId,
    pub title: String,
    pub state: DownloadJobState,
    pub(super) run_seq: u64,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) settled_at: Option<f64>,
    pub(super) worker_alive: Weak<()>,
}

impl DownloadJob {
    #[must_use]
    pub fn is_cancel_requested(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_unfinished(&self) -> bool {
        matches!(
            self.state,
            DownloadJobState::Waiting
                | DownloadJobState::Running { .. }
                | DownloadJobState::Recovering { .. }
        )
    }
}

#[derive(Clone)]
pub(super) struct JobClock(pub(super) Arc<dyn Fn() -> f64 + Send + Sync>);

impl JobClock {
    pub(super) fn now(&self) -> f64 {
        let t = (self.0)();
        if t.is_finite() { t } else { 0.0 }
    }
}

impl fmt::Debug for JobClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JobClock(..)")
    }
}

/// Worker-side handle for reporting progress, recovery, and completion of a download job.
///
/// Dropping every clone of a job's `DownloadJobHandle` while the job is still unfinished
/// ends the job with `Stopped { code: StopCode::App1, .. }` (`APP-1`) on a subsequent
/// [`DownloadJobList::tick`]. A download worker must keep its handle alive until after
/// sending its final report (`report_done`, `report_stopped`, or `report_cancelled`).
#[derive(Debug, Clone)]
pub struct DownloadJobHandle {
    pub(super) id: DownloadJobId,
    pub(super) run_seq: u64,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) tx: Sender<(DownloadJobId, u64, DownloadJobUpdate)>,
    pub(super) clock: JobClock,
    pub(super) alive: Arc<()>,
}

impl DownloadJobHandle {
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn send(&self, update: DownloadJobUpdate) {
        let _ = self.tx.send((self.id, self.run_seq, update));
    }

    pub fn report_running(&self, fraction: Option<f32>, stage: Option<String>) {
        self.send(DownloadJobUpdate::Running { fraction, stage });
    }

    pub fn report_recovering(&self, attempt: u32, reason: impl Into<String>, retry_in_secs: f64) {
        self.report_recovering_at(attempt, reason, self.clock.now(), retry_in_secs);
    }

    pub fn report_recovering_at(
        &self,
        attempt: u32,
        reason: impl Into<String>,
        reported_at: f64,
        retry_in_secs: f64,
    ) {
        let next_attempt_at = reported_at + retry_in_secs.max(0.0);
        self.send(DownloadJobUpdate::Recovering {
            attempt,
            reason: reason.into(),
            next_attempt_at,
        });
    }

    pub fn report_done(&self, result: impl Into<String>) {
        self.send(DownloadJobUpdate::Done { result: result.into() });
    }

    pub fn report_stopped(&self, code: StopCode, reason: impl Into<String>) {
        self.send(DownloadJobUpdate::Stopped { code, reason: reason.into() });
    }

    pub fn report_cancelled(&self) {
        self.send(DownloadJobUpdate::Cancelled);
    }
}

#[cfg(test)]
#[path = "download_jobs_tests.rs"]
mod tests;
