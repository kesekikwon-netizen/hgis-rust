//! Builds one hidden view and wires its callbacks. Callbacks run on the UI thread while the app's window
//! handles messages; they only put a signal into a channel or hand over a spare view, and `session::pump`
//! does the rest.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use raw_window_handle::HasWindowHandle;
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{
    NewWindowResponse, PageLoadEvent, PermissionKind, PermissionResponse, Rect, WebContext, WebView, WebViewBuilder,
    WebViewBuilderExtWindows, WebViewExtWindows,
};

use crate::dialogs::{quiet_dialogs_script, ConfirmPolicy};
use crate::downloads::{held_back_without_click, plan_download, DownloadRefusal, PlannedFile};
use crate::events::{BrowserError, DownloadDecider, DownloadId, DownloadOffer, ScriptCall, ViewId};
use crate::names::safe_file_name;
use crate::policy::{Navigation, NavigationPolicy};

/// What a callback reports. Everything in it can cross threads (the script answer arrives on a `Send` callback).
#[derive(Debug)]
pub(crate) enum Signal {
    LoadStarted { view: ViewId, url: String },
    Loaded { view: ViewId, url: String },
    Script { view: ViewId, call: ScriptCall, json: String },
    PageMessage { view: ViewId, body: String },
    Refused { view: ViewId, url: String, why: Navigation },
    PopupOpened { opener: ViewId, view: ViewId, url: String },
    PopupRefused { opener: ViewId, url: String },
    DownloadStarted { id: DownloadId, view: ViewId, url: String, plan: PlannedFile, file_name: String },
    DownloadRefused { view: ViewId, url: String, why: DownloadRefusal },
    DownloadEnded { part_path: Option<PathBuf>, url: String, success: bool },
    /// A page started a file the engine would hold back; the host asks for the same address itself.
    AskAgain { view: ViewId, url: String },
}

/// State the callbacks of all views share.
pub(crate) struct Shared {
    pub(crate) policy: NavigationPolicy,
    pub(crate) confirm_policy: ConfirmPolicy,
    /// Spare views, built ahead: a page that opens a window must get its view at once.
    pub(crate) spare: RefCell<Vec<(ViewId, WebView)>>,
    /// Spare views a page has just taken as its popup, with the opener; `pump` moves them to the session.
    pub(crate) adopted: RefCell<Vec<(ViewId, ViewId, WebView)>>,
    pub(crate) decider: RefCell<Option<DownloadDecider>>,
    pub(crate) next_download: Cell<u64>,
    /// Run in the main frame of every page once its document is parsed.
    pub(crate) ready_script: Option<String>,
    /// Page messages taken since the last `pump`: a page cannot make the host hoard more than a few.
    pub(crate) messages: Cell<u32>,
    /// Addresses of held-back files the host has asked for itself and whose offer is still to come.
    pub(crate) asked_again: RefCell<Vec<String>>,
}

/// Page messages taken between two `pump` calls, and the longest one taken (a dialog notice is far shorter).
const MAX_PAGE_MESSAGES: u32 = 200;
const MAX_PAGE_MESSAGE_BYTES: usize = 8192;

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared").field("spare", &self.spare.borrow().len()).finish_non_exhaustive()
    }
}

/// A hidden view has a desktop-sized page so that a site lays out its normal (not its phone) page; nothing
/// of it is ever drawn.
fn hidden_bounds() -> Rect {
    Rect { position: LogicalPosition::new(0.0, 0.0).into(), size: LogicalSize::new(1280.0, 900.0).into() }
}

