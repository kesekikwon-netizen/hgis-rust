//! The heritage intranet run (`target/cpp-ref/src/app/KaHeritageBrowser.cpp`), as a stage
//! machine that knows no browser.
//!
//! The C++ app drives a hidden `QWebEngineView` with one page script per step and moves
//! from stage to stage by the script's answer (`KaHeritageBrowser.cpp:596-940` for login,
//! `:1030-1524` for the download). This module is that stage logic: the same scripts
//! ([`scripts`]), the same order, the same words. The pages are behind [`PageHost`], and what
//! happened comes out as [`HeritageEvent`]s.

pub mod scripts;

#[path = "machine.rs"]
mod machine;
#[path = "machine_login.rs"]
mod machine_login;
#[path = "machine_kinds.rs"]
mod machine_kinds;
#[path = "machine_pages.rs"]
mod machine_pages;
#[path = "login_answers.rs"]
mod login_answers;

#[cfg(test)]
#[path = "tests_login.rs"]
mod tests_login;
#[cfg(test)]
#[path = "tests_kinds.rs"]
mod tests_kinds;
#[cfg(test)]
#[path = "tests_pages.rs"]
mod tests_pages;

pub use machine::{HeritageAutomation, HeritageConfig, HeritageEvent, HeritageStage};
pub use machine_kinds::MAX_ATTEMPTS;
pub use machine_login::NO_ACCOUNT;

/// A page of the hidden browser, as the stage logic sees it.
///
/// The heritage run and the topographic run watch the same browser the same way, so this is
/// the NGII pair rather than a second, identical one (`ngii_auto/mod.rs:117-140`).
pub use crate::ngii_auto::{PageHost, PageId, PageRef};

/// What the machine wants done to a page: run `script`, and the answer names `token`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCall {
    /// The page to run in.
    pub page: PageId,
    /// The script text; always a `wrap()`ed function expression for this run.
    pub script: String,
    /// The answer comes back through
    /// [`HeritageAutomation::script_answered`] with this number.
    pub token: u64,
}

/// How often the C++ probes the login page (`KaHeritageBrowser.cpp:103`).
pub const LOGIN_POLL_MS: u64 = 700;
/// How many quiet ticks the C++ lets a page settle before it acts (`:395`).
pub const SETTLE_TICKS: u8 = 2;
