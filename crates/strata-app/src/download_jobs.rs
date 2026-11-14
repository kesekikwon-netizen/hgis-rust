//! Pure download job types, stop codes, and thread-safe worker handles
//! (`docs/RUST_PLAN.md` §2.8; slim bar precedent in `target/cpp-ref/src/app/KaProgressPlate.cpp:15-24`
//! and background download note in `target/cpp-ref/src/app/KaDownloadUi.cpp:51-55`).

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[path = "download_jobs_list.rs"]
mod download_jobs_list;
pub use download_jobs_list::DownloadJobList;

/// Seconds a `Done` job remains visible before leaving the list.
pub const DONE_LINGER_SECS: f64 = 8.0;
/// Seconds a `Cancelled` job remains visible before leaving the list.
pub const CANCELLED_LINGER_SECS: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownloadJobId(pub u64);

/// User-actionable stop codes (`docs/RUST_PLAN.md` §2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCode {
    Dir1,
    Dir2,
    Area1,
    Area2,
    Key1,
    Key2,
    Acc1,
    Acc2,
    Site1,
    Data1,
    None1,
}

impl StopCode {
    pub const ALL: [Self; 11] = [
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
    Recovering { attempt: u32, reason: String, retry_in_secs: f64 },
    Done { result: String },
    Stopped { code: StopCode, reason: String },
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DownloadJob {
    pub id: DownloadJobId,
    pub title: String,
    pub state: DownloadJobState,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) settled_at: Option<f64>,
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

#[derive(Debug, Clone)]
pub struct DownloadJobHandle {
    pub(super) id: DownloadJobId,
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) tx: Sender<(DownloadJobId, DownloadJobUpdate)>,
}

impl DownloadJobHandle {
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn send(&self, update: DownloadJobUpdate) {
        let _ = self.tx.send((self.id, update));
    }

    pub fn report_running(&self, fraction: Option<f32>, stage: Option<String>) {
        self.send(DownloadJobUpdate::Running { fraction, stage });
    }

    pub fn report_recovering(&self, attempt: u32, reason: impl Into<String>, retry_in_secs: f64) {
        self.send(DownloadJobUpdate::Recovering { attempt, reason: reason.into(), retry_in_secs });
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
