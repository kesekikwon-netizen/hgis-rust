//! The checks of the probe, one step after another, each with a time limit.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use strata_webview::dialogs::DialogKind;
use strata_webview::downloads::DownloadRefusal;
use strata_webview::policy::{Navigation, NavigationPolicy};
use strata_webview::{BrowserError, BrowserEvent, BrowserOptions, DownloadPlace, HiddenBrowser, ScriptCall, ViewId};

use crate::site::{Site, FILE_BYTES};

const STEP_LIMIT: Duration = Duration::from_secs(15);

pub struct Probe {
    pub(crate) site: Site,
    pub(crate) other: Site,
    pub(crate) work: PathBuf,
    pub(crate) browser: Option<HiddenBrowser>,
    pub(crate) seen: Vec<BrowserEvent>,
    step: u32,
    entered: bool,
    pub(crate) step_started: Instant,
    pub(crate) call: Option<ScriptCall>,
    pub(crate) popup: Option<ViewId>,
    /// How far a step with several parts has come.
    pub(crate) part: u8,
    pub(crate) released: Option<PathBuf>,
    pub(crate) accept: Rc<Cell<bool>>,
    offered: Rc<RefCell<Vec<String>>>,
    pub outcome: Rc<RefCell<(u32, u32)>>,
}

impl Probe {
    pub fn new(site: Site, other: Site, work: PathBuf) -> Self {
        Self {
            site,
            other,
            work,
            browser: None,
            seen: Vec::new(),
            step: 0,
            entered: false,
            step_started: Instant::now(),
            call: None,
            popup: None,
            part: 0,
            released: None,
            accept: Rc::new(Cell::new(true)),
            offered: Rc::default(),
            outcome: Rc::default(),
        }
    }

    fn check(&mut self, name: &str, ok: bool) {
        println!("PROBE {} {name}", if ok { "ok  " } else { "FAIL" });
        let mut outcome = self.outcome.borrow_mut();
        if ok {
            outcome.0 += 1;
        } else {
            outcome.1 += 1;
        }
    }

    fn next(&mut self) {
        self.step += 1;
        self.entered = false;
        self.step_started = Instant::now();
        self.seen.clear();
        self.call = None;
        self.part = 0;
    }

    pub(crate) fn step_number(&self) -> u32 {
        self.step
    }

    pub(crate) fn run(&mut self, view: ViewId, script: &str) {
        self.call = self.browser.as_mut().and_then(|b| b.run_script(view, script).ok());
    }

    pub(crate) fn answer(&self) -> Option<&str> {
        let call = self.call?;
        self.seen.iter().find_map(|e| match e {
            BrowserEvent::Script { call: c, json, .. } if *c == call => Some(json.as_str()),
            _ => None,
        })
    }

    pub(crate) fn files(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.work.join("files"))
            .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    pub(crate) fn start(&mut self, frame: &eframe::Frame) -> Result<(), BrowserError> {
        let options = BrowserOptions {
            data_parent: self.work.join("profile"),
            // The network switch is off for the whole probe: only the local site can be reached.
            navigation: NavigationPolicy::new(Arc::new(|| false), Some(&self.site.origin())),
            document_ready_script: Some("window.__early = 'ready-script';".to_owned()),
        };
        let mut browser = HiddenBrowser::start(frame, options)?;
        let (accept, offered, folder) = (Rc::clone(&self.accept), Rc::clone(&self.offered), self.work.join("files"));
        browser.set_download_decider(Box::new(move |offer| {
            offered.borrow_mut().push(offer.suggested_name.clone());
            accept.get().then(|| DownloadPlace { directory: folder.clone(), file_name: offer.suggested_name.clone() })
        }));
        let main = browser.main_view();
        browser.navigate(main, &self.site.url("/main"))?;
        self.browser = Some(browser);
        Ok(())
    }

