//! Parallel HTTP/1.1 keep-alive batch file downloader backed by a single `curl` process.

use std::{
    collections::{HashMap, VecDeque}, ffi::OsStr, fmt::{self, Write as _}, fs,
    io::{BufRead, BufReader, Write}, path::{Path, PathBuf}, process::{Child, ChildStderr, Command, Stdio},
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, mpsc::{self, Receiver, RecvTimeoutError, Sender}, Arc, Mutex},
    thread::{self, JoinHandle}, time::Duration,
};

/// Maximum number of parallel connections allowed for a batch (`1..=MAX_PARALLEL`).
pub const MAX_PARALLEL: usize = 6;
const RECORD_PREFIX: &str = "STRATA_BATCH\t";

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

/// Error returned when a batch cannot be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchError {
    InvalidParallel(usize),
    InvalidInput(&'static str),
    CreateDirFailed(String),
    CurlStartFailed(String),
}

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParallel(n) => write!(f, "parallel connections must be in 1..={MAX_PARALLEL}, got {n}"),
            Self::InvalidInput(msg) => f.write_str(msg),
            Self::CreateDirFailed(err) => write!(f, "failed to create destination folder: {err}"),
            Self::CurlStartFailed(err) => write!(f, "failed to start curl: {err}"),
        }
    }
}

impl std::error::Error for BatchError {}

/// Timeout elapsed while waiting for the next batch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTimeout;

struct SharedState {
    cancelled: AtomicBool,
    worker_done: AtomicBool,
    pending_results: AtomicUsize,
    child: Mutex<Option<Child>>,
    destinations: Vec<PathBuf>,
}

impl SharedState {
    fn new(destinations: Vec<PathBuf>, child: Option<Child>) -> Arc<Self> {
        let done = child.is_none();
        Arc::new(Self {
            cancelled: AtomicBool::new(false), worker_done: AtomicBool::new(done),
            pending_results: AtomicUsize::new(0), child: Mutex::new(child), destinations,
        })
    }
}

/// Running batch handle that streams per-file results and cancels on drop.
pub struct BatchHandle {
    shared: Arc<SharedState>,
    rx: Mutex<Receiver<BatchResult>>,
    worker: Mutex<Option<JoinHandle<()>>>,
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

