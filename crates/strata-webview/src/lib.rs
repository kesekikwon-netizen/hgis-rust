//! Hidden browser session for sites that need a logged-in browser (`strata-webview`).
//!
//! The C++ app drives such sites with a hidden QtWebEngine view (`target/cpp-ref/src/app/KaTopographicBrowser.cpp`).
//! Here the same is done with the WebView2 engine that is part of Windows, through `wry`: views that are never
//! shown, a private profile, scripts run in a page's main frame, popups kept as further hidden views,
//! downloads written to a part file first. Windows only; on other systems the crate is empty.

#![forbid(unsafe_code)]

#[cfg(windows)]
pub mod dialogs;
#[cfg(windows)]
pub mod downloads;
#[cfg(windows)]
pub mod names;
#[cfg(windows)]
pub mod policy;
#[cfg(windows)]
pub mod profile;
#[cfg(windows)]
mod events;
#[cfg(windows)]
mod session;
#[cfg(windows)]
mod view;

#[cfg(windows)]
pub use events::{BrowserError, BrowserEvent, DownloadDecider, DownloadId, DownloadOffer, DownloadPlace, ScriptCall, ViewId, ViewInfo};
#[cfg(windows)]
pub use session::{BrowserOptions, HiddenBrowser};
