//! Local HTTP/1.1 keep-alive test server and batch downloader tests (part 1/2).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::*;

pub(super) const TEST_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const FAKE_KEY: &str = "FAKE-TEST-KEY-0000";

static TEMP_SEQ: AtomicUsize = AtomicUsize::new(1);

pub(super) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(super) fn new(tag: &str) -> Self {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("strata-map2a-{tag}-{}-{seq}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(super) fn has_part_files(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else { return false };
    for entry in entries.flatten() {
        let p = entry.path();
        if (p.is_dir() && has_part_files(&p)) || p.to_string_lossy().ends_with(".part") {
            return true;
        }
    }
    false
}

pub(super) struct HoldGate {
    state: Mutex<(usize, bool)>,
    cv: Condvar,
}

impl HoldGate {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self { state: Mutex::new((0, false)), cv: Condvar::new() })
    }

    pub(super) fn enter_and_wait(&self) {
        let mut guard = self.state.lock().unwrap();
        guard.0 += 1;
        self.cv.notify_all();
        let deadline = Instant::now() + TEST_TIMEOUT;
        while !guard.1 {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            guard = self.cv.wait_timeout(guard, deadline - now).unwrap().0;
        }
    }

    pub(super) fn wait_entered(&self, min_entered: usize, timeout: Duration) -> bool {
        let mut guard = self.state.lock().unwrap();
        let deadline = Instant::now() + timeout;
        while guard.0 < min_entered {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
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
    CloseWithoutAnswer,
    HoldThenOk(Arc<HoldGate>, Vec<u8>),
}

struct ServerShared {
    routes: HashMap<String, Reply>,
    shutdown: AtomicBool,
    accepted_connections: AtomicUsize,
    open_requests: AtomicUsize,
    peak_open_requests: AtomicUsize,
    requests: Mutex<Vec<(String, Vec<String>)>>,
    streams: Mutex<Vec<TcpStream>>,
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
        let shared = Arc::new(ServerShared {
            routes,
            shutdown: AtomicBool::new(false),
            accepted_connections: AtomicUsize::new(0),
            open_requests: AtomicUsize::new(0),
            peak_open_requests: AtomicUsize::new(0),
            requests: Mutex::new(Vec::new()),
            streams: Mutex::new(Vec::new()),
        });
        let bg = Arc::clone(&shared);
        let accept_thread = thread::spawn(move || {
            let mut workers = Vec::new();
            for incoming in listener.incoming() {
                if bg.shutdown.load(Ordering::Acquire) {
                    break;
                }
                let Ok(stream) = incoming else { break };
                if bg.shutdown.load(Ordering::Acquire) {
                    break;
                }
                bg.accepted_connections.fetch_add(1, Ordering::AcqRel);
                if let Ok(clone) = stream.try_clone() {
                    bg.streams.lock().unwrap().push(clone);
                }
                let cs = Arc::clone(&bg);
                workers.push(thread::spawn(move || serve_connection(stream, &cs)));
            }
            for w in workers {
                let _ = w.join();
            }
        });
        Self { port, shared, accept_thread: Some(accept_thread) }
    }

    pub(super) fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    pub(super) fn accepted_connections(&self) -> usize {
        self.shared.accepted_connections.load(Ordering::Acquire)
    }

    pub(super) fn peak_open_requests(&self) -> usize {
        self.shared.peak_open_requests.load(Ordering::Acquire)
    }

    pub(super) fn request_paths(&self) -> Vec<String> {
        self.shared.requests.lock().unwrap().iter().map(|(p, _)| p.clone()).collect()
    }

    pub(super) fn first_request_headers(&self) -> Vec<String> {
        self.shared.requests.lock().unwrap().first().map(|(_, h)| h.clone()).unwrap_or_default()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        for s in self.shared.streams.lock().unwrap().drain(..) {
            let _ = s.shutdown(Shutdown::Both);
        }
        if let Some(handle) = self.accept_thread.take() {
            let _ = handle.join();
        }
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
        if matches!(reader.read_line(&mut req_line), Ok(0) | Err(_)) {
            break;
        }
        let target = req_line.split_whitespace().nth(1).unwrap_or("");
        if target.is_empty() {
            break;
        }
        let path = target.split('?').next().unwrap_or(target).to_owned();
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            if matches!(reader.read_line(&mut line), Ok(0) | Err(_)) {
                return;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }
            headers.push(trimmed.to_owned());
        }
        let open_now = shared.open_requests.fetch_add(1, Ordering::AcqRel) + 1;
        shared.peak_open_requests.fetch_max(open_now, Ordering::AcqRel);
        shared.requests.lock().unwrap().push((path.clone(), headers));
        let action = shared.routes.get(&path).cloned().unwrap_or_else(|| Reply::NotFound(b"missing".to_vec()));
        let keep_open = write_reply(&mut writer, action);
        shared.open_requests.fetch_sub(1, Ordering::AcqRel);
        if !keep_open {
            let _ = writer.shutdown(Shutdown::Both);
            break;
        }
    }
}

