//! Parallel HTTP/1.1 keep-alive batch file downloader backed by a single `curl` process.

use std::{
    ffi::OsStr, fmt, path::PathBuf, process::Child,
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, mpsc::{Receiver, RecvTimeoutError}, Arc, Mutex},
    thread::JoinHandle, time::Duration,
};

use crate::curl_batch_io::{cleanup_parts, finish_child, start_with_program_checked};
#[cfg(test)]
pub(crate) use crate::curl_batch_io::{
    build_batch_config, build_curl_command, normalize_match_path, part_path, run_reader,
};

/// Maximum number of parallel connections allowed for a batch (`1..=MAX_PARALLEL`).
pub const MAX_PARALLEL: usize = 6;
/// Sentinel exit code when `curl` produced no per-item record and no non-zero process exit code.
pub const EXIT_NO_RECORD: i32 = -1;
/// Sentinel exit code when HTTP 200 arrived but renaming `<destination>.part` failed.
pub const EXIT_RENAME_FAILED: i32 = -2;
pub(crate) const RECORD_PREFIX: &str = "STRATA_BATCH\t";

/// One file transfer in a batch: request URL and final destination path.
#[derive(Clone, PartialEq, Eq)]
pub struct BatchItem {
    pub url: String,
    pub destination: PathBuf,
}

impl BatchItem {
    #[must_use]
    pub fn new(url: impl Into<String>, destination: impl Into<PathBuf>) -> Self {
        Self { url: url.into(), destination: destination.into() }
    }
}

impl fmt::Debug for BatchItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BatchItem").field("url", &"<redacted>").field("destination", &self.destination).finish()
    }
}

/// Outcome of one finished file transfer in the batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchResult {
    pub index: usize,
    pub http_status: u16,
    pub exit_code: i32,
}

impl BatchResult {
    /// Returns `true` only when HTTP 200 arrived and `<destination>.part` was renamed into place.
    #[must_use]
    pub const fn is_saved(&self) -> bool { self.http_status == 200 && self.exit_code == 0 }
}

/// Terminal or active state of a [`BatchHandle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchEnd {
    Running,
    Normal,
    Incomplete { missing_records: usize },
    Cancelled,
    ReaderPanicked { missing_records: usize },
}

/// Error returned when a batch cannot be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchError {
    InvalidParallel(usize),
    InvalidInput(&'static str),
    DuplicateDestination { index: usize, first_index: usize },
    LaunchBlocked(String),
    CreateDirFailed(String),
    CurlStartFailed(String),
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParallel(n) => write!(f, "parallel connections must be in 1..={MAX_PARALLEL}, got {n}"),
            Self::InvalidInput(msg) => f.write_str(msg),
            Self::DuplicateDestination { index, first_index } => {
                write!(f, "duplicate destination at item index {index} (same as index {first_index})")
            }
            Self::LaunchBlocked(reason) => write!(f, "batch launch blocked: {reason}"),
            Self::CreateDirFailed(err) => write!(f, "failed to create destination folder: {err}"),
            Self::CurlStartFailed(err) => write!(f, "failed to start curl: {err}"),
        }
    }
}

impl std::error::Error for BatchError {}

/// Timeout elapsed while waiting for the next batch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTimeout;

pub(crate) struct SharedState {
    pub(crate) cancelled: AtomicBool,
    pub(crate) reader_panicked: AtomicBool,
    pub(crate) worker_done: AtomicBool,
    pub(crate) missing_records: AtomicUsize,
    pub(crate) pending_results: AtomicUsize,
    pub(crate) child: Mutex<Option<Child>>,
    pub(crate) destinations: Vec<PathBuf>,
}

impl SharedState {
    pub(crate) fn new(destinations: Vec<PathBuf>, child: Option<Child>, done: bool) -> Arc<Self> {
        Arc::new(Self {
            cancelled: AtomicBool::new(false), reader_panicked: AtomicBool::new(false),
            worker_done: AtomicBool::new(done), missing_records: AtomicUsize::new(0),
            pending_results: AtomicUsize::new(0), child: Mutex::new(child), destinations,
        })
    }
}

/// Running batch handle that streams per-file results and cancels on drop.
pub struct BatchHandle {
    pub(crate) shared: Arc<SharedState>,
    pub(crate) rx: Mutex<Receiver<BatchResult>>,
    pub(crate) worker: Mutex<Option<JoinHandle<()>>>,
    #[cfg(test)]
    pub(crate) pid: Option<u32>,
}

