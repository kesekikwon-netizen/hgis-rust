//! One run of the heritage intranet download in a hidden browser session: the stage logic of
//! `strata_net::heritage_auto` fed with what `strata_webview` reports, frame by frame, on the
//! UI thread.
//!
//! The shape is `strata-ngii-run`'s: a `Core` holding the machine and a queue of what it wants
//! done, and a `frame` that drains the browser's events, lets the machine decide, and hands the
//! machine's wishes back to the browser. The browser itself cannot be used inside its own
//! callbacks, so everything goes through the queue.
//!
//! Windows only: the session is WebView2. The run against the fixture is
//! `examples/heritage_run`, the shape of `strata-ngii-run/examples/portal_run`.

#[cfg(windows)]
pub mod finish;
#[cfg(windows)]
mod host;
#[cfg(windows)]
mod run;

#[cfg(windows)]
pub use run::{start_run, HeritageRunHandle, RunOptions};
#[cfg(windows)]
pub use strata_net::heritage_auto::{DatasetOutcome, HeritageConfig, HeritageEvent, HeritageStage};