fn write_reply(writer: &mut TcpStream, reply: Reply) -> bool {
    let (status, body) = match reply {
        Reply::Ok(b) => ("200 OK", b),
        Reply::DelayedOk(b, d) => {
            thread::sleep(d);
            ("200 OK", b)
        }
        Reply::NotFound(b) => ("404 Not Found", b),
        Reply::CloseWithoutAnswer => return false,
        Reply::HoldThenOk(gate, b) => {
            gate.enter_and_wait();
            ("200 OK", b)
        }
    };
    let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n", body.len());
    writer.write_all(head.as_bytes()).is_ok() && writer.write_all(&body).is_ok() && writer.flush().is_ok()
}

pub(super) fn curl_version_line() -> String {
    let out = Command::new("curl").arg("--version").output().expect("run curl --version");
    String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").to_owned()
}

/// Parallelism outside `1..=6` is refused; headers are sent; `is_done` and `Drop` cancellation work.
#[test]
fn parallel_bounds_headers_done_and_drop_cancel() {
    let tmp = TempDir::new("bounds");
    let item = BatchItem::new("http://127.0.0.1:1/unused", tmp.path().join("t.bin"));
    assert_eq!(start(std::slice::from_ref(&item), &[], Duration::from_secs(5), 0).unwrap_err(), BatchError::InvalidParallel(0));
    assert_eq!(start(std::slice::from_ref(&item), &[], Duration::from_secs(5), 7).unwrap_err(), BatchError::InvalidParallel(7));

    let gate = HoldGate::new();
    let routes = HashMap::from([
        ("/h".to_owned(), Reply::Ok(b"header-ok".to_vec())),
        ("/hold".to_owned(), Reply::HoldThenOk(gate.clone(), b"never-saved".to_vec())),
    ]);
    let server = TestServer::start(routes);
    let dest = tmp.path().join("h.bin");
    let headers = [("Referer", "https://localhost"), ("User-Agent", "StrataBatchTest/1.0")];
    let handle = start(&[BatchItem::new(server.url("/h"), &dest)], &headers, Duration::from_secs(5), 6).expect("start");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), Some(BatchResult { index: 0, http_status: 200, exit_code: 0 }));
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert!(handle.is_done());
    assert_eq!(fs::read(&dest).unwrap(), b"header-ok");
    let seen = server.first_request_headers();
    assert!(seen.iter().any(|h| h.eq_ignore_ascii_case("Referer: https://localhost")), "{seen:?}");
    assert!(seen.iter().any(|h| h.eq_ignore_ascii_case("User-Agent: StrataBatchTest/1.0")), "{seen:?}");

    let (held_dest, next_dest) = (tmp.path().join("held.bin"), tmp.path().join("after_drop.bin"));
    {
        let _drop_handle = start(
            &[BatchItem::new(server.url("/hold"), &held_dest), BatchItem::new(server.url("/h"), &next_dest)],
            &[],
            Duration::from_secs(10),
            1,
        ).expect("start drop test");
        assert!(gate.wait_entered(1, TEST_TIMEOUT));
    }
    gate.release();
    assert!(!held_dest.exists() && !next_dest.exists() && !has_part_files(tmp.path()));
}