impl fmt::Debug for BatchHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BatchHandle").field("items", &self.shared.destinations.len()).field("is_done", &self.is_done()).finish()
    }
}

impl BatchHandle {
    /// Returns the next finished transfer if one is already available.
    #[must_use]
    pub fn try_recv(&self) -> Option<BatchResult> {
        let res = self.rx.lock().ok()?.try_recv().ok()?;
        self.shared.pending_results.fetch_sub(1, Ordering::AcqRel);
        Some(res)
    }

    /// Waits up to `timeout` for the next finished transfer (`Ok(None)` when the batch has ended).
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<BatchResult>, WaitTimeout> {
        match self.rx.lock().map_err(|_| WaitTimeout)?.recv_timeout(timeout) {
            Ok(res) => { self.shared.pending_results.fetch_sub(1, Ordering::AcqRel); Ok(Some(res)) }
            Err(RecvTimeoutError::Disconnected) => Ok(None),
            Err(RecvTimeoutError::Timeout) => Err(WaitTimeout),
        }
    }

    /// Returns `true` once the batch process has ended and all buffered results have been read.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.shared.worker_done.load(Ordering::Acquire) && self.shared.pending_results.load(Ordering::Acquire) == 0
    }

    /// Returns how the batch ended (or [`BatchEnd::Running`] while still active).
    #[must_use]
    pub fn end_state(&self) -> BatchEnd {
        if self.shared.cancelled.load(Ordering::Acquire) && self.shared.worker_done.load(Ordering::Acquire) {
            return BatchEnd::Cancelled;
        }
        if !self.is_done() {
            return BatchEnd::Running;
        }
        let missing = self.shared.missing_records.load(Ordering::Acquire);
        if self.shared.reader_panicked.load(Ordering::Acquire) {
            BatchEnd::ReaderPanicked { missing_records: missing }
        } else if missing > 0 {
            BatchEnd::Incomplete { missing_records: missing }
        } else {
            BatchEnd::Normal
        }
    }

    /// Returns `true` only when all items received `curl` completion records and were drained.
    #[must_use]
    pub fn ended_normally(&self) -> bool { self.end_state() == BatchEnd::Normal }

    /// Stops the batch immediately, kills `curl`, and deletes any remaining `.part` files.
    pub fn cancel(&self) {
        self.shared.cancelled.store(true, Ordering::Release);
        let _ = finish_child(&self.shared, true);
        if let Some(handle) = self.worker.lock().ok().and_then(|mut g| g.take()) { let _ = handle.join(); }
        cleanup_parts(&self.shared.destinations);
        if let Ok(rx) = self.rx.lock() {
            while rx.try_recv().is_ok() { self.shared.pending_results.fetch_sub(1, Ordering::AcqRel); }
        }
        self.shared.worker_done.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn pid(&self) -> Option<u32> { self.pid }
}

impl Drop for BatchHandle {
    fn drop(&mut self) { self.cancel(); }
}

/// Default pre-spawn gate used by [`start`] (one-line connection point for `NET0c3`).
pub fn default_allow_launch(_urls: &[&str]) -> Result<(), String> { Ok(()) }

/// Starts downloading `items` after asking `allow_launch` whether the batch URLs may be fetched.
pub fn start_checked<F>(items: &[BatchItem], headers: &[(&str, &str)], timeout_per_file: Duration, parallel: usize, allow_launch: F) -> Result<BatchHandle, BatchError>
where F: FnOnce(&[&str]) -> Result<(), String> {
    start_with_program_checked(OsStr::new("curl"), items, headers, timeout_per_file, parallel, allow_launch)
}

/// Starts downloading `items` with one `curl` process over `1..=6` reused parallel connections.
pub fn start(items: &[BatchItem], headers: &[(&str, &str)], timeout_per_file: Duration, parallel: usize) -> Result<BatchHandle, BatchError> {
    start_checked(items, headers, timeout_per_file, parallel, default_allow_launch)
}

#[cfg(test)]
#[path = "curl_batch_a_tests.rs"]
mod tests_a;
#[cfg(test)]
#[path = "curl_batch_b_tests.rs"]
mod tests_b;
#[cfg(test)]
#[path = "curl_batch_c_tests.rs"]
mod tests_c;
