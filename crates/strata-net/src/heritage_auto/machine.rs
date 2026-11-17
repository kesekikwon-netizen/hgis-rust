//! The stages of a heritage run and what the app may read of it.
//!
//! Stage order, from `KaHeritageBrowser.cpp`: open the portal, prove whether we are logged in,
//! log in, open the download page, agree to the pledge, pick the kind, pick the region, search,
//! press 전체다운로드, wait for the ZIP. One kind at a time, in the C++ tab order
//! (`MainWindow.cpp:3135-3140` shows each kind as soon as it is ready).

pub use super::types::{HeritageConfig, HeritageEvent, HeritageStage};
use super::{PageHost, PageId, ScriptCall};

/// One heritage run.
#[derive(Debug)]
pub struct HeritageAutomation {
    pub(super) config: HeritageConfig,
    pub(super) stage: HeritageStage,
    pub(super) token: u64,
    /// Ticks spent in the current stage without an answer moving it on.
    pub(super) quiet_ticks: u8,
    pub(super) observed_at_ms: u64,
    pub(super) region_poll_at_ms: u64,
    /// How many times the browser was sent to the login page.
    pub(super) opens: u8,
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
    /// The pledge the page showed when the box was ticked (`HeritageIntranetFlow.cpp:230-243`),
    /// so a receipt can hold it.
    pub(super) pledge: Option<String>,
    /// How many transfers of the current kind have failed since it started.
    pub(super) retry_count: u64,
    /// When the next retry may happen (ms since the run began).
    pub(super) retry_at_ms: u64,
    /// Whether a pledge was agreed at any point in this run.
    pub(super) pledge_kept: bool,
    pub(super) group: super::transfer_group::TransferGroup,
    pub(super) phase_since_ms: u64,
    pub(super) baseline: Option<super::readiness::Baseline>,
    pub(super) queued_events: Vec<HeritageEvent>,
}

impl HeritageAutomation {
    /// Validates the native document callback before dispatch (`KaHeritageBrowser.cpp:658-659`).
    #[must_use]
    pub fn accepts_script_token(&self, token: u64) -> bool {
        !self.is_over() && self.pending == Some(token)
    }

    /// Observes the browser frame time before accepting its callbacks.
    pub fn observe_time(&mut self, now_ms: u64) {
        self.observed_at_ms = self.observed_at_ms.max(now_ms);
        if matches!(
            self.stage,
            HeritageStage::Region | HeritageStage::RegionReady
        ) {
            // C++ counts timer polls before its in-flight return, not completed answers
            // (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:871-885`).
            let polls =
                self.observed_at_ms.saturating_sub(self.region_poll_at_ms) / super::LOGIN_POLL_MS;
            self.quiet_ticks = self
                .quiet_ticks
                .saturating_add(u8::try_from(polls).unwrap_or(u8::MAX));
            self.region_poll_at_ms += polls * super::LOGIN_POLL_MS;
        }
    }

    /// Starts a run in [`HeritageStage::Idle`]; the first `next_step` opens the portal.
    #[must_use]
    pub fn new(config: HeritageConfig) -> Self {
        Self {
            config,
            stage: HeritageStage::Idle,
            token: 0,
            quiet_ticks: 0,
            observed_at_ms: 0,
            region_poll_at_ms: 0,
            opens: 0,
            kind_index: 0,
            pending: None,
            asked_at_ms: 0,
            pledge: None,
            retry_count: 0,
            retry_at_ms: 0,
            pledge_kept: false,
            group: super::transfer_group::TransferGroup::default(),
            phase_since_ms: 0,
            baseline: None,
            queued_events: Vec::new(),
        }
    }

    /// Where the run stands now.
    #[must_use]
    pub fn stage(&self) -> &HeritageStage {
        &self.stage
    }

    /// The pledge the page showed when the box was ticked, for the receipt.
    #[must_use]
    pub fn pledge_text(&self) -> Option<&str> {
        self.pledge.as_deref()
    }

    /// The token a test answers, when it set the stage directly.
    #[cfg(test)]
    pub(crate) fn current_token(&self) -> u64 {
        self.token
    }

    /// Puts the machine in a stage, for a test that does not want to drive it there.
    ///
    /// The token is advanced and marked outstanding, so `script_answered` accepts the answer a
    /// test gives straight away — which is what `current_token` hands it.
    #[cfg(test)]
    pub(crate) fn force_stage(&mut self, stage: HeritageStage) {
        self.stage = stage;
        self.token += 1;
        self.pending = Some(self.token);
        self.asked_at_ms = 0;
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
        self.observe_time(now_ms);
        // The portal is opened once, as soon as the browser has a page to open it in. That is
        // a navigation, not a script, so this call answers `None`.
        if self.stage == HeritageStage::Idle {
            self.asked_at_ms = now_ms;
            self.open_portal(host)?;
            return None;
        }
        // One ask at a time, and no sooner than the poll interval: the C++ does the same
        // (`KaHeritageBrowser.cpp:103`). Asking on every frame makes every answer arrive
        // "late" and be dropped by the token check below.
        if self.pending.is_some() || now_ms.saturating_sub(self.asked_at_ms) < super::LOGIN_POLL_MS
        {
            return None;
        }
        // The page the asks go to is the one the browser has open; without one there is
        // nothing to ask.
        let page = host.pages().first().map(|page| page.id)?;
        let script = match self.stage {
            HeritageStage::LoginProbe
            | HeritageStage::AwaitLogin
            | HeritageStage::TutorialProbe => super::scripts::login_probe_script(),
            HeritageStage::LoggingIn => {
                super::scripts::login_script(&self.config.account_id, &self.config.account_password)
            }
            HeritageStage::DownloadAll => super::scripts::download_all_script(),
            _ => self.page_script()?,
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
        self.go(HeritageStage::LoginProbe);
        Some(())
    }

    /// A script answered. Returns what the app must know, if anything.
    pub fn script_answered(&mut self, token: u64, value: &str) -> Option<HeritageEvent> {
        // Only the answer we are waiting for, so a late answer cannot move a later stage.
        if !self.accepts_script_token(token) {
            return None;
        }
        self.pending = None;
        // Qt delivered a decoded QVariant string (`KaHeritageBrowser.cpp:661-663`).
        // WebView2 returns JSON, including escapes inside JSON.stringify results.
        let decoded = serde_json::from_str::<String>(value).ok();
        let value = decoded.as_deref().unwrap_or(value).trim();
        match self.stage.clone() {
            HeritageStage::LoginProbe => self.on_login_probe(value),
            HeritageStage::LoggingIn => self.on_login_submitted(value),
            HeritageStage::AwaitLogin => self.on_login_wait(value),
            HeritageStage::TutorialProbe => self.on_tutorial_probe(value),
            HeritageStage::DownloadAll => self.on_download_all(value),
            _ => self.on_page_answer(value),
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
}
