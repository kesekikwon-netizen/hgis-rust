//! Local HTTP/1.1 keep-alive test server, OS process probes, and batch tests (part 1/3).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    collections::HashMap, ffi::OsStr, fs, io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener, TcpStream}, path::{Path, PathBuf}, process::Command,
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, mpsc, Arc, Condvar, Mutex},
    thread::{self, JoinHandle}, time::{Duration, Instant},
};

use super::*;

pub(super) const TEST_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const FAKE_KEY: &str = "FAKE-TEST-KEY-0000";
static TEMP_SEQ: AtomicUsize = AtomicUsize::new(1);

pub(super) fn start_with_program(program: &OsStr, items: &[BatchItem], headers: &[(&str, &str)], timeout: Duration, parallel: usize) -> Result<BatchHandle, BatchError> {
    start_with_program_checked(program, items, headers, timeout, parallel, default_allow_launch)
}

pub(super) fn start_from_reader_for_test<R: Read + Send + 'static>(items: &[BatchItem], stream: R) -> BatchHandle {
    let destinations: Vec<PathBuf> = items.iter().map(|i| i.destination.clone()).collect();
    let by_part: HashMap<String, usize> = destinations.iter().enumerate().map(|(i, d)| (normalize_match_path(&part_path(d).to_string_lossy()), i)).collect();
    let ((tx, rx), shared) = (mpsc::channel(), SharedState::new(destinations, None, false));
    let bg = Arc::clone(&shared);
    BatchHandle { shared, rx: Mutex::new(rx), worker: Mutex::new(Some(thread::spawn(move || run_reader(stream, tx, by_part, &bg)))), pid: None }
}

pub(super) struct TempDir { path: PathBuf }

impl TempDir {
    pub(super) fn new(tag: &str) -> Self {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("strata-map2a-{tag}-{}-{seq}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }
    pub(super) fn path(&self) -> &Path { &self.path }
}

impl Drop for TempDir {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.path); }
}

pub(super) fn has_part_files(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else { return false };
    for entry in entries.flatten() {
        let p = entry.path();
        if (p.is_dir() && has_part_files(&p)) || p.to_string_lossy().ends_with(".part") { return true; }
    }
    false
}

/// Queries the operating system directly (outside `Child`) whether `pid` currently exists.
pub(super) fn os_process_exists(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let out = Command::new("tasklist.exe").args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"]).creation_flags(0x0800_0000).output().expect("tasklist");
        String::from_utf8_lossy(&out.stdout).contains(&format!(",\"{pid}\","))
    }
    #[cfg(unix)]
    {
        let alive = Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success());
        let zombie = fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|s| s.contains(") Z "));
        alive && !zombie
    }
}

