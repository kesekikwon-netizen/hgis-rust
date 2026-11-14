//! The whole NGII sheet order on the real browser engine against the fake portal of the C++ tests
//! (`strata-ngii-fixture`): the C++ page script, the stage logic and the hidden session together. Opens a
//! small window (the parent a hidden view needs), runs the cases of `cases.rs` one after another with the
//! network switch OFF, prints one line per case and ends with exit code 0 only if every case held. It
//! contacts nothing outside 127.0.0.1 and uses no account but the fake one of the C++ tests.
//!
//! run: cargo run -p strata-ngii-run --example portal_run            (all cases)
//!      cargo run -p strata-ngii-run --example portal_run -- 4       (one case, counted from 1)
#![allow(clippy::unwrap_used, clippy::expect_used, missing_debug_implementations)]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
mod cases;
#[cfg(windows)]
mod runner;

#[cfg(windows)]
fn main() {
    use eframe::egui;
    let only: Option<usize> = std::env::args().nth(1).and_then(|a| a.parse().ok());
    let work = std::env::temp_dir().join(format!("strata-ngii-run-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("work folder");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([460.0, 260.0]).with_title("strata-ngii-run portal"),
        ..Default::default()
    };
    let runner = runner::Runner::new(work.clone(), only);
    let outcome = std::rc::Rc::clone(&runner.outcome);
    let _ = eframe::run_native("strata-ngii-run portal", options, Box::new(|_cc| Ok(Box::new(runner))));
    let (passed, failed) = *outcome.borrow();
    let _ = std::fs::remove_dir_all(&work);
    println!("PORTAL {passed} passed, {failed} failed; work folder left: {}", work.exists());
    std::process::exit(i32::from(failed > 0 || passed == 0));
}