    /// Stops the batch immediately, kills `curl`, and deletes any remaining `.part` files.
    pub fn cancel(&self) {
        self.shared.cancelled.store(true, Ordering::Release);
        reap_child(&self.shared);
        if let Some(handle) = self.worker.lock().ok().and_then(|mut g| g.take()) {
            let _ = handle.join();
        }
        cleanup_parts(&self.shared.destinations);
        if let Ok(rx) = self.rx.lock() {
            while rx.try_recv().is_ok() {
                self.shared.pending_results.fetch_sub(1, Ordering::AcqRel);
            }
        }
        self.shared.worker_done.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn is_process_running(&self) -> bool {
        self.shared.child.lock().ok().and_then(|mut g| g.as_mut().map(|c| matches!(c.try_wait(), Ok(None)))).unwrap_or(false)
    }
}

impl Drop for BatchHandle {
    fn drop(&mut self) { self.cancel(); }
}

pub(crate) fn part_path(destination: &Path) -> PathBuf {
    let mut os = destination.as_os_str().to_os_string();
    os.push(".part");
    PathBuf::from(os)
}

fn escape_quoted(raw: &str) -> String { raw.replace('\\', "\\\\").replace('"', "\\\"") }

pub(crate) fn build_curl_command(program: &OsStr) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(["--config", "-"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub(crate) fn build_batch_config(items: &[BatchItem], headers: &[(&str, &str)], timeout_per_file: Duration, parallel: usize) -> Result<String, BatchError> {
    if !(1..=MAX_PARALLEL).contains(&parallel) {
        return Err(BatchError::InvalidParallel(parallel));
    }
    if timeout_per_file.is_zero() {
        return Err(BatchError::InvalidInput("timeout_per_file must be positive"));
    }
    let (mut cfg, secs) = (String::new(), timeout_per_file.as_secs_f64().max(0.05));
    let _ = writeln!(cfg, "silent\nlocation\nparallel\nparallel-immediate\nparallel-max = {parallel}\nmax-time = {secs:.3}");
    for &(name, value) in headers {
        if name.trim().is_empty() || name.contains(['\r', '\n', '\0']) || value.contains(['\r', '\n', '\0']) {
            return Err(BatchError::InvalidInput("invalid request header"));
        }
        let _ = writeln!(cfg, "header = \"{}\"", escape_quoted(&format!("{name}: {value}")));
    }
    let _ = writeln!(cfg, "write-out = \"%{{stderr}}{RECORD_PREFIX}%{{http_code}}\\t%{{exitcode}}\\t%{{filename_effective}}\\n\"");
    for item in items {
        if item.url.is_empty() || item.url.contains(['\r', '\n', '\0']) {
            return Err(BatchError::InvalidInput("invalid item url"));
        }
        let part = part_path(&item.destination);
        let Some(part_str) = part.to_str() else { return Err(BatchError::InvalidInput("destination path is not valid UTF-8")) };
        if item.destination.as_os_str().is_empty() || part_str.contains(['\r', '\n', '\0']) {
            return Err(BatchError::InvalidInput("invalid destination path"));
        }
        let _ = writeln!(cfg, "url = \"{}\"\noutput = \"{}\"", escape_quoted(&item.url), escape_quoted(part_str));
    }
    Ok(cfg)
}

/// Single launch point for the `curl` batch process (hook point for the crate-wide offline switch).
fn spawn_curl_process(program: &OsStr, config: Vec<u8>) -> Result<(Child, ChildStderr), BatchError> {
    let mut child = build_curl_command(program).spawn().map_err(|e| BatchError::CurlStartFailed(e.to_string()))?;
    let (Some(stderr), Some(mut stdin)) = (child.stderr.take(), child.stdin.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(BatchError::CurlStartFailed("missing curl stdio pipe".to_owned()));
    };
    thread::spawn(move || { let _ = stdin.write_all(&config); });
    Ok((child, stderr))
}

pub(crate) fn start_with_program(program: &OsStr, items: &[BatchItem], headers: &[(&str, &str)], timeout_per_file: Duration, parallel: usize) -> Result<BatchHandle, BatchError> {
    let config = build_batch_config(items, headers, timeout_per_file, parallel)?;
    let destinations: Vec<PathBuf> = items.iter().map(|i| i.destination.clone()).collect();
    let mut by_part: HashMap<String, VecDeque<usize>> = HashMap::with_capacity(destinations.len());
    for (idx, dest) in destinations.iter().enumerate() {
        if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| BatchError::CreateDirFailed(e.to_string()))?;
        }
        let part = part_path(dest);
        let _ = fs::remove_file(&part);
        by_part.entry(part.to_string_lossy().replace('\\', "/")).or_default().push_back(idx);
    }
    let (tx, rx) = mpsc::channel();
    if items.is_empty() {
        drop(tx);
        return Ok(BatchHandle { shared: SharedState::new(destinations, None), rx: Mutex::new(rx), worker: Mutex::new(None) });
    }
    let (child, stderr) = spawn_curl_process(program, config.into_bytes())?;
    let shared = SharedState::new(destinations, Some(child));
    let bg = Arc::clone(&shared);
    let worker = thread::spawn(move || run_reader(stderr, tx, by_part, &bg));
    Ok(BatchHandle { shared, rx: Mutex::new(rx), worker: Mutex::new(Some(worker)) })
}

/// Starts downloading `items` with one `curl` process over `1..=6` reused parallel connections.
pub fn start(items: &[BatchItem], headers: &[(&str, &str)], timeout_per_file: Duration, parallel: usize) -> Result<BatchHandle, BatchError> {
    start_with_program(OsStr::new("curl"), items, headers, timeout_per_file, parallel)
}

fn run_reader(stderr: ChildStderr, tx: Sender<BatchResult>, mut by_part: HashMap<String, VecDeque<usize>>, shared: &SharedState) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    while !shared.cancelled.load(Ordering::Acquire) {
        line.clear();
        if matches!(reader.read_line(&mut line), Ok(0) | Err(_)) || shared.cancelled.load(Ordering::Acquire) {
            break;
        }
        if let Some(res) = handle_record(&line, &mut by_part, shared) {
            shared.pending_results.fetch_add(1, Ordering::AcqRel);
            if tx.send(res).is_err() {
                shared.pending_results.fetch_sub(1, Ordering::AcqRel);
                break;
            }
        }
    }
    reap_child(shared);
    cleanup_parts(&shared.destinations);
    shared.worker_done.store(true, Ordering::Release);
    drop(tx);
}

fn handle_record(line: &str, by_part: &mut HashMap<String, VecDeque<usize>>, shared: &SharedState) -> Option<BatchResult> {
    let rest = line.trim_end_matches(['\r', '\n']).strip_prefix(RECORD_PREFIX)?;
    let mut fields = rest.splitn(3, '\t');
    let http_status = fields.next()?.parse::<u16>().unwrap_or(0);
    let exit_code = fields.next()?.parse::<i32>().unwrap_or(-1);
    let idx = by_part.get_mut(&fields.next()?.replace('\\', "/"))?.pop_front()?;
    let dest = shared.destinations.get(idx)?;
    if shared.cancelled.load(Ordering::Acquire) {
        return None;
    }
    let part = part_path(dest);
    if exit_code == 0 && http_status == 200 {
        if !part.exists() { let _ = fs::write(&part, b""); }
        if fs::rename(&part, dest).is_err() { let _ = fs::remove_file(&part); }
    } else {
        let _ = fs::remove_file(&part);
    }
    Some(BatchResult { index: idx, http_status, exit_code })
}

fn reap_child(shared: &SharedState) {
    if let Some(mut child) = shared.child.lock().ok().and_then(|mut g| g.take()) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn cleanup_parts(destinations: &[PathBuf]) {
    for dest in destinations { let _ = fs::remove_file(part_path(dest)); }
}

#[cfg(test)]
#[path = "curl_batch_a_tests.rs"]
mod tests_a;
#[cfg(test)]
#[path = "curl_batch_b_tests.rs"]
mod tests_b;