pub(super) fn wait_process_gone(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while os_process_exists(pid) {
        if Instant::now() >= deadline { return false; }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

pub(super) fn kill_os_process(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill.exe").args(["/F", "/PID", &pid.to_string()]).creation_flags(0x0800_0000).output();
    }
    #[cfg(unix)]
    {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
    }
}

pub(super) struct HoldGate { state: Mutex<(usize, bool)>, cv: Condvar }

impl HoldGate {
    pub(super) fn new() -> Arc<Self> { Arc::new(Self { state: Mutex::new((0, false)), cv: Condvar::new() }) }

    pub(super) fn enter_and_wait(&self) {
        let mut guard = self.state.lock().unwrap();
        guard.0 += 1;
        self.cv.notify_all();
        let deadline = Instant::now() + TEST_TIMEOUT;
        while !guard.1 {
            let now = Instant::now();
            if now >= deadline { break; }
            guard = self.cv.wait_timeout(guard, deadline - now).unwrap().0;
        }
    }

    pub(super) fn wait_entered(&self, min_entered: usize, timeout: Duration) -> bool {
        let mut guard = self.state.lock().unwrap();
        let deadline = Instant::now() + timeout;
        while guard.0 < min_entered {
            let now = Instant::now();
            if now >= deadline { return false; }
            guard = self.cv.wait_timeout(guard, deadline - now).unwrap().0;
        }
        true
    }

    pub(super) fn release(&self) {
        let mut guard = self.state.lock().unwrap();
        guard.1 = true;
        self.cv.notify_all();
    }
}

#[derive(Clone)]
pub(super) enum Reply {
    Ok(Vec<u8>),
    DelayedOk(Vec<u8>, Duration),
    NotFound(Vec<u8>),
    Redirect302(String),
    CloseWithoutAnswer,
    HoldThenOk(Arc<HoldGate>, Vec<u8>),
}

struct ServerShared {
    routes: HashMap<String, Reply>,
    shutdown: AtomicBool,
    accepted_connections: AtomicUsize,
    peak_open_requests: AtomicUsize,
    active_state: (Mutex<usize>, Condvar),
    requests: Mutex<Vec<(String, String, Vec<String>)>>,
    streams: Mutex<Vec<TcpStream>>,
    gates: Vec<Arc<HoldGate>>,
}

pub(super) struct TestServer {
    port: u16,
    shared: Arc<ServerShared>,
    accept_thread: Option<JoinHandle<()>>,
}

impl TestServer {
    pub(super) fn start(routes: HashMap<String, Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
        let port = listener.local_addr().unwrap().port();
        let gates = routes.values().filter_map(|r| if let Reply::HoldThenOk(g, _) = r { Some(Arc::clone(g)) } else { None }).collect();
        let shared = Arc::new(ServerShared {
            routes, shutdown: AtomicBool::new(false), accepted_connections: AtomicUsize::new(0),
            peak_open_requests: AtomicUsize::new(0), active_state: (Mutex::new(0), Condvar::new()),
            requests: Mutex::new(Vec::new()), streams: Mutex::new(Vec::new()), gates,
        });
        let bg = Arc::clone(&shared);
        let accept_thread = thread::spawn(move || {
            let mut workers = Vec::new();
            for incoming in listener.incoming() {
                if bg.shutdown.load(Ordering::Acquire) { break; }
                let Ok(stream) = incoming else { break };
                if bg.shutdown.load(Ordering::Acquire) { break; }
                bg.accepted_connections.fetch_add(1, Ordering::AcqRel);
                if let Ok(clone) = stream.try_clone() { bg.streams.lock().unwrap().push(clone); }
                let cs = Arc::clone(&bg);
                workers.push(thread::spawn(move || serve_connection(stream, &cs)));
            }
            for w in workers { let _ = w.join(); }
        });
        Self { port, shared, accept_thread: Some(accept_thread) }
    }

    pub(super) fn url(&self, path: &str) -> String { format!("http://127.0.0.1:{}{path}", self.port) }
    pub(super) fn accepted_connections(&self) -> usize { self.shared.accepted_connections.load(Ordering::Acquire) }
    pub(super) fn peak_open_requests(&self) -> usize { self.shared.peak_open_requests.load(Ordering::Acquire) }
    pub(super) fn request_paths(&self) -> Vec<String> { self.shared.requests.lock().unwrap().iter().map(|(p, _, _)| p.clone()).collect() }
    pub(super) fn raw_targets(&self) -> Vec<String> { self.shared.requests.lock().unwrap().iter().map(|(_, t, _)| t.clone()).collect() }
    pub(super) fn first_request_headers(&self) -> Vec<String> { self.shared.requests.lock().unwrap().first().map(|(_, _, h)| h.clone()).unwrap_or_default() }

    pub(super) fn wait_idle(&self, timeout: Duration) -> bool {
        let mut active = self.shared.active_state.0.lock().unwrap();
        let deadline = Instant::now() + timeout;
        while *active > 0 {
            let now = Instant::now();
            if now >= deadline { return false; }
            active = self.shared.active_state.1.wait_timeout(active, deadline - now).unwrap().0;
        }
        true
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        for g in &self.shared.gates { g.release(); }
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        for s in self.shared.streams.lock().unwrap().drain(..) { let _ = s.shutdown(Shutdown::Both); }
        if let Some(handle) = self.accept_thread.take() { let _ = handle.join(); }
    }
}

fn serve_connection(stream: TcpStream, shared: &ServerShared) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(TEST_TIMEOUT));
    let _ = stream.set_write_timeout(Some(TEST_TIMEOUT));
    let Ok(mut writer) = stream.try_clone() else { return };
    let mut reader = BufReader::new(stream);
    while !shared.shutdown.load(Ordering::Acquire) {
        let mut req_line = String::new();
        if matches!(reader.read_line(&mut req_line), Ok(0) | Err(_)) { break; }
        let target = req_line.split_whitespace().nth(1).unwrap_or("").to_owned();
        if target.is_empty() { break; }
        let path = target.split('?').next().unwrap_or(&target).to_owned();
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            if matches!(reader.read_line(&mut line), Ok(0) | Err(_)) { return; }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() { break; }
            headers.push(trimmed.to_owned());
        }
        let open_now = { let mut a = shared.active_state.0.lock().unwrap(); *a += 1; *a };
        shared.peak_open_requests.fetch_max(open_now, Ordering::AcqRel);
        shared.requests.lock().unwrap().push((path.clone(), target.clone(), headers));
        let action = shared.routes.get(&target).or_else(|| shared.routes.get(&path)).cloned().unwrap_or_else(|| Reply::NotFound(b"missing".to_vec()));
        let keep_open = write_reply(&mut writer, action);
        {
            let mut a = shared.active_state.0.lock().unwrap();
            *a -= 1;
            if *a == 0 { shared.active_state.1.notify_all(); }
        }
        if !keep_open { let _ = writer.shutdown(Shutdown::Both); break; }
    }
}

