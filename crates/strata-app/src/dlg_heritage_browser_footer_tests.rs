//! F1c-2 — the fetch click must not write an invented pledge receipt. cpp writes the receipt only after
//! the intranet answers "agreed" with the terms text (MainWindow.cpp:3227-3244, KaHeritageBrowser.cpp:988-993);
//! the Rust HTTP flow has no such step, so nothing is written. Headless egui frames, accesskit click.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::time::SystemTime;

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) -> Vec<(String, Option<Rect>)> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 400.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| super::render_heritage_footer(app, ui));
    });
    out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
        update
            .nodes
            .iter()
            .filter_map(|(_, n)| {
                let bounds = n.bounds().map(|b| {
                    Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
                });
                n.label().map(|l| (l.to_owned(), bounds))
            })
            .collect()
    })
}

fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

#[test]
fn fetch_click_writes_no_pledge_receipt() {
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"").expect("survey file");
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    // An account lets the click reach the old receipt write; an empty region stops the request in
    // heritage_preflight before any network thread starts.
    app.accounts.heritage_configured = true;
    app.modals.heritage_sido.clear();
    app.modals.heritage_city.clear();
    let temp_receipt = std::env::temp_dir().join("주변유적").join("receipts").join("pledge_receipt.txt");
    let before = modified(&temp_receipt);

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut nodes = Vec::new();
    for _ in 0..3 {
        nodes = frame(&mut app, &ctx, vec![]);
    }
    let at = nodes
        .iter()
        .find(|(l, _)| l == "받기")
        .and_then(|(_, r)| *r)
        .unwrap_or_else(|| panic!("fetch button missing: {nodes:?}"))
        .center();
    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, vec![press(true)]);
    frame(&mut app, &ctx, vec![press(false)]);

    assert!(!crate::dialogs_download_print::heritage_fetch_in_progress(), "no request thread may start");
    assert_eq!(modified(&temp_receipt), before, "no receipt in %TEMP%");
    let listing: Vec<_> = std::fs::read_dir(dir.path()).expect("list").map(|e| e.expect("entry").file_name()).collect();
    assert_eq!(listing, vec![std::ffi::OsString::from("현장.gpkg")], "survey folder unchanged");
}
