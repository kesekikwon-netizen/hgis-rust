//! The run: a hidden browser session and the stage logic, driven one frame at a time.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use raw_window_handle::HasWindowHandle;

use strata_net::heritage_auto::{HeritageAutomation, HeritageConfig, HeritageEvent, PageId};
use strata_webview::policy::NavigationPolicy;
use strata_webview::profile::{remove_session_folder, sweep_abandoned_sessions};
use strata_webview::{BrowserError, BrowserEvent, BrowserOptions, DownloadPlace, HiddenBrowser, ScriptCall, ViewId};

use crate::host::{QueuedHost, Wanted};

/// How a run is set up.
#[derive(Clone)]
pub struct RunOptions {
    /// The folder under which the browser session makes its data folder, removed when the run
    /// ends. Not the survey folder: a run must not write beside the user's data.
    pub data_parent: PathBuf,
    pub config: HeritageConfig,
    /// The app's network switch.
    pub network_allowed: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Where received files go: `<this>/주변유적/<시도> <시군>/원본/…`. The app's own data
    /// folder, never the survey folder (`MainWindow.cpp:3168-3170`).
    pub download_root: PathBuf,
}

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("data_parent", &self.data_parent)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

struct Core {
    machine: HeritageAutomation,
    host: QueuedHost,
    began: Instant,
}

impl Core {
    fn now(&self) -> u64 {
        u64::try_from(self.began.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// A heritage run. `frame` is called once per frame while it lives; when the run stops or
/// finishes the session is closed and its data folder removed.
pub struct HeritageRunHandle {
    browser: Option<HiddenBrowser>,
    core: Rc<RefCell<Core>>,
    answers: HashMap<ScriptCall, u64>,
    /// The data folder of a closed session that the engine has not let go of yet.
    leftover: Option<PathBuf>,
}

impl std::fmt::Debug for HeritageRunHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeritageRunHandle")
            .field("open", &self.browser.is_some())
            .finish_non_exhaustive()
    }
}

/// Starts a run inside `parent` (the app's window). Nothing is asked of the site until `frame`.
///
/// # Errors
///
/// When the browser session cannot be made (WebView2 missing, or the data folder unwritable).
pub fn start_run<W: HasWindowHandle>(parent: &W, options: RunOptions) -> Result<HeritageRunHandle, BrowserError> {
    // A run that died with the app left its session folder behind.
    let _ = sweep_abandoned_sessions(&options.data_parent);
    // The fixture is on this PC; it stays reachable while the network switch is off
    // (the same rule `strata-ngii-run` uses).
    let local = options.config.fixture_base.clone();
    let mut browser = HiddenBrowser::start(
        parent,
        BrowserOptions {
            data_parent: options.data_parent.clone(),
            navigation: NavigationPolicy::new(options.network_allowed, local.as_deref()),
            document_ready_script: None,
        },
    )?;
    let core = Rc::new(RefCell::new(Core {
        machine: HeritageAutomation::new(options.config.clone()),
        host: QueuedHost::default(),
        began: Instant::now(),
    }));
    // A file the site offers is accepted only while the run is fetching a kind, and it goes
    // under 주변유적 (never the survey folder). The C++ refuses one at any other stage
    // (`KaHeritageBrowser.cpp:1295-1302`).
    let decide = Rc::clone(&core);
    let root = options.download_root.clone();
    let (sido, city) = (
        options.config.sido.clone(),
        options.config.city.clone(),
    );
    browser.set_download_decider(Box::new(move |offer| {
        let mut core = decide.try_borrow_mut().ok()?;
        if !core.machine.accepts_files() {
            return None;
        }
        let serial = core.now();
        let directory = crate::finish::received_folder(&root, &sido, &city, serial);
        std::fs::create_dir_all(&directory).ok()?;
        let file_name = crate::finish::safe_name(&offer.suggested_name);
        core.machine.file_offered(directory.join(&file_name));
        Some(DownloadPlace {
            directory,
            file_name,
        })
    }));
    Ok(HeritageRunHandle {
        browser: Some(browser),
        core,
        answers: HashMap::new(),
        leftover: None,
    })
}

impl HeritageRunHandle {
    /// The stage the run is in.
    #[must_use]
    pub fn stage(&self) -> strata_net::heritage_auto::HeritageStage {
        self.core.borrow().machine.stage().clone()
    }

