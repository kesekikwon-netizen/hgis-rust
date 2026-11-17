//! The heritage run on the real browser engine against the fixture: the ported scripts, the
//! stage machine and the hidden session together. The native parent is invisible from creation.
//! Exit code 0 requires every selected case to finish and every visibility check to hold.
//!
//! It contacts nothing outside 127.0.0.1 and uses no account but the fixture's. The shape is
//! `strata-ngii-run/examples/portal_run`.
//!
//! run: cargo run -p strata-heritage-run --example heritage_run
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_debug_implementations
)]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
mod assertions;
#[cfg(windows)]
mod case;
#[cfg(windows)]
mod hidden_host;
#[cfg(windows)]
mod runner;
#[cfg(windows)]
mod sample_zip;

#[cfg(windows)]
fn main() {
    let work = std::env::temp_dir().join(format!("strata-heritage-run-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("work folder");
    let runner = runner::Runner::new(work.clone());
    let planned = runner.planned();
    let outcome = std::rc::Rc::clone(&runner.outcome);
    let hosting = hidden_host::run(runner);
    if let Err(why) = &hosting {
        eprintln!("HIDDEN HOST FAILED: {why}");
    }
    let (passed, failed) = *outcome.borrow();
    let _ = std::fs::remove_dir_all(&work);
    println!("HERITAGE {passed} passed, {failed} failed, {planned} planned");
    std::process::exit(i32::from(
        hosting.is_err() || !hidden_host::matrix_succeeded(passed, failed, planned),
    ));
}
