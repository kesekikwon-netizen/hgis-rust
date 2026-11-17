//! Fresh browser generations (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:533-562`).

use super::{Core, HeritageRunHandle, RunOptions};
use crate::host::QueuedHost;
use raw_window_handle::HasWindowHandle;
use std::{cell::RefCell, rc::Rc, sync::Arc, time::Instant};
use strata_net::heritage_auto::HeritageAutomation;
use strata_webview::policy::NavigationPolicy;
use strata_webview::profile::{remove_session_folder, sweep_abandoned_sessions};
use strata_webview::{BrowserError, BrowserOptions, DownloadPlace, HiddenBrowser};

/// Starts a hidden run inside the existing application parent.
///
/// # Errors
/// Returns the actual browser or profile construction failure.
pub fn start_run<W: HasWindowHandle>(
    parent: &W,
    options: RunOptions,
) -> Result<HeritageRunHandle, BrowserError> {
    let _ = sweep_abandoned_sessions(&options.data_parent);
    let core = Rc::new(RefCell::new(Core {
        machine: HeritageAutomation::new(options.config.clone()),
        host: QueuedHost::default(),
        began: Instant::now(),
    }));
    let browser = open_browser(parent, &options, Rc::clone(&core))?;
    Ok(HeritageRunHandle {
        browser: Some(browser),
        core,
        options,
        leftover: None,
        pending_events: Vec::new(),
    })
}

fn open_browser<W: HasWindowHandle>(
    parent: &W,
    options: &RunOptions,
    core: Rc<RefCell<Core>>,
) -> Result<HiddenBrowser, BrowserError> {
    let mut browser = HiddenBrowser::start(
        parent,
        BrowserOptions {
            data_parent: options.data_parent.clone(),
            navigation: NavigationPolicy::new(
                Arc::clone(&options.network_allowed),
                options.config.fixture_base.as_deref(),
            ),
            document_ready_script: None,
            confirm_policy: strata_webview::dialogs::ConfirmPolicy::Accept,
        },
    )?;
    let epoch = core.borrow().host.epoch();
    let root = options.download_root.clone();
    let (sido, city) = (options.config.sido.clone(), options.config.city.clone());
    browser.set_download_decider(Box::new(move |offer| {
        let mut core = core.try_borrow_mut().ok()?;
        if core.host.epoch() != epoch || !core.machine.accepts_files() {
            return None;
        }
        let now = core.now();
        core.machine.observe_time(now);
        let directory = match crate::finish::allocate_received_folder(&root, &sido, &city) {
            Ok(directory) => directory,
            Err(error) => {
                core.machine.fail(format!(
                    "저장할 폴더가 없어 내려받기를 취소했습니다. ({error})"
                ));
                return None;
            }
        };
        let file_name = crate::finish::safe_name(&offer.suggested_name);
        core.machine.file_offered(directory.join(&file_name));
        Some(DownloadPlace {
            directory,
            file_name,
        })
    }));
    Ok(browser)
}

impl HeritageRunHandle {
    pub(super) fn close_session(&mut self) {
        if let Some(browser) = self.browser.take() {
            self.core.borrow_mut().host.reset_documents();
            self.leftover = Some(browser.shut_down());
        }
    }

    pub(super) fn refresh_session<W: HasWindowHandle>(&mut self, parent: &W) {
        if self.core.borrow().machine.session_restart_required() {
            self.close_session();
        }
        if self
            .leftover
            .as_ref()
            .is_some_and(|folder| remove_session_folder(folder))
        {
            self.leftover = None;
        }
        let due = {
            let core = self.core.borrow();
            core.machine.retry_due(core.now())
        };
        if !due || self.leftover.is_some() || self.browser.is_some() {
            return;
        }
        match open_browser(parent, &self.options, Rc::clone(&self.core)) {
            Ok(browser) => {
                let reopened = {
                    let mut core = self.core.borrow_mut();
                    let now = core.now();
                    core.machine.session_reopened(now)
                };
                if reopened {
                    self.browser = Some(browser);
                } else {
                    self.leftover = Some(browser.shut_down());
                }
            }
            Err(error) => self.core.borrow_mut().machine.fail(error.to_string()),
        }
    }
}
