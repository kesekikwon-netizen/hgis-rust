//! The heritage run on the real browser engine against the fixture: the ported scripts, the
//! stage machine and the hidden session together. Opens a small window (the parent a hidden
//! view needs), drives one case and ends with exit code 0 only if it held.
//!
//! It contacts nothing outside 127.0.0.1 and uses no account but the fixture's. The shape is
//! `strata-ngii-run/examples/portal_run`.
//!
//! run: cargo run -p strata-heritage-run --example heritage_run
#![allow(clippy::unwrap_used, clippy::expect_used, missing_debug_implementations)]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
mod case;
#[cfg(windows)]
mod runner;

#[cfg(windows)]
fn main() {
    use eframe::egui;
    let work = std::env::temp_dir().join(format!("strata-heritage-run-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("work folder");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 260.0])
            .with_title("strata-heritage-run"),
        ..Default::default()
    };
    let runner = runner::Runner::new(work.clone());
    let outcome = std::rc::Rc::clone(&runner.outcome);
    let _ = eframe::run_native(
        "strata-heritage-run",
        options,
        Box::new(|_cc| Ok(Box::new(runner))),
    );
    let (passed, failed) = *outcome.borrow();
    let _ = std::fs::remove_dir_all(&work);
    println!("HERITAGE {passed} passed, {failed} failed");
    std::process::exit(i32::from(failed > 0 || passed == 0));
}
