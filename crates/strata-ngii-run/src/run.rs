//! One run of the NGII sheet order in a hidden browser session: the stage logic of `strata_net::ngii_auto`
//! fed with what `strata_webview` reports, frame by frame, on the UI thread.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use raw_window_handle::HasWindowHandle;
use strata_net::ngii_auto::{AutoConfig, AutoEvent, NgiiAutomation, PageId, PageRef, Scope, Stage, UrlParts};
use strata_webview::downloads::DownloadRefusal;
use strata_webview::policy::{Navigation, NavigationPolicy};
use strata_webview::profile::remove_session_folder;
use strata_webview::{BrowserError, BrowserEvent, BrowserOptions, DownloadId, DownloadPlace, HiddenBrowser, ScriptCall, ViewId};

use crate::host::{QueuedHost, Wanted};

/// How a run is set up.
#[derive(Clone)]
pub struct RunOptions {
    /// Folder under which the browser session makes its data folder (removed when the run is over).
    pub data_parent: PathBuf,
    pub config: AutoConfig,
    /// The app's network switch.
    pub network_allowed: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions").field("data_parent", &self.data_parent).field("config", &self.config).finish_non_exhaustive()
    }
}

struct Core {
    machine: NgiiAutomation,
    host: QueuedHost,
    began: Instant,
    /// Files the stage logic said yes to, in that order, until the browser reports each as started or
    /// as one it could not place.
    accepted: VecDeque<PathBuf>,
}

impl Core {
    fn now(&self) -> u64 {
        u64::try_from(self.began.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// The run. `frame` is called once per frame while it lives; when the order is complete, stopped or failed
/// the browser session is closed and its data folder removed.
pub struct NgiiRun {
    browser: Option<HiddenBrowser>,
    core: Rc<RefCell<Core>>,
    answers: HashMap<ScriptCall, u64>,
    /// Running transfers by the file each will become.
    transfers: HashMap<DownloadId, PathBuf>,
    /// The data folder of a closed session that the engine has not let go of yet.
    leftover: Option<PathBuf>,
}

impl std::fmt::Debug for NgiiRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NgiiRun").field("open", &self.browser.is_some()).finish_non_exhaustive()
    }
}

/// The origin of a portal on this PC (tests only): it stays reachable while the network switch is off.
fn local_test_origin(config: &AutoConfig) -> Option<String> {
    let url = UrlParts::parse(&config.portal_url)?;
    let port = url.port?;
    (config.test_mode && url.scheme == "http" && url.host == "127.0.0.1").then(|| format!("http://127.0.0.1:{port}"))
}

impl NgiiRun {
    /// Opens the hidden session inside `parent` (the app's window). Nothing is asked of the site yet.
    pub fn start<W: HasWindowHandle>(parent: &W, options: RunOptions) -> Result<Self, BrowserError> {
        let local = local_test_origin(&options.config);
        // The C++ browser sets its transfer page up when the document is ready, with the same page script
        // (KaTopographicBrowser.cpp:527-534).
        let ready = format!("{}({{command:'configureTransfer',testMode:{}}});", options.config.script, options.config.test_mode);
        let mut browser = HiddenBrowser::start(
            parent,
            BrowserOptions {
                data_parent: options.data_parent,
                navigation: NavigationPolicy::new(options.network_allowed, local.as_deref()),
                document_ready_script: Some(ready),
            },
        )?;
        let core = Rc::new(RefCell::new(Core { machine: NgiiAutomation::new(options.config), host: QueuedHost::default(), began: Instant::now(), accepted: VecDeque::new() }));
        let decide = Rc::clone(&core);
        // Called by the engine while a page waits; the browser itself cannot be used in here.
        browser.set_download_decider(Box::new(move |offer| {
            let mut core = decide.try_borrow_mut().ok()?;
            let now = core.now();
            let Core { machine, host, accepted, .. } = &mut *core;
            let target = machine.download_offered(host, PageId(offer.view.0), &offer.suggested_name, now)?;
            accepted.push_back(target.directory.join(&target.file_name));
            Some(DownloadPlace { directory: target.directory, file_name: target.file_name })
        }));
        Ok(Self { browser: Some(browser), core, answers: HashMap::new(), transfers: HashMap::new(), leftover: None })
    }

    /// Asks for the sheets of a circle (see `NgiiAutomation::prepare_sheets`).
    pub fn prepare_sheets(&mut self, scope: Scope) {
        let mut core = self.core.borrow_mut();
        let now = core.now();
        let Core { machine, host, .. } = &mut *core;
        machine.prepare_sheets(host, scope, now);
    }

