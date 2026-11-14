//! One hidden browser session: a private profile in its own data folder, a first view, popups as further
//! views, scripts, downloads. It lives on the UI thread as a child of the app's window and is driven by
//! `pump`, which the app calls once per frame while the session is open. Nothing here waits.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};

use raw_window_handle::HasWindowHandle;
use wry::{WebContext, WebView};

use crate::dialogs::parse_page_message;
use crate::downloads::{discard_download, finish_download, received_bytes, PlannedFile};
use crate::events::{BrowserError, BrowserEvent, DownloadDecider, DownloadId, ScriptCall, ViewId, ViewInfo};
use crate::policy::{Navigation, NavigationPolicy};
use crate::profile::SessionFolder;
use crate::view::{build_hidden_view, Engine, Shared, Signal};

/// Spare views kept ready for popups (a login chain opens one window after another).
const SPARE_VIEWS: usize = 2;

/// How a session is set up.
#[derive(Debug, Clone)]
pub struct BrowserOptions {
    /// Folder under which the session makes its own data folder (see `profile`).
    pub data_parent: PathBuf,
    pub navigation: NavigationPolicy,
    /// Script run in the main frame of every page once its document is parsed.
    pub document_ready_script: Option<String>,
}

struct OpenView {
    id: ViewId,
    opener: Option<ViewId>,
    view: WebView,
}

struct Transfer {
    id: DownloadId,
    url: String,
    plan: PlannedFile,
    reported: u64,
}

/// The session. Dropping it closes every view; `shut_down` also hands back the data folder for removal.
pub struct HiddenBrowser {
    views: Vec<OpenView>,
    shared: Rc<Shared>,
    tx: Sender<Signal>,
    rx: Receiver<Signal>,
    transfers: Vec<Transfer>,
    next_view: u32,
    next_call: u64,
    // Declared after the views: the engine's context and the folder's lock outlive every view.
    _context: WebContext,
    folder: SessionFolder,
}

impl std::fmt::Debug for HiddenBrowser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HiddenBrowser").field("views", &self.views.len()).field("transfers", &self.transfers.len()).finish_non_exhaustive()
    }
}

impl HiddenBrowser {
    /// Starts a session with one empty hidden view inside `parent` (the app's window).
    pub fn start<W: HasWindowHandle>(parent: &W, options: BrowserOptions) -> Result<Self, BrowserError> {
        let folder = SessionFolder::create(&options.data_parent).map_err(|e| BrowserError::DataFolder(e.to_string()))?;
        let mut context = WebContext::new(Some(folder.path().to_path_buf()));
        let (tx, rx) = channel();
        let shared = Rc::new(Shared {
            policy: options.navigation,
            spare: RefCell::new(Vec::new()),
            adopted: RefCell::new(Vec::new()),
            decider: RefCell::new(None),
            next_download: Cell::new(1),
            ready_script: options.document_ready_script,
        });
        let first = build_hidden_view(parent, ViewId(1), Engine::Fresh(&mut context), &shared, &tx)?;
        let mut session = Self {
            views: vec![OpenView { id: ViewId(1), opener: None, view: first }],
            shared,
            tx,
            rx,
            transfers: Vec::new(),
            next_view: 2,
            next_call: 1,
            _context: context,
            folder,
        };
        session.refill_spares(parent);
        Ok(session)
    }

    #[must_use]
    pub fn main_view(&self) -> ViewId {
        ViewId(1)
    }

    /// The open views with the address each shows now.
    #[must_use]
    pub fn views(&self) -> Vec<ViewInfo> {
        self.views.iter().map(|v| ViewInfo { id: v.id, url: v.view.url().unwrap_or_default(), opener: v.opener }).collect()
    }

    fn view(&self, id: ViewId) -> Result<&WebView, BrowserError> {
        self.views.iter().find(|v| v.id == id).map(|v| &v.view).ok_or(BrowserError::NoSuchView(id))
    }

    /// Sends a view to an address the session's rule allows.
    pub fn navigate(&mut self, view: ViewId, url: &str) -> Result<(), BrowserError> {
        match self.shared.policy.judge(url) {
            Navigation::Allow => self.view(view)?.load_url(url).map_err(|e| BrowserError::Engine(e.to_string())),
            why => Err(BrowserError::Refused(why)),
        }
    }

    /// Runs a script in the main frame of a view. Its value comes back as `BrowserEvent::Script` (JSON text).
    pub fn run_script(&mut self, view: ViewId, script: &str) -> Result<ScriptCall, BrowserError> {
        let call = ScriptCall(self.next_call);
        self.next_call += 1;
        let tx = self.tx.clone();
        self.view(view)?
            .evaluate_script_with_callback(script, move |json| {
                let _ = tx.send(Signal::Script { view, call, json });
            })
            .map_err(|e| BrowserError::Engine(e.to_string()))?;
        Ok(call)
    }

