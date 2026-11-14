//! Local fake HTTP server and fixtures ported from `LocalDownloadServer`
//! in `target/cpp-ref/tests/test_topographic_browser.cpp:38-225`.

#![forbid(unsafe_code)]

mod fixtures_a;
mod fixtures_b;
mod server;
mod server_routes;

use std::fmt;
use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub use fixtures_a::*;
pub use fixtures_b::*;

/// Total payload size for streamed/SDK transfers (`test_topographic_browser.cpp:67`).
pub const TRANSFER_SIZE: usize = 4 * 1024 * 1024;
/// Initial chunk written before holding a transfer open (`test_topographic_browser.cpp:68`).
pub const FIRST_CHUNK_SIZE: usize = 1024 * 1024;

/// Observer callback invoked on each `innorixDownLoad2.do` request (`test_topographic_browser.cpp:66`).
pub type SdkRequestCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Shared settings, switches, and counters of `LocalDownloadServer` (`test_topographic_browser.cpp:40-68`).
pub struct NgiiFixtureState {
    pub portal_fixture: Vec<u8>,
    pub download_fixture: Vec<u8>,
    pub search_fixture: Vec<u8>,
    pub login_fixture: Vec<u8>,
    pub any_id_fixture: Vec<u8>,
    pub logged_in: bool,
    pub accept_login: bool,
    pub hold_login_response: bool,
    pub fail_portal_connection: bool,
    pub redirect_transfer: bool,
    pub search_count: usize,
    pub login_check_count: usize,
    pub login_submission_count: usize,
    pub stream_fixture_archives: bool,
    pub fixture_archive_finished: bool,
    pub pending_xml_name: Vec<u8>,
    pub transfer_chunked: bool,
    pub interrupted_requests: usize,
    pub hold_sdk_transfers: bool,
    pub sdk_requests: Vec<String>,
    pub active_sdk_file: String,
    pub sdk_xml_name: String,
    pub sdk_request_observed: Option<SdkRequestCallback>,
    pub(crate) pending_xml_socket: Option<TcpStream>,
    pub(crate) transfer_socket: Option<TcpStream>,
    pub(crate) pending_login_script: Option<TcpStream>,
    pub(crate) sdk_xml_socket: Option<TcpStream>,
    pub(crate) held_sockets: Vec<TcpStream>,
    pub(crate) bg_writers: Vec<JoinHandle<()>>,
}

impl Default for NgiiFixtureState {
    fn default() -> Self {
        // Defaults from target/cpp-ref/tests/test_topographic_browser.cpp:40-66.
        Self {
            portal_fixture: Vec::new(),
            download_fixture: Vec::new(),
            search_fixture: Vec::new(),
            login_fixture: Vec::new(),
            any_id_fixture: Vec::new(),
            logged_in: true,
            accept_login: false,
            hold_login_response: false,
            fail_portal_connection: false,
            redirect_transfer: false,
            search_count: 0,
            login_check_count: 0,
            login_submission_count: 0,
            stream_fixture_archives: false,
            fixture_archive_finished: false,
            pending_xml_name: Vec::new(),
            transfer_chunked: false,
            interrupted_requests: 0,
            hold_sdk_transfers: false,
            sdk_requests: Vec::new(),
            active_sdk_file: String::new(),
            sdk_xml_name: String::new(),
            sdk_request_observed: None,
            pending_xml_socket: None,
            transfer_socket: None,
            pending_login_script: None,
            sdk_xml_socket: None,
            held_sockets: Vec::new(),
            bg_writers: Vec::new(),
        }
    }
}

impl fmt::Debug for NgiiFixtureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NgiiFixtureState")
            .field("portal_fixture_len", &self.portal_fixture.len())
            .field("download_fixture_len", &self.download_fixture.len())
            .field("search_fixture_len", &self.search_fixture.len())
            .field("login_fixture_len", &self.login_fixture.len())
            .field("any_id_fixture_len", &self.any_id_fixture.len())
            .field("logged_in", &self.logged_in)
            .field("accept_login", &self.accept_login)
            .field("hold_login_response", &self.hold_login_response)
            .field("fail_portal_connection", &self.fail_portal_connection)
            .field("redirect_transfer", &self.redirect_transfer)
            .field("search_count", &self.search_count)
            .field("login_check_count", &self.login_check_count)
            .field("login_submission_count", &self.login_submission_count)
            .field("stream_fixture_archives", &self.stream_fixture_archives)
            .field("fixture_archive_finished", &self.fixture_archive_finished)
            .field("pending_xml_name", &self.pending_xml_name)
            .field("transfer_chunked", &self.transfer_chunked)
            .field("interrupted_requests", &self.interrupted_requests)
            .field("hold_sdk_transfers", &self.hold_sdk_transfers)
            .field("sdk_requests", &self.sdk_requests)
            .field("active_sdk_file", &self.active_sdk_file)
            .field("sdk_xml_name", &self.sdk_xml_name)
            .field("has_sdk_observer", &self.sdk_request_observed.is_some())
            .finish()
    }
}

