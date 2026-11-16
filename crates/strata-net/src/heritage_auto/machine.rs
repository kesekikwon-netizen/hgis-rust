//! The stages of a heritage run and what the app may read of it.
//!
//! Stage order, from `KaHeritageBrowser.cpp`: open the portal, prove whether we are logged in,
//! log in, open the download page, agree to the pledge, pick the kind, pick the region, search,
//! press 전체다운로드, wait for the ZIP. One kind at a time, in the C++ tab order
//! (`MainWindow.cpp:3135-3140` shows each kind as soon as it is ready).

use std::path::PathBuf;

use super::{PageHost, PageId, ScriptCall};

/// Where the run stands. The C++ keeps this in `m_stage` (`KaHeritageBrowser.cpp:96-121`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HeritageStage {
    /// Nothing started.
    #[default]
    Idle,
    /// Ask the page whether we are past the login form (`:626-632`).
    LoginProbe,
    /// Fill the form and call the site's `goLogin()` (`:898-933`).
    LoggingIn,
    /// Open the download panel through `showAgreePopup()` (`:1030-1100`).
    DownloadPage,
    /// Tick the pledge box and press 확인 (`:1100-1180`).
    Terms,
    /// Pick the kind's tab (`:1180-1260`).
    Kind,
    /// Pick 시·도 and 시·군 (`:1260-1360`).
    Region,
    /// Press 검색 and read how many rows came back (`:1360-1450`).
    Search,
    /// Press 전체다운로드 for the current kind (`:1450-1524`).
    DownloadAll,
    /// The site is building the ZIP; probe until a file arrives or the retries run out.
    Waiting,
    /// Every kind is through.
    Done,
    /// The run stopped, with the words to show.
    Stopped(String),
}

/// What the app must know about a step.
#[derive(Debug, Clone, PartialEq)]
pub enum HeritageEvent {
    /// Words for the status line; the run goes on.
    Note(String),
    /// A ZIP is complete on disk for `kind`.
    ZipReady {
        /// The kind's Korean name, one of [`super::scripts::KINDS`].
        kind: &'static str,
        /// Where the browser put it.
        path: PathBuf,
    },
    /// The run stopped; the words to show the user.
    Stopped(String),
}

/// What a run is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeritageConfig {
    /// The saved account id; never written to a log or a receipt.
    pub account_id: String,
    /// The saved password; never written to a log or a receipt.
    pub account_password: String,
    /// 시·도 of the survey's city (`HeritageRegionResolver`).
    pub sido: String,
    /// 시·군 of the survey's city.
    pub city: String,
    /// A fixture base address, when a test or `--snap` drives a local server instead of the
    /// site. `None` means the real intranet.
    pub fixture_base: Option<String>,
}

impl HeritageConfig {
    /// The login landing page for this run: the fixture's when one was given.
    #[must_use]
    pub fn login_url(&self) -> String {
        match &self.fixture_base {
            Some(base) => format!("{}/user/checkNet.do", base.trim_end_matches('/')),
            None => super::scripts::LOGIN_URL.to_owned(),
        }
    }

    /// The download panel address for this run.
    #[must_use]
    pub fn download_url(&self) -> String {
        match &self.fixture_base {
            Some(base) => format!("{}/user/data/heritageDownload.do", base.trim_end_matches('/')),
            None => format!(
                "https://intranet.gis-heritage.go.kr{}",
                super::scripts::DOWNLOAD_PAGE_PATH
            ),
        }
    }
}

/// One heritage run.
#[derive(Debug)]
pub struct HeritageAutomation {
    pub(super) config: HeritageConfig,
    pub(super) stage: HeritageStage,
    pub(super) token: u64,
    /// Ticks spent in the current stage without an answer moving it on.
    pub(super) quiet_ticks: u8,
    /// How many times the browser was sent to the login page.
    pub(super) opens: u8,
    /// Whether a submit was already tried, so a second "still-login" is a give-up.
    pub(super) submitted: bool,
    /// The kind being fetched, as an index into [`super::scripts::KINDS`].
    pub(super) kind_index: usize,
    /// The token of a script still waiting for its answer.
    ///
    /// One ask at a time. The C++ polls every 700 ms (`KaHeritageBrowser.cpp:103`) and waits
    /// for the answer before asking again; asking on every frame makes the answer that does
    /// arrive look older than the newest token and be dropped, and the run never moves.
    pub(super) pending: Option<u64>,
    /// When the last ask went out, so the next one waits [`super::LOGIN_POLL_MS`].
    pub(super) asked_at_ms: u64,
}

