//! One run of the NGII topographic sheet order: the stage logic of `strata_net::ngii_auto` driven in a
//! hidden browser session of `strata_webview`. Windows only; on other systems the crate is empty.

#![forbid(unsafe_code)]

#[cfg(windows)]
mod host;
#[cfg(windows)]
mod run;

#[cfg(windows)]
pub use run::{NgiiRun, RunOptions};
