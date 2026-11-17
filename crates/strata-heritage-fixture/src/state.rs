//! What the fixture must serve, and what a test can ask of it.

use std::collections::HashMap;

use crate::server;

/// The account the fixture accepts, and the city it knows.
#[derive(Debug, Clone)]
pub struct FixtureState {
    /// The only `(USER_ID, USER_PW)` pair that leads to a logged-in page.
    pub account: (String, String),
    /// The only `(시도, 시군구)` pair the download page answers with rows for.
    pub city: (String, String),
    /// Whether the next `checkNet.do` shows a logged-in page.
    pub logged_in: bool,
    /// Signed in once already; a wrong password after that must not open a session.
    pub ever_logged_in: bool,
    /// Requests per route path, for a test to prove a step was reached.
    pub hits: HashMap<String, usize>,
    /// The ZIP `downloadFilesAll.do` answers with, per kind name.
    pub zips: HashMap<String, Vec<u8>>,
    /// Milliseconds `downloadFilesAll.do` waits before answering; 0 for a fast run.
    pub download_delay_ms: u64,
    /// Whether the logged-in content frame must show the site's agreement first.
    pub require_pledge: bool,
}

impl Default for FixtureState {
    fn default() -> Self {
        Self {
            account: ("user1".to_owned(), "pass1".to_owned()),
            city: ("경상북도".to_owned(), "경주시".to_owned()),
            logged_in: false,
            ever_logged_in: false,
            hits: HashMap::new(),
            zips: HashMap::new(),
            download_delay_ms: 0,
            require_pledge: false,
        }
    }
}

impl FixtureState {
    /// Count a request to `path`.
    pub fn count(&mut self, path: &str) {
        *self.hits.entry(path.to_owned()).or_insert(0) += 1;
    }

    /// How many times `path` was asked for.
    #[must_use]
    pub fn hits(&self, path: &str) -> usize {
        self.hits.get(path).copied().unwrap_or(0)
    }
}

/// A running fixture server. Dropping it stops the listener.
pub struct Fixture {
    pub(crate) base_url: String,
    pub(crate) stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) state: std::sync::Arc<std::sync::Mutex<FixtureState>>,
    pub(crate) join: Option<std::thread::JoinHandle<()>>,
}

impl Fixture {
    /// Starts a fixture that accepts `account` and knows `city`.
    #[must_use]
    pub fn start(account: (&str, &str), city: (&str, &str)) -> Self {
        let state = FixtureState {
            account: (account.0.to_owned(), account.1.to_owned()),
            city: (city.0.to_owned(), city.1.to_owned()),
            ..FixtureState::default()
        };
        server::spawn(state)
    }

    /// The address to hand the run, e.g. `http://127.0.0.1:54321`.
    #[must_use]
    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    /// A full address for `path` under this fixture.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// How many times `path` was asked for.
    #[must_use]
    pub fn hits(&self, path: &str) -> usize {
        self.state.lock().map(|s| s.hits(path)).unwrap_or(0)
    }

    /// Makes `downloadFilesAll.do` answer with `bytes` for `kind_name`.
    pub fn give_zip(&self, kind_name: &str, bytes: Vec<u8>) {
        if let Ok(mut s) = self.state.lock() {
            s.zips.insert(kind_name.to_owned(), bytes);
        }
    }

    /// Shows a real checkbox and confirmation before the download form (`KaHeritageBrowser.cpp:985-1001`).
    pub fn require_pledge(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.require_pledge = true;
        }
    }

    /// Makes `downloadFilesAll.do` wait `ms` before answering, standing in for the
    /// 40–120 s the site takes to build a ZIP.
    pub fn set_download_delay(&self, ms: u64) {
        if let Ok(mut s) = self.state.lock() {
            s.download_delay_ms = ms;
        }
    }

    /// Fetches a page the way a browser would, without a browser: plain GET.
    #[must_use]
    pub fn get(&self, path: &str) -> String {
        server::http_get(&self.url(path), None)
    }

    /// Posts a form body to `path`; returns the response body.
    #[must_use]
    pub fn post_form(&self, path: &str, body: &str) -> String {
        server::http_post_form(&self.url(path), body)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        // Unblock `accept` by connecting once.
        let _ = std::net::TcpStream::connect(self.base_url.trim_start_matches("http://"));
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl std::fmt::Debug for Fixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fixture")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}
