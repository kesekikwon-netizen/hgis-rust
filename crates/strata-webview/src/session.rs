//! One hidden browser session: a private profile in its own data folder, a first view, popups as further
//! views, scripts, downloads. It lives on the UI thread as a child of the app's window and is driven by
//! `pump`, which the app calls once per frame while the session is open. Nothing here sleeps; building a
//! view does take the engine a few hundred milliseconds, and `pump` builds at most one per call.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
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
/// After so many failures in a row no further spare view is tried; popups are then refused and reported.
const SPARE_ATTEMPTS: u8 = 3;

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

/// The session. Dropping it closes every view and leaves no unfinished file; `shut_down` also says which
/// data folder to remove.
pub struct HiddenBrowser {
    views: Vec<OpenView>,
    shared: Rc<Shared>,
    tx: Sender<Signal>,
    rx: Receiver<Signal>,
    transfers: Vec<Transfer>,
    /// Addresses of offers that were refused: the engine reports their end too, and that end must not be
    /// taken for the end of a running transfer of the same address (review of 2026-11-15, S2).
    refused_ends: Vec<String>,
    spare_failures: u8,
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

impl Drop for HiddenBrowser {
    fn drop(&mut self) {
        self.close_everything();
    }
}

impl HiddenBrowser {
    /// Starts a session with one empty hidden view inside `parent` (the app's window). When the network
    /// switch is off at this moment the engine is started without a way out (see `policy`).
    pub fn start<W: HasWindowHandle>(parent: &W, options: BrowserOptions) -> Result<Self, BrowserError> {
        let folder = SessionFolder::create(&options.data_parent).map_err(|e| BrowserError::DataFolder(e.to_string()))?;
        let mut context = WebContext::new(Some(folder.path().to_path_buf()));
        let (tx, rx) = channel();
        let arguments = options.navigation.offline_engine_arguments();
        let shared = Rc::new(Shared {
            policy: options.navigation,
            spare: RefCell::new(Vec::new()),
            adopted: RefCell::new(Vec::new()),
            decider: RefCell::new(None),
            next_download: Cell::new(1),
            ready_script: options.document_ready_script,
            messages: Cell::new(0),
        });
        let first = build_hidden_view(parent, ViewId(1), Engine::Fresh { context: &mut context, arguments }, &shared, &tx)?;
        let mut session = Self {
            views: vec![OpenView { id: ViewId(1), opener: None, view: first }],
            shared,
            tx,
            rx,
            transfers: Vec::new(),
            refused_ends: Vec::new(),
            spare_failures: 0,
            next_view: 2,
            next_call: 1,
            _context: context,
            folder,
        };
        for _ in 0..SPARE_VIEWS {
            session.add_spare(parent);
        }
        Ok(session)
    }

    #[must_use]
    pub fn main_view(&self) -> ViewId {
        ViewId(1)
    }