/// Where the first view of a session gets its engine from, and where a later one does.
pub(crate) enum Engine<'a> {
    /// `arguments` are the engine arguments of a session that starts offline (see `policy`).
    Fresh { context: &'a mut WebContext, arguments: Option<String> },
    SameAs(&'a WebView),
}

pub(crate) fn build_hidden_view<W: HasWindowHandle>(
    parent: &W,
    id: ViewId,
    engine: Engine<'_>,
    shared: &Rc<Shared>,
    tx: &Sender<Signal>,
) -> Result<WebView, BrowserError> {
    let mut builder = match engine {
        Engine::Fresh { context, arguments } => {
            let builder = WebViewBuilder::new_with_web_context(context);
            match arguments {
                Some(arguments) => builder.with_additional_browser_args(arguments),
                None => builder,
            }
        }
        // A popup must use the engine and the private profile of its opener, or the page gets no window back.
        Engine::SameAs(view) => WebViewBuilder::new().with_environment(view.environment()),
    };
    let (policy_nav, policy_pop) = (shared.policy.clone(), shared.policy.clone());
    let (tx_nav, tx_load, tx_msg, tx_pop, tx_start, tx_end) = (tx.clone(), tx.clone(), tx.clone(), tx.clone(), tx.clone(), tx.clone());
    let (shared_pop, shared_dl, shared_msg) = (Rc::clone(shared), Rc::clone(shared), Rc::clone(shared));
    builder = builder
        .with_bounds(hidden_bounds())
        .with_visible(false)
        .with_focused(false)
        .with_incognito(true)
        .with_devtools(false)
        .with_hotkeys_zoom(false)
        .with_default_context_menus(false)
        .with_browser_accelerator_keys(false)
        .with_general_autofill_enabled(false)
        .with_initialization_script_for_main_only(quiet_dialogs_script(shared.confirm_policy), false)
        // The engine asks before a page may hand over a second file without a click, and a hidden view
        // cannot show that question: the transfer page of the NGII site sends the files of an order one
        // after another (KaTopographicBrowser.cpp:789-793). Each file still passes the host's decider.
        // Everything else a page may ask for (camera, microphone, position, notifications …) is refused.
        .with_permission_handler(|kind| match kind {
            PermissionKind::AutomaticDownloads => PermissionResponse::Allow,
            _ => PermissionResponse::Deny,
        })
        .with_navigation_handler(move |url| match policy_nav.judge(&url) {
            Navigation::Allow => true,
            why => {
                let _ = tx_nav.send(Signal::Refused { view: id, url, why });
                false
            }
        })
        .with_on_page_load_handler(move |event, url| {
            let _ = tx_load.send(match event {
                PageLoadEvent::Started => Signal::LoadStarted { view: id, url },
                PageLoadEvent::Finished => Signal::Loaded { view: id, url },
            });
        })
        .with_ipc_handler(move |request| {
            let taken = shared_msg.messages.get();
            if taken < MAX_PAGE_MESSAGES && request.body().len() <= MAX_PAGE_MESSAGE_BYTES {
                shared_msg.messages.set(taken + 1);
                let _ = tx_msg.send(Signal::PageMessage { view: id, body: request.body().clone() });
            }
        })
        .with_new_window_req_handler(move |url, _features| {
            let why = policy_pop.judge(&url);
            if why != Navigation::Allow {
                let _ = tx_pop.send(Signal::Refused { view: id, url, why });
                return NewWindowResponse::Deny;
            }
            let taken = shared_pop.spare.borrow_mut().pop();
            let Some((popup, view)) = taken else {
                let _ = tx_pop.send(Signal::PopupRefused { opener: id, url });
                return NewWindowResponse::Deny;
            };
            let engine_view = view.webview();
            shared_pop.adopted.borrow_mut().push((popup, id, view));
            let _ = tx_pop.send(Signal::PopupOpened { opener: id, view: popup, url });
            NewWindowResponse::Create { webview: engine_view }
        })
        .with_download_started_handler(move |url, path| {
            let offered = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let offer = DownloadOffer { view: id, url: url.clone(), suggested_name: safe_file_name(&offered) };
            // A file of a type the engine holds back is refused here and asked for again by the host
            // (`session::pump`); that second offer of the same address passes.
            if held_back_without_click(&offer.suggested_name) {
                let mut asked = shared_dl.asked_again.borrow_mut();
                match asked.iter().position(|a| *a == url) {
                    Some(at) => drop(asked.remove(at)),
                    None => {
                        let _ = tx_start.send(Signal::AskAgain { view: id, url });
                        return false;
                    }
                }
            }
            // A decider that is itself busy deciding (it let the window handle messages) refuses this one.
            let place = shared_dl.decider.try_borrow_mut().ok().and_then(|mut decider| decider.as_mut().and_then(|decide| decide(&offer)));
            let Some(place) = place else {
                let _ = tx_start.send(Signal::DownloadRefused { view: id, url, why: DownloadRefusal::Declined });
                return false;
            };
            let serial = shared_dl.next_download.get();
            shared_dl.next_download.set(serial + 1);
            match plan_download(&place.directory, &place.file_name, serial) {
                Ok(plan) => {
                    path.clone_from(&plan.part_path);
                    let _ = tx_start.send(Signal::DownloadStarted { id: DownloadId(serial), view: id, url, plan, file_name: place.file_name });
                    true
                }
                Err(why) => {
                    let _ = tx_start.send(Signal::DownloadRefused { view: id, url, why });
                    false
                }
            }
        })
        .with_download_completed_handler(move |url, path, success| {
            let _ = tx_end.send(Signal::DownloadEnded { part_path: path, url, success });
        });
    if let Some(script) = &shared.ready_script {
        // The C++ browser injects its page script at DocumentReady in the page's own world, in the main
        // frame only (KaTopographicBrowser.cpp:527-535): the document is parsed, the page's onload has not
        // run yet. The engine runs such a script in sub frames as well, hence the test; the line break keeps
        // a script that ends in a comment from swallowing the closing brace.
        let at_ready = format!(
            "if (window === window.top) {{ document.addEventListener('DOMContentLoaded', function () {{ try {{ {script}\n }} catch (e) {{}} }}); }}"
        );
        builder = builder.with_initialization_script_for_main_only(at_ready, true);
    }
    builder.build_as_child(parent).map_err(|e| BrowserError::Engine(e.to_string()))
}