fn write_reply(writer: &mut TcpStream, reply: Reply) -> bool {
    let (status, extra_hdr, body) = match reply {
        Reply::Ok(b) => ("200 OK", String::new(), b),
        Reply::DelayedOk(b, d) => { thread::sleep(d); ("200 OK", String::new(), b) }
        Reply::NotFound(b) => ("404 Not Found", String::new(), b),
        Reply::Redirect302(loc) => ("302 Found", format!("Location: {loc}\r\n"), Vec::new()),
        Reply::CloseWithoutAnswer => return false,
        Reply::HoldThenOk(gate, b) => { gate.enter_and_wait(); ("200 OK", String::new(), b) }
    };
    let head = format!("HTTP/1.1 {status}\r\n{extra_hdr}Content-Length: {}\r\nConnection: keep-alive\r\n\r\n", body.len());
    writer.write_all(head.as_bytes()).is_ok() && writer.write_all(&body).is_ok() && writer.flush().is_ok()
}

pub(super) fn curl_version_line() -> String {
    let out = Command::new("curl").arg("--version").output().expect("run curl --version");
    String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").to_owned()
}

/// Parallelism outside `1..=6` is refused; headers are sent; `is_done` and `Drop` OS cancellation work.
#[test]
fn parallel_bounds_headers_done_and_drop_cancel() {
    let tmp = TempDir::new("bounds");
    let item = BatchItem::new("http://127.0.0.1:1/unused", tmp.path().join("t.bin"));
    assert_eq!(start(std::slice::from_ref(&item), &[], Duration::from_secs(5), 0).unwrap_err(), BatchError::InvalidParallel(0));
    assert_eq!(start(std::slice::from_ref(&item), &[], Duration::from_secs(5), 7).unwrap_err(), BatchError::InvalidParallel(7));

    let gate = HoldGate::new();
    let server = TestServer::start(HashMap::from([
        ("/h".to_owned(), Reply::Ok(b"header-ok".to_vec())),
        ("/hold".to_owned(), Reply::HoldThenOk(gate.clone(), b"never-saved".to_vec())),
    ]));
    let dest = tmp.path().join("h.bin");
    let headers = [("Referer", "https://localhost"), ("User-Agent", "StrataBatchTest/1.0")];
    let handle = start(&[BatchItem::new(server.url("/h"), &dest)], &headers, Duration::from_secs(5), 6).expect("start");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), Some(BatchResult { index: 0, http_status: 200, exit_code: 0 }));
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert!(handle.is_done() && handle.ended_normally());
    assert_eq!(fs::read(&dest).unwrap(), b"header-ok");
    let seen = server.first_request_headers();
    assert!(seen.iter().any(|h| h.eq_ignore_ascii_case("Referer: https://localhost")), "{seen:?}");
    assert!(seen.iter().any(|h| h.eq_ignore_ascii_case("User-Agent: StrataBatchTest/1.0")), "{seen:?}");

    let (held_dest, next_dest) = (tmp.path().join("held.bin"), tmp.path().join("after_drop.bin"));
    let (pid, reqs_before_drop) = {
        let drop_handle = start(&[BatchItem::new(server.url("/hold"), &held_dest), BatchItem::new(server.url("/h"), &next_dest)], &[], Duration::from_secs(10), 1).expect("start drop");
        assert!(gate.wait_entered(1, TEST_TIMEOUT));
        let pid = drop_handle.pid().expect("spawned pid");
        assert!(os_process_exists(pid), "OS must report curl PID {pid} alive before drop");
        (pid, server.request_paths().len())
    };
    assert!(wait_process_gone(pid, TEST_TIMEOUT), "OS must report curl PID {pid} gone after drop");
    gate.release();
    assert!(server.wait_idle(TEST_TIMEOUT), "server finished handling held request");
    assert_eq!(server.request_paths().len(), reqs_before_drop, "no new request after drop");
    assert!(!held_dest.exists() && !next_dest.exists() && !has_part_files(tmp.path()));
}
