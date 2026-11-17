//! The run: a hidden browser session and the stage logic, driven one frame at a time.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use raw_window_handle::HasWindowHandle;

use strata_net::heritage_auto::{HeritageAutomation, HeritageConfig, HeritageEvent, PageId};

use strata_webview::{BrowserEvent, HiddenBrowser};

use crate::host::{QueuedHost, Wanted};
#[path = "run_session.rs"]
mod session;
pub use session::start_run;

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
    /// The data folder of a closed session that the engine has not let go of yet.
    leftover: Option<PathBuf>,
    pending_events: Vec<HeritageEvent>,
    options: RunOptions,
}

impl std::fmt::Debug for HeritageRunHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeritageRunHandle")
            .field("open", &self.browser.is_some())
            .finish_non_exhaustive()
    }
}

impl HeritageRunHandle {
    /// Queues the app's actual import outcome without browser reentrancy (`KaHeritageBrowser.cpp:1206-1217`).
    pub fn dataset_loaded(
        &mut self,
        files: &[PathBuf],
        outcome: strata_net::heritage_auto::DatasetOutcome,
    ) -> bool {
        let mut core = self.core.borrow_mut();
        let now = core.now();
        core.machine.observe_time(now);
        core.machine.dataset_loaded(files, outcome)
    }

    /// Cancels navigation and transfers (`KaHeritageBrowser.cpp:479-489`).
    /// The next frame reports cancellation and continues profile cleanup.
    pub fn cancel(&mut self) {
        if let Some(event) = self.core.borrow_mut().machine.cancel() {
            self.pending_events.push(event);
        }
        self.close_session();
    }

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
        self.core.borrow().machine.is_over()
            && self.browser.is_none()
            && self.leftover.is_none()
            && self.pending_events.is_empty()
    }

    /// One frame: what the browser reports goes to the stage logic, what the logic wants goes
    /// to the browser. Returns what happened in the run.
    pub fn frame<W: HasWindowHandle>(&mut self, parent: &W) -> Vec<HeritageEvent> {
        self.refresh_session(parent);
        let mut events_extra = std::mem::take(&mut self.pending_events);
        let Some(browser) = self.browser.as_mut() else {
            let mut core = self.core.borrow_mut();
            let now = core.now();
            events_extra.extend(core.machine.timers(now));
            return events_extra;
        };
        let epoch = self.core.borrow().host.epoch();
        let reported = browser.pump(parent);
        let mut core = self.core.borrow_mut();
        let now = core.now();
        let Core { machine, host, .. } = &mut *core;
        machine.observe_time(now);
        host.pages = pages(browser);
        for event in reported {
            match event {
                BrowserEvent::Script { call, json, .. } => {
                    if !host.record_probe(epoch, call, &json) {
                        events_extra.extend(host.action_answer(epoch, call, &json, machine));
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
                    }
                }
                BrowserEvent::DownloadFailed { .. } => {
                    events_extra
                        .extend(machine.transfer_failed("파일을 끝까지 받지 못했습니다", now));
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
        let restart = machine.session_restart_required();
        drop(core);
        if over || restart {
            // Closing the session ends every transfer and leaves no unfinished file.
            self.close_session();
            return events_extra;
        }
        // Opening the portal is a navigation, not a script; ask the machine for the next step.
        {
            let mut core = self.core.borrow_mut();
            let Core { machine, host, .. } = &mut *core;
            host.pages = pages(browser);
            if let Some(call) = machine.next_step(host, now) {
                host.begin(browser, machine, call);
            }
        }
        let mut core = self.core.borrow_mut();
        let Core { host, .. } = &mut *core;
        let extra = std::mem::take(&mut host.wanted);
        drop(core);
        for want in wanted.into_iter().chain(extra) {
            match want {
                Wanted::Script {
                    page,
                    script,
                    token,
                } => {
                    let mut core = self.core.borrow_mut();
                    let Core { machine, host, .. } = &mut *core;
                    host.begin(
                        browser,
                        machine,
                        strata_net::heritage_auto::ScriptCall {
                            page,
                            script,
                            token,
                        },
                    );
                }
                Wanted::Navigate(url) => {
                    let main = browser.main_view();
                    let _ = browser.navigate(main, &url);
                }
            }
        }
        let mut core = self.core.borrow_mut();
        let Core { machine, host, .. } = &mut *core;
        events_extra.extend(host.advance(browser, machine));
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