    /// The session's data folder (to remove with `profile::remove_session_folder` when the session is over).
    #[must_use]
    pub fn data_folder(&self) -> &Path {
        self.folder.path()
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
    /// A page that never answers gets no event: the caller keeps its own time limit.
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

    /// Who decides where an offered file goes. Without a decider every file is refused. The engine does not
    /// say how large a file will be and a running transfer cannot be stopped on its own: a caller that
    /// wants a size limit watches `DownloadProgress` and ends the session.
    pub fn set_download_decider(&mut self, decider: DownloadDecider) {
        if let Ok(mut slot) = self.shared.decider.try_borrow_mut() {
            *slot = Some(decider);
        }
    }

    /// Closes one view (never the first: end the session instead).
    pub fn close_view(&mut self, view: ViewId) {
        if view != self.main_view() {
            self.views.retain(|v| v.id != view);
        }
    }

    /// Builds one spare view when one is missing.
    fn add_spare<W: HasWindowHandle>(&mut self, parent: &W) {
        if self.spare_failures >= SPARE_ATTEMPTS || self.shared.spare.borrow().len() >= SPARE_VIEWS {
            return;
        }
        let Some(first) = self.views.first() else {
            return;
        };
        let id = ViewId(self.next_view);
        match build_hidden_view(parent, id, Engine::SameAs(&first.view), &self.shared, &self.tx) {
            Ok(view) => {
                self.next_view += 1;
                self.spare_failures = 0;
                self.shared.spare.borrow_mut().push((id, view));
            }
            // Without a spare view the next popup is refused and reported; nothing else breaks.
            Err(_) => self.spare_failures += 1,
        }
    }

    /// Collects what happened since the last call, takes over popups, keeps spare views ready and measures
    /// running transfers. Call once per frame from the UI thread.
    pub fn pump<W: HasWindowHandle>(&mut self, parent: &W) -> Vec<BrowserEvent> {
        self.shared.messages.set(0);
        // Building a view lets the window handle messages, and a page may open a popup just then. Popups
        // are taken over after that, so that every view an event below speaks of is already listed.
        self.add_spare(parent);
        let adopted: Vec<_> = self.shared.adopted.borrow_mut().drain(..).collect();
        for (id, opener, view) in adopted {
            self.views.push(OpenView { id, opener: Some(opener), view });
        }
        let mut events = Vec::new();
        while let Ok(signal) = self.rx.try_recv() {
            match signal {
                Signal::LoadStarted { view, url } => events.push(BrowserEvent::PageLoadStarted { view, url }),
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
                Signal::DownloadRefused { view, url, why } => {
                    self.refused_ends.push(url.clone());
                    events.push(BrowserEvent::DownloadRefused { view, url, why });
                }
                Signal::DownloadEnded { part_path, url, success } => self.transfer_ended(part_path, &url, success, &mut events),
            }
        }
        // When a page closes its window, wry destroys the view's container window on the engine's close
        // request (wry 0.57.1 webview2/mod.rs:694-697) and leaves the page itself alive. A view whose
        // container is gone cannot say where it is; dropping it here closes the page too.
        let main = self.main_view();
        let gone: Vec<ViewId> = self.views.iter().filter(|v| v.id != main && v.view.bounds().is_err()).map(|v| v.id).collect();
        self.views.retain(|v| !gone.contains(&v.id));
        events.extend(gone.into_iter().map(|view| BrowserEvent::ViewClosed { view }));
        for transfer in &mut self.transfers {
            let bytes = received_bytes(&transfer.plan);
            if bytes != transfer.reported {
                transfer.reported = bytes;
                events.push(BrowserEvent::DownloadProgress { id: transfer.id, bytes });
            }
        }
        events
    }

    /// The engine says a download ended. A complete one names its file; one that broke off or was refused
    /// names only its address.
    fn transfer_ended(&mut self, part_path: Option<PathBuf>, url: &str, success: bool, events: &mut Vec<BrowserEvent>) {
        if part_path.is_none() {
            if let Some(at) = self.refused_ends.iter().position(|refused| refused == url) {
                self.refused_ends.remove(at);
                return;
            }
        }
        let at = self.transfers.iter().position(|t| part_path.as_ref().map_or(t.url == url, |p| *p == t.plan.part_path));
        let Some(transfer) = at.map(|i| self.transfers.remove(i)) else {
            return;
        };
        match success.then(|| finish_download(&transfer.plan)) {
            Some(Ok(bytes)) => events.push(BrowserEvent::DownloadFinished { id: transfer.id, path: transfer.plan.final_path, bytes }),
            _ => {
                discard_download(&transfer.plan);
                events.push(BrowserEvent::DownloadFailed { id: transfer.id, url: transfer.url });
            }
        }
    }

    /// Closes every view (the spare ones hold the shared state that holds them: review of 2026-11-15, S4)
    /// and removes what unfinished transfers left.
    fn close_everything(&mut self) {
        self.views.clear();
        if let Ok(mut spare) = self.shared.spare.try_borrow_mut() {
            spare.clear();
        }
        if let Ok(mut adopted) = self.shared.adopted.try_borrow_mut() {
            adopted.clear();
        }
        if let Ok(mut decider) = self.shared.decider.try_borrow_mut() {
            *decider = None;
        }
        for transfer in self.transfers.drain(..) {
            discard_download(&transfer.plan);
        }
    }

    /// Ends the session and hands back the data folder. The engine's helper processes may hold files in it
    /// for a moment: the host calls `profile::remove_session_folder` now and again later; the sweep at the
    /// next start is the last resort.
    #[must_use]
    pub fn shut_down(mut self) -> PathBuf {
        self.close_everything();
        self.folder.path().to_path_buf()
    }
}