    /// Who decides where an offered file goes. Without a decider every file is refused.
    pub fn set_download_decider(&mut self, decider: DownloadDecider) {
        *self.shared.decider.borrow_mut() = Some(decider);
    }

    /// Closes one view (never the first: end the session instead).
    pub fn close_view(&mut self, view: ViewId) {
        if view != self.main_view() {
            self.views.retain(|v| v.id != view);
        }
    }

    fn refill_spares<W: HasWindowHandle>(&mut self, parent: &W) {
        while self.shared.spare.borrow().len() < SPARE_VIEWS {
            let id = ViewId(self.next_view);
            let Some(first) = self.views.first() else {
                return;
            };
            match build_hidden_view(parent, id, Engine::SameAs(&first.view), &self.shared, &self.tx) {
                Ok(view) => {
                    self.next_view += 1;
                    self.shared.spare.borrow_mut().push((id, view));
                }
                // Without a spare view the next popup is refused and reported; nothing else breaks.
                Err(_) => return,
            }
        }
    }

    /// Collects what happened since the last call, takes over popups, keeps spare views ready and measures
    /// running transfers. Call once per frame from the UI thread.
    pub fn pump<W: HasWindowHandle>(&mut self, parent: &W) -> Vec<BrowserEvent> {
        let adopted: Vec<_> = self.shared.adopted.borrow_mut().drain(..).collect();
        for (id, opener, view) in adopted {
            self.views.push(OpenView { id, opener: Some(opener), view });
        }
        self.refill_spares(parent);
        let mut events = Vec::new();
        while let Ok(signal) = self.rx.try_recv() {
            match signal {
                Signal::Loaded { view, url } => events.push(BrowserEvent::PageLoaded { view, url }),
                Signal::Script { view, call, json } => events.push(BrowserEvent::Script { call, view, json }),
                Signal::PageMessage { view, body } => {
                    if let Some(notice) = parse_page_message(&body) {
                        events.push(BrowserEvent::Dialog { view, kind: notice.kind, text: notice.text });
                    }
                }
                Signal::Refused { view, url, why } => events.push(BrowserEvent::NavigationRefused { view, url, why }),
                Signal::PopupOpened { opener, view, url } => events.push(BrowserEvent::PopupOpened { opener, view, url }),
                Signal::PopupRefused { opener, url } => events.push(BrowserEvent::PopupRefused { opener, url }),
                Signal::DownloadStarted { id, view, url, plan, file_name } => {
                    self.transfers.push(Transfer { id, url: url.clone(), plan, reported: 0 });
                    events.push(BrowserEvent::DownloadStarted { id, view, url, file_name });
                }
                Signal::DownloadRefused { view, url, why } => events.push(BrowserEvent::DownloadRefused { view, url, why }),
                Signal::DownloadEnded { part_path, url, success } => {
                    let at = self.transfers.iter().position(|t| part_path.as_ref().map_or(t.url == url, |p| *p == t.plan.part_path));
                    if let Some(transfer) = at.map(|i| self.transfers.remove(i)) {
                        match success.then(|| finish_download(&transfer.plan)) {
                            Some(Ok(bytes)) => events.push(BrowserEvent::DownloadFinished { id: transfer.id, path: transfer.plan.final_path, bytes }),
                            _ => {
                                discard_download(&transfer.plan);
                                events.push(BrowserEvent::DownloadFailed { id: transfer.id, url: transfer.url });
                            }
                        }
                    }
                }
            }
        }
        for transfer in &mut self.transfers {
            let bytes = received_bytes(&transfer.plan);
            if bytes != transfer.reported {
                transfer.reported = bytes;
                events.push(BrowserEvent::DownloadProgress { id: transfer.id, bytes });
            }
        }
        events
    }

    /// Ends the session: every view is closed, unfinished transfers leave nothing behind, and the data folder
    /// is handed back. The engine's helper processes may hold files in it for a moment: the host calls
    /// `profile::remove_session_folder` now and again later; the sweep at the next start is the last resort.
    #[must_use]
    pub fn shut_down(self) -> PathBuf {
        let Self { views, shared, transfers, _context, folder, .. } = self;
        drop(views);
        shared.spare.borrow_mut().clear();
        shared.adopted.borrow_mut().clear();
        for transfer in &transfers {
            discard_download(&transfer.plan);
        }
        drop(_context);
        folder.release()
    }
}
