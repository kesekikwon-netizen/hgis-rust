//! The order of topographic sheets on the NGII site (국토정보플랫폼), driven without a window.
//!
//! The C++ app does this with a hidden browser and one page script, `data/providers/ngii-topographic.js`: the
//! script is called every 400 ms with a command and answers with a state, and `KaTopographicBrowser`
//! (`target/cpp-ref/src/app/KaTopographicBrowser.cpp`) moves from stage to stage by those answers. This module
//! is that stage logic, with the same script, the same order of steps and the same words. It knows no browser:
//! the pages are behind [`PageHost`], time is a number of milliseconds the caller passes in, and what
//! happened comes out as [`AutoEvent`]s.

use std::path::PathBuf;

use serde_json::{Map, Value};

mod answer;
mod files;
mod machine;
mod pages;
mod poll;
mod run;
mod url;

#[cfg(test)]
mod tests_answer;
#[cfg(test)]
mod tests_files;
#[cfg(test)]
mod tests_poll;
#[cfg(test)]
mod tests_support;

pub use machine::NgiiAutomation;
pub use url::UrlParts;

/// The portal's map page (`KaTopographicBrowser.cpp:49`).
pub const PORTAL_HOME: &str = "https://map.ngii.go.kr/ms/map/NlipMap.do?tabGb=total";
/// The purpose written into the application form when the user gave none (`:168`).
pub const DEFAULT_PURPOSE: &str = "고고학 발굴조사 현장 도면 작성 및 지형 확인";

/// The page script of the C++ app, unchanged: `data/providers/ngii-topographic.js` holds the same bytes as
/// `target/cpp-ref/data/providers/ngii-topographic.js`. It is a function expression; the poll calls it with
/// its input (`KaTopographicBrowser.cpp:798`).
pub const PAGE_SCRIPT: &str = include_str!("../../../../data/providers/ngii-topographic.js");

/// One page of the hidden browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u32);

/// A page as the browser reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRef {
    pub id: PageId,
    pub url: String,
    /// The page whose script opened this one.
    pub opener: Option<PageId>,
}

/// What the stage logic needs from the browser.
pub trait PageHost {
    /// The open pages, oldest first.
    fn pages(&self) -> Vec<PageRef>;
    /// Runs a script in a page's main frame. Its value comes back through
    /// [`NgiiAutomation::script_answered`] with the token returned here; `None` when the page is gone.
    fn run_script(&mut self, page: PageId, script: &str) -> Option<u64>;
    /// Sends the first page to an address.
    fn navigate(&mut self, url: &str);
}

/// Where the automation stands (`m_automationStage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Select,
    OpenForm,
    Form,
    Submit,
    Order,
    Files,
    Download,
    Transfer,
    Complete,
    Stopped,
}

impl Stage {
    /// The command the page script gets in this stage.
    #[must_use]
    pub fn command(self) -> &'static str {
        match self {
            Self::Idle => "",
            Self::Select => "select",
            Self::OpenForm => "openForm",
            Self::Form => "form",
            Self::Submit => "submit",
            Self::Order => "order",
            Self::Files => "files",
            Self::Download => "download",
            Self::Transfer => "transfer",
            Self::Complete => "complete",
            Self::Stopped => "stopped",
        }
    }
}

/// The circle to get sheets for, in EPSG:5179 metres, with the sheet numbers the app worked out and the ones
/// it already keeps (`prepareSheets`, `:660`).
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub numbers: Vec<String>,
    pub cached_numbers: Vec<String>,
}

/// The account and the two fields of the application form. Never printed.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Account {
    pub login_id: String,
    pub login_password: String,
    pub birth_date: String,
    pub purpose: String,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account").field("login_id", &!self.login_id.is_empty()).finish_non_exhaustive()
    }
}

/// How a run is set up.
#[derive(Debug, Clone)]
pub struct AutoConfig {
    /// The portal's map page; [`PORTAL_HOME`] outside tests.
    pub portal_url: String,
    /// The text of `ngii-topographic.js`.
    pub script: String,
    /// Folder under which every sheet gets a folder of its own (`m_downloadDirectory`).
    pub download_directory: PathBuf,
    pub account: Account,
    /// Lets a portal on 127.0.0.1 stand in for the real one (`QStandardPaths::isTestModeEnabled()`).
    pub test_mode: bool,
    /// Asks the page script to say at which of its checks it stopped (`KA_HGIS_NGII_DIAGNOSTICS=1` in the
    /// C++, `:780`). The note names a stage and a step, never page content.
    pub diagnostics: bool,
}

/// What the stage logic tells its caller.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoEvent {
    /// The run needs attention or stopped; the words to show (`automationNeedsInput`).
    NeedsInput(String),
    /// The portal answered which sheets cover the circle (`selectionUpdated`). The map CRS is EPSG:5179.
    SelectionUpdated { items: Value, cached: Value, unavailable: Value },
    /// A sheet's data file is complete on disk (`sheetDownloaded`).
    SheetDownloaded { path: PathBuf, record: Map<String, Value> },
}

/// Where the browser must put an offered file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadTarget {
    pub directory: PathBuf,
    pub file_name: String,
}