/// Ephemeral localhost HTTP test double ported from `LocalDownloadServer` (`test_topographic_browser.cpp:38-192`).
#[derive(Debug)]
pub struct NgiiFixtureSite {
    addr: SocketAddr,
    state: Arc<Mutex<NgiiFixtureState>>,
    stop: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

impl NgiiFixtureSite {
    /// Binds `127.0.0.1:0` with default `LocalDownloadServer` settings and starts serving.
    pub fn start() -> io::Result<Self> {
        Self::start_with(NgiiFixtureState::default())
    }

    /// Binds `127.0.0.1:0` with `initial` settings and starts serving on a background thread.
    pub fn start_with(initial: NgiiFixtureState) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let addr = listener.local_addr()?;
        let state = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        let state_bg = Arc::clone(&state);
        let stop_bg = Arc::clone(&stop);
        let accept_thread = thread::spawn(move || server::run_accept_loop(listener, state_bg, stop_bg));
        Ok(Self {
            addr,
            state,
            stop,
            accept_thread: Some(accept_thread),
        })
    }

    /// Bound `127.0.0.1:<ephemeral>` socket address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Formats `http://127.0.0.1:<port><path>`, defaulting empty `path` to `"/"` (`test_topographic_browser.cpp:189-191`).
    pub fn url(&self, path: &str) -> String {
        let suffix = if path.is_empty() { "/" } else { path };
        format!("http://127.0.0.1:{}{}", self.addr.port(), suffix)
    }

    /// Shared settings and counters handle (`test_topographic_browser.cpp:40-68`).
    pub fn state(&self) -> &Arc<Mutex<NgiiFixtureState>> {
        &self.state
    }

    /// Completes the held `transfer_socket` and any `pending_xml_socket` (`test_topographic_browser.cpp:81-96`).
    pub fn finish_transfer(&self) -> bool {
        server::finish_transfer(&self.state)
    }

    /// Completes the held `sdk_xml_socket` (`test_topographic_browser.cpp:97-103`).
    pub fn finish_sdk_xml(&self) -> bool {
        server::finish_sdk_xml(&self.state)
    }

    /// Aborts the held `transfer_socket` mid-stream (`test_topographic_browser.cpp:392`).
    pub fn abort_transfer(&self) -> bool {
        server::abort_transfer(&self.state)
    }

    /// Releases `pending_login_script` with an empty `200 OK` JS body (`test_topographic_browser.cpp:784-785, 932-933`).
    pub fn finish_pending_login_script(&self) -> bool {
        server::finish_pending_login_script(&self.state)
    }

    pub fn has_transfer_socket(&self) -> bool {
        self.state.lock().map(|g| g.transfer_socket.is_some()).unwrap_or(false)
    }

    pub fn has_pending_xml_socket(&self) -> bool {
        self.state.lock().map(|g| g.pending_xml_socket.is_some()).unwrap_or(false)
    }

    pub fn has_pending_login_script(&self) -> bool {
        self.state.lock().map(|g| g.pending_login_script.is_some()).unwrap_or(false)
    }

    pub fn has_sdk_xml_socket(&self) -> bool {
        self.state.lock().map(|g| g.sdk_xml_socket.is_some()).unwrap_or(false)
    }

    pub fn held_socket_count(&self) -> usize {
        self.state.lock().map(|g| g.held_sockets.len()).unwrap_or(0)
    }
}

impl Drop for NgiiFixtureSite {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        let writers = if let Ok(mut guard) = self.state.lock() {
            for sock in [
                guard.pending_xml_socket.take(),
                guard.transfer_socket.take(),
                guard.pending_login_script.take(),
                guard.sdk_xml_socket.take(),
            ]
            .into_iter()
            .flatten()
            {
                let _ = sock.shutdown(Shutdown::Both);
            }
            for sock in guard.held_sockets.drain(..) {
                let _ = sock.shutdown(Shutdown::Both);
            }
            std::mem::take(&mut guard.bg_writers)
        } else {
            Vec::new()
        };
        if let Some(handle) = self.accept_thread.take() {
            let _ = handle.join();
        }
        for writer in writers {
            let _ = writer.join();
        }
    }
}