    /// The user's stop. The session is closed at the next `frame`, which also ends files being received.
    pub fn stop(&mut self) {
        let mut core = self.core.borrow_mut();
        let Core { machine, host, .. } = &mut *core;
        machine.stop(host);
    }

    #[must_use]
    pub fn stage(&self) -> Stage {
        self.core.borrow().machine.stage()
    }

    /// Reads the run's state (phase, notice, texts) without copying it.
    pub fn with_state<T>(&self, read: impl FnOnce(&NgiiAutomation) -> T) -> T {
        read(&self.core.borrow().machine)
    }

    /// Whether the run is over and nothing of its session is left.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.browser.is_none() && self.leftover.is_none()
    }

    fn pages(browser: &HiddenBrowser) -> Vec<PageRef> {
        browser.views().into_iter().map(|v| PageRef { id: PageId(v.id.0), url: v.url, opener: v.opener.map(|o| PageId(o.0)) }).collect()
    }

    /// One frame: what the browser reports goes to the stage logic, what the logic wants goes to the
    /// browser. Returns what happened in the run.
    pub fn frame<W: HasWindowHandle>(&mut self, parent: &W) -> Vec<AutoEvent> {
        if let Some(folder) = &self.leftover {
            if remove_session_folder(folder) {
                self.leftover = None;
            }
        }
        let Some(browser) = self.browser.as_mut() else {
            return self.core.borrow_mut().machine.take_events();
        };
        // The decider may run inside `pump`; the state is not borrowed across it.
        let reported = browser.pump(parent);
        let mut core = self.core.borrow_mut();
        let now = core.now();
        let Core { machine, host, accepted, .. } = &mut *core;
        host.pages = Self::pages(browser);
        for event in reported {
            match event {
                BrowserEvent::PageLoadStarted { view, .. } | BrowserEvent::PopupOpened { view, .. } => machine.page_load_started(host, PageId(view.0)),
                BrowserEvent::PageLoaded { view, .. } => machine.page_loaded(host, PageId(view.0), now),
                BrowserEvent::Script { call, json, .. } => {
                    if let Some(token) = self.answers.remove(&call) {
                        machine.script_answered(host, token, &json, now);
                    }
                }
                BrowserEvent::Dialog { .. } => machine.dialog_raised(),
                // The C++ page drops file:, javascript: and qrc: links without a word and takes every
                // other foreign scheme as a link to a transfer program (KaTopographicBrowser.cpp:70-78).
                BrowserEvent::NavigationRefused { url, why: Navigation::RefusedScheme, .. } => {
                    let scheme = url.split(':').next().unwrap_or_default().to_ascii_lowercase();
                    if !matches!(scheme.as_str(), "file" | "javascript" | "qrc" | "") {
                        machine.external_program_asked(host);
                    }
                }
                // The browser reports the files the logic said yes to in the order it was asked.
                BrowserEvent::DownloadStarted { id, .. } => {
                    if let Some(path) = accepted.pop_front() {
                        self.transfers.insert(id, path);
                    }
                }
                // The logic said yes and the browser could not place the file: the transfer is lost.
                BrowserEvent::DownloadRefused { why, .. } if why != DownloadRefusal::Declined => {
                    if let Some(path) = accepted.pop_front() {
                        machine.download_failed(host, &path);
                    }
                }
                BrowserEvent::DownloadProgress { id, bytes } => {
                    if let Some(path) = self.transfers.get(&id) {
                        machine.download_progress(host, path, bytes, now);
                    }
                }
                BrowserEvent::DownloadFinished { id, path, .. } => {
                    self.transfers.remove(&id);
                    machine.download_finished(host, &path, now);
                }
                BrowserEvent::DownloadFailed { id, .. } => {
                    if let Some(path) = self.transfers.remove(&id) {
                        machine.download_failed(host, &path);
                    }
                }
                _ => {}
            }
        }
        machine.tick(host, now);
        let wanted = std::mem::take(&mut host.wanted);
        let over = matches!(machine.stage(), Stage::Stopped | Stage::Complete);
        let events = machine.take_events();
        drop(core);
        if over {
            // Closing the session ends every transfer and leaves no unfinished file.
            if let Some(browser) = self.browser.take() {
                self.leftover = Some(browser.shut_down());
            }
            return events;
        }
        for want in wanted {
            match want {
                Wanted::Script { page, script, token } => {
                    if let Ok(call) = browser.run_script(ViewId(page.0), &script) {
                        self.answers.insert(call, token);
                    }
                }
                // A refused address is not retried: the logic's own limit says that the site did not answer.
                Wanted::Navigate(url) => {
                    let main = browser.main_view();
                    let _ = browser.navigate(main, &url);
                }
            }
        }
        events
    }
}