    /// The words to put in the status line.
    #[must_use]
    pub fn progress_words(&self) -> String {
        self.core.borrow().machine.progress_words()
    }

    /// Whether the run is over and nothing of its session is left.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.browser.is_none() && self.leftover.is_none()
    }

    /// One frame: what the browser reports goes to the stage logic, what the logic wants goes
    /// to the browser. Returns what happened in the run.
    pub fn frame<W: HasWindowHandle>(&mut self, parent: &W) -> Vec<HeritageEvent> {
        if let Some(folder) = &self.leftover {
            if remove_session_folder(folder) {
                self.leftover = None;
            }
        }
        let Some(browser) = self.browser.as_mut() else {
            return Vec::new();
        };
        let reported = browser.pump(parent);
        let mut events_extra: Vec<HeritageEvent> = Vec::new();
        let mut core = self.core.borrow_mut();
        let now = core.now();
        let Core { machine, host, .. } = &mut *core;
        host.pages = pages(browser);
        for event in reported {
            match event {
                BrowserEvent::Script { call, json, .. } => {
                    if let Some(token) = self.answers.remove(&call) {
                        events_extra.extend(machine.script_answered(token, &json));
                    }
                }
                // The C++ reads the finished file's size against what the browser reported
                // and asks again when they differ (`:1370-1382`).
                BrowserEvent::DownloadFinished { path, .. } => {
                    if !crate::finish::looks_like_zip(&path) {
                        let reason = "받은 파일이 압축 파일이 아닙니다";
                        events_extra.extend(machine.transfer_failed(reason, now));
                    } else {
                        events_extra.extend(machine.zip_arrived(path));
                        // One file finishes a kind (the C++ moves on as soon as the file is
                        // complete, `KaHeritageBrowser.cpp:1396-1400`).
                        events_extra.extend(machine.kind_done());
                    }
                }
                BrowserEvent::DownloadFailed { .. } => {
                    events_extra.extend(machine.transfer_failed("파일을 끝까지 받지 못했습니다", now));
                }
                BrowserEvent::DownloadRefused { .. } => {
                    events_extra.extend(machine.transfer_failed("파일을 저장하지 못했습니다", now));
                }
                // The stage machine reads the page through scripts, not through load events.
                _ => {}
            }
        }
        let wanted = std::mem::take(&mut host.wanted);
        // The run's own timers: a retry that has come due, and the wait for the panel.
        events_extra.extend(machine.timers(now));
        let over = machine.is_over();
        drop(core);
        if over {
            // Closing the session ends every transfer and leaves no unfinished file.
            if let Some(browser) = self.browser.take() {
                self.leftover = Some(browser.shut_down());
            }
            return events_extra;
        }
        // Opening the portal is a navigation, not a script; ask the machine for the next step.
        {
            let mut core = self.core.borrow_mut();
            let Core { machine, host, .. } = &mut *core;
            host.pages = pages(browser);
            if let Some(call) = machine.next_step(host, now) {
                if let Ok(script_call) = browser.run_script(ViewId(call.page.0), &call.script) {
                    self.answers.insert(script_call, call.token);
                }
            }
        }
        let mut core = self.core.borrow_mut();
        let Core { host, .. } = &mut *core;
        let extra = std::mem::take(&mut host.wanted);
        drop(core);
        for want in wanted.into_iter().chain(extra) {
            match want {
                Wanted::Script { page, script, token } => {
                    if let Ok(call) = browser.run_script(ViewId(page.0), &script) {
                        self.answers.insert(call, token);
                    }
                }
                Wanted::Navigate(url) => {
                    let main = browser.main_view();
                    let _ = browser.navigate(main, &url);
                }
            }
        }
        events_extra
    }
}

fn pages(browser: &HiddenBrowser) -> Vec<strata_net::heritage_auto::PageRef> {
    browser
        .views()
        .into_iter()
        .map(|v| strata_net::heritage_auto::PageRef {
            id: PageId(v.id.0),
            url: v.url,
            opener: v.opener.map(|o| PageId(o.0)),
        })
        .collect()
}