    /// One step: `enter` once, then `done` each frame until it answers.
    fn advance(&mut self, frame: &eframe::Frame) {
        let main = ViewId(1);
        let enter = !self.entered;
        self.entered = true;
        let done: Option<bool> = match self.step {
            0 => {
                if enter {
                    if let Err(e) = self.start(frame) {
                        println!("PROBE start error: {e}");
                        self.check("the session starts", false);
                        self.step = 99;
                        return;
                    }
                }
                let at = |want_start: bool| {
                    self.seen.iter().position(|e| match e {
                        BrowserEvent::PageLoadStarted { url, .. } => want_start && url.ends_with("/main"),
                        BrowserEvent::PageLoaded { url, .. } => !want_start && url.ends_with("/main"),
                        _ => false,
                    })
                };
                at(false).map(|loaded| at(true).is_some_and(|started| started < loaded))
            }
            1 => {
                if enter {
                    self.run(main, "window.__n + 1");
                }
                self.answer().map(|json| json == "42")
            }
            2 => {
                if enter {
                    self.run(main, "pop()");
                }
                let opened = self.seen.iter().find_map(|e| match e {
                    BrowserEvent::PopupOpened { opener, view, .. } if *opener == main => Some(*view),
                    _ => None,
                });
                let loaded = opened.is_some_and(|p| self.seen.iter().any(|e| matches!(e, BrowserEvent::PageLoaded { view, url } if *view == p && url.ends_with("/pop"))));
                if let (Some("true"), Some(p), true) = (self.answer(), opened, loaded) {
                    self.popup = Some(p);
                    let views = self.browser.as_ref().map(HiddenBrowser::views).unwrap_or_default();
                    Some(views.len() == 2 && views.iter().any(|v| v.id == p && v.opener == Some(main) && v.url.ends_with("/pop")))
                } else {
                    None
                }
            }
            3 => {
                if enter {
                    let popup = self.popup.unwrap_or(main);
                    self.run(popup, "document.title");
                }
                self.answer().map(|json| json == "\"pop:downForm:41\"")
            }
            4 => {
                if enter {
                    self.run(main, "ask()");
                }
                let said = |kind: DialogKind, text: &str| self.seen.iter().any(|e| matches!(e, BrowserEvent::Dialog { kind: k, text: t, .. } if *k == kind && t == text));
                let all = said(DialogKind::Confirm, "sure?") && said(DialogKind::Alert, "answer false") && said(DialogKind::Prompt, "name?");
                self.answer().filter(|_| all).map(|json| json == "[false,null]")
            }
            5 => {
                if enter {
                    self.accept.set(true);
                    self.run(main, "location.href = '/file.zip'; 1");
                }
                let finished = self.seen.iter().find_map(|e| match e {
                    BrowserEvent::DownloadFinished { path, bytes, .. } => Some((path.clone(), *bytes)),
                    _ => None,
                });
                finished.map(|(path, bytes)| {
                    let started = self.seen.iter().any(|e| matches!(e, BrowserEvent::DownloadStarted { file_name, .. } if file_name == "_evil name_.zip"));
                    let on_disk = std::fs::metadata(&path).map_or(0, |m| m.len());
                    started
                        && bytes == FILE_BYTES as u64
                        && on_disk == FILE_BYTES as u64
                        && path == self.work.join("files").join("_evil name_.zip")
                        && self.offered.borrow().first().map(String::as_str) == Some("_evil name_.zip")
                        && self.files() == vec!["_evil name_.zip".to_owned()]
                })
            }
            6 => {
                if enter {
                    self.accept.set(false);
                    self.run(main, "location.href = '/file.zip?again'; 1");
                }
                self.seen
                    .iter()
                    .any(|e| matches!(e, BrowserEvent::DownloadRefused { why: DownloadRefusal::Declined, .. }))
                    .then(|| self.files() == vec!["_evil name_.zip".to_owned()])
            }
            7 => {
                if enter {
                    let refused = self.browser.as_mut().map(|b| b.navigate(main, "https://example.org/"));
                    let ok = refused == Some(Err(BrowserError::Refused(Navigation::RefusedOffline)));
                    self.check("a web address is refused while the network switch is off", ok);
                    self.run(main, "location.href = 'mailto:nobody@example.org'; 1");
                }
                self.seen.iter().any(|e| matches!(e, BrowserEvent::NavigationRefused { why: Navigation::RefusedScheme, .. })).then_some(true)
            }
            8 => {
                if enter {
                    self.run(main, "location.href = 'http://127.0.0.1:9/elsewhere'; 1");
                }
                self.seen.iter().any(|e| matches!(e, BrowserEvent::NavigationRefused { why: Navigation::RefusedOffline, url, .. } if url.contains(":9/"))).then_some(true)
            }
            9 => {
                if enter {
                    let url = self.site.url("/ready");
                    let _ = self.browser.as_mut().map(|b| b.navigate(main, &url));
                }
                let loaded = self.seen.iter().any(|e| matches!(e, BrowserEvent::PageLoaded { url, .. } if url.ends_with("/ready")));
                if loaded && self.call.is_none() {
                    self.run(main, "document.title");
                }
                self.answer().map(|json| json == "\"onload saw ready-script/ready-script/undefined\"")
            }
            10..=15 => crate::steps_more::advance(self, enter),
            16..=18 => crate::steps_end::advance(self, enter, frame),
            _ => {
                self.step = 99;
                return;
            }
        };
        const NAMES: [&str; 19] = [
            "the local site loads in a hidden view with the network switch off",
            "a script's value comes back as JSON",
            "a popup becomes a second hidden view with its opener",
            "the popup page sees its window name and its opener's data",
            "confirm, alert and prompt are answered silently and reported",
            "a file is written to a part file and put in place under a safe name",
            "a refused file leaves nothing behind",
            "a non-web address is refused and reported",
            "a page cannot leave for a web address while the switch is off",
            "the ready script runs when the document is parsed, in the main frame only",
            "a hidden view lays its page out at desktop size",
            "a popup that closes itself is reported and its view is gone",
            "a new popup opens afterwards and its opener can close it",
            "no request reaches another address while the switch is off",
            "the first empty page of a popup has quiet dialogs too",
            "a javascript: link runs in its page and is no navigation",
            "a refused second offer leaves the running transfer alone, then the session is shut down",
            "shut down leaves no part file and the data folder can be removed",
            "a session that is simply dropped lets go of its folder too",
        ];
        let name = NAMES[self.step as usize];
        match done {
            Some(ok) => {
                self.check(name, ok);
                self.next();
            }
            None if self.step_started.elapsed() > STEP_LIMIT => {
                println!("PROBE timeout in step {}; last events: {:?}; offered {:?}; files {:?}; released {:?}", self.step, self.seen.iter().rev().take(4).collect::<Vec<_>>(), self.offered.borrow(), self.files(), self.released);
                self.check(name, false);
                self.next();
            }
            None => {}
        }
    }
}

impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("strata-webview probe: step {}", self.step));
        });
        if let Some(browser) = self.browser.as_mut() {
            let events = browser.pump(&*frame);
            self.seen.extend(events);
        }
        if self.step == 99 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.advance(frame);
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}