impl HeritageAutomation {
    /// Starts a run in [`HeritageStage::Idle`]; the first `next_step` opens the portal.
    #[must_use]
    pub fn new(config: HeritageConfig) -> Self {
        Self {
            config,
            stage: HeritageStage::Idle,
            token: 0,
            quiet_ticks: 0,
            opens: 0,
            submitted: false,
            kind_index: 0,
            pending: None,
            asked_at_ms: 0,
        }
    }

    /// Where the run stands now.
    #[must_use]
    pub fn stage(&self) -> &HeritageStage {
        &self.stage
    }

    /// The kind being fetched, as its Korean name.
    #[must_use]
    pub fn current_kind(&self) -> &'static str {
        super::scripts::KINDS[self.kind_index.min(super::scripts::KINDS.len() - 1)].1
    }

    /// The next thing to ask a page, or `None` when the run is over, waiting for the last
    /// answer, or waiting for its turn.
    ///
    /// `host` is used to open the portal; every later step is a script whose answer comes
    /// back through [`Self::script_answered`].
    pub fn next_step(&mut self, host: &mut dyn PageHost, now_ms: u64) -> Option<ScriptCall> {
        // The portal is opened once, as soon as the browser has a page to open it in. That is
        // a navigation, not a script, so this call answers `None`.
        if self.stage == HeritageStage::Idle {
            self.open_portal(host)?;
            return None;
        }
        // One ask at a time, and no sooner than the poll interval: the C++ does the same
        // (`KaHeritageBrowser.cpp:103`). Asking on every frame makes every answer arrive
        // "late" and be dropped by the token check below.
        if self.pending.is_some() || now_ms.saturating_sub(self.asked_at_ms) < super::LOGIN_POLL_MS {
            return None;
        }
        // The page the asks go to is the one the browser has open; without one there is
        // nothing to ask.
        let page = host.pages().first().map(|p| p.id)?;
        let script = match self.stage.clone() {
            HeritageStage::LoginProbe => super::scripts::login_probe_script(),
            HeritageStage::LoggingIn => super::scripts::login_script(
                &self.config.account_id.clone(),
                &self.config.account_password.clone(),
            ),
            HeritageStage::Done | HeritageStage::Stopped(_) | HeritageStage::Idle => return None,
            _ => return None,
        };
        Some(self.ask_on(page, script, now_ms))
    }

    /// Opens the portal, once. Answers `None` until the browser has a page to open it in.
    fn open_portal(&mut self, host: &mut dyn PageHost) -> Option<()> {
        if host.pages().is_empty() {
            return None;
        }
        host.navigate(&self.config.login_url());
        self.opens += 1;
        self.stage = HeritageStage::LoginProbe;
        self.quiet_ticks = 0;
        Some(())
    }

    /// A script answered. Returns what the app must know, if anything.
    pub fn script_answered(&mut self, token: u64, value: &str) -> Option<HeritageEvent> {
        // Only the answer we are waiting for, so a late answer cannot move a later stage.
        if self.pending != Some(token) {
            return None;
        }
        self.pending = None;
        let value = value.trim().trim_matches('"');
        match self.stage.clone() {
            HeritageStage::LoginProbe => self.on_login_probe(value),
            HeritageStage::LoggingIn => self.on_login_submitted(value),
            _ => None,
        }
    }

    /// A token for the next script, so a late answer to an earlier one is ignored.
    pub(super) fn ask_on(&mut self, page: PageId, script: String, now_ms: u64) -> ScriptCall {
        self.token += 1;
        self.pending = Some(self.token);
        self.asked_at_ms = now_ms;
        ScriptCall {
            page,
            script,
            token: self.token,
        }
    }

    /// Moves the run on and remembers why.
    pub(super) fn go(&mut self, stage: HeritageStage) {
        self.stage = stage;
        self.quiet_ticks = 0;
    }

    /// Stops the run with the words the user should see.
    pub(super) fn stop(&mut self, words: String) -> Option<HeritageEvent> {
        self.stage = HeritageStage::Stopped(words.clone());
        Some(HeritageEvent::Stopped(words))
    }
}
