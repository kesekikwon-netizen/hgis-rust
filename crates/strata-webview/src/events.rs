//! What a hidden session tells its host, and the small types both sides share.

use std::path::PathBuf;

use crate::dialogs::DialogKind;
use crate::downloads::DownloadRefusal;
use crate::policy::Navigation;

/// One hidden view: the first page of the session or a popup a page opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub u32);

/// One script run asked for with `HiddenBrowser::run_script`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScriptCall(pub u64);

/// One file transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DownloadId(pub u64);

/// A view as the host sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewInfo {
    pub id: ViewId,
    /// The address the view shows now (empty before its first page).
    pub url: String,
    /// The view whose page opened this one.
    pub opener: Option<ViewId>,
}

/// A file a page wants to hand over. The name is already made safe (`names::safe_file_name`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOffer {
    pub view: ViewId,
    pub url: String,
    pub suggested_name: String,
}

/// Where the host wants an offered file. The folder must exist; the name must be a safe name that is free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlace {
    pub directory: PathBuf,
    pub file_name: String,
}

/// Asked once for every offered file, on the UI thread, while the page waits. `None` refuses the file.
pub type DownloadDecider = Box<dyn FnMut(&DownloadOffer) -> Option<DownloadPlace>>;

/// Something that happened in the session since the last `pump`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserEvent {
    /// A page finished loading in a view.
    PageLoaded { view: ViewId, url: String },
    /// A page opened a window; it lives on as a hidden view.
    PopupOpened { opener: ViewId, view: ViewId, url: String },
    /// A page tried to open a window and no spare view was ready: the window was not opened.
    PopupRefused { opener: ViewId, url: String },
    /// A view was kept from going to an address.
    NavigationRefused { view: ViewId, url: String, why: Navigation },
    /// The answer of a script run: the JSON text of its value.
    Script { call: ScriptCall, view: ViewId, json: String },
    /// A page asked for a dialog; it was answered without showing anything.
    Dialog { view: ViewId, kind: DialogKind, text: String },
    DownloadStarted { id: DownloadId, view: ViewId, url: String, file_name: String },
    DownloadRefused { view: ViewId, url: String, why: DownloadRefusal },
    DownloadProgress { id: DownloadId, bytes: u64 },
    /// The file is complete under its real name.
    DownloadFinished { id: DownloadId, path: PathBuf, bytes: u64 },
    /// The transfer broke off or the file could not be put in place; nothing of it is left on disk.
    DownloadFailed { id: DownloadId, url: String },
}

/// Why the session could not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserError {
    /// The engine (WebView2 through wry) answered with an error; its text.
    Engine(String),
    /// The session's data folder could not be made.
    DataFolder(String),
    NoSuchView(ViewId),
    Refused(Navigation),
}

impl std::fmt::Display for BrowserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(text) => write!(f, "browser engine: {text}"),
            Self::DataFolder(text) => write!(f, "session folder: {text}"),
            Self::NoSuchView(view) => write!(f, "no view {}", view.0),
            Self::Refused(why) => write!(f, "address refused: {why:?}"),
        }
    }
}

impl std::error::Error for BrowserError {}
