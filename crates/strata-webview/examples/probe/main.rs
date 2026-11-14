//! Probe of the hidden browser against a site on this PC. It opens a small window (the parent a hidden view
//! needs), runs the steps in `steps.rs` with the network switch OFF, prints one line per check and ends with
//! exit code 0 only if every check held. It contacts nothing outside 127.0.0.1.
//!
//! run: cargo run -p strata-webview --example probe
#![allow(clippy::unwrap_used, clippy::expect_used, missing_debug_implementations)]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
mod site;
#[cfg(windows)]
mod steps;

#[cfg(windows)]
fn main() {
    use eframe::egui;
    let site = site::Site::start().expect("local site");
    let work = std::env::temp_dir().join(format!("strata-webview-probe-{}", std::process::id()));
    std::fs::create_dir_all(work.join("files")).expect("work folder");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 260.0]).with_title("strata-webview probe"),
        ..Default::default()
    };
    let probe = steps::Probe::new(site, work.clone());
    let outcome = std::rc::Rc::clone(&probe.outcome);
    let _ = eframe::run_native("strata-webview probe", options, Box::new(|_cc| Ok(Box::new(probe))));
    let (passed, failed) = *outcome.borrow();
    let _ = std::fs::remove_dir_all(&work);
    println!("PROBE {} passed, {} failed; work folder left: {}", passed, failed, work.exists());
    std::process::exit(i32::from(failed > 0 || passed == 0));
}
