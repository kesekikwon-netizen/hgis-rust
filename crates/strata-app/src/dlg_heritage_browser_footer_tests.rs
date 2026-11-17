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

    assert_eq!(modified(&temp_receipt), before, "no receipt in %TEMP%");
    let listing: Vec<_> = std::fs::read_dir(dir.path()).expect("list").map(|e| e.expect("entry").file_name()).collect();
    assert_eq!(listing, vec![std::ffi::OsString::from("현장.gpkg")], "survey folder unchanged");
}

fn collect_texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push(t.galley.text().to_owned()),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| collect_texts(s, out)),
        _ => {}
    }
}

fn window_frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
    events: Vec<Event>,
) -> (Vec<(String, bool, Option<Rect>)>, String) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        crate::dlg_heritage_browser::render_heritage_browser(app, ctx, open);
    });
    let mut texts = Vec::new();
    out.shapes.iter().for_each(|c| collect_texts(&c.shape, &mut texts));
    let nodes = out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
        update
            .nodes
            .iter()
            .filter_map(|(_, n)| {
                let bounds = n.bounds().map(|b| {
                    Rect::from_min_max(Pos2::new(b.x0 as f32, b.y0 as f32), Pos2::new(b.x1 as f32, b.y1 as f32))
                });
                n.label().map(|l| (l.to_owned(), n.is_disabled(), bounds))
            })
            .collect()
    });
    (nodes, texts.join("\n"))
}

/// NET0b, replaced by HER7: 「받기」 used to be disabled with the WB1 tooltip. The run exists now
/// (`strata-heritage-run`), so the button is live whenever the account, the survey and the city
/// are in place, and a press asks for a run.
#[test]
fn heritage_start_button_is_live_and_asks_for_a_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"").expect("survey file");
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    app.accounts.heritage_configured = true;
    app.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::HeritageBrowser);
    app.modals.heritage_sido = "경상북도".to_owned();
    app.modals.heritage_city = "경주시".to_owned();
    app.modals.heritage_dataset_checks = [true, false, true, true, true, true, true];

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.show_tooltips_only_when_still = false;
    });
    let mut open = true;
    let mut nodes = Vec::new();
    for _ in 0..3 {
        (nodes, _) = window_frame(&mut app, &ctx, &mut open, vec![]);
    }
    let (_, disabled, bounds) = nodes
        .iter()
        .find(|(l, _, _)| l == "받기")
        .cloned()
        .unwrap_or_else(|| panic!("fetch button missing: {nodes:?}"));
    assert!(!disabled, "받기 must be live when the account, survey and city are set: {nodes:?}");
    for enabled_btn in ["상세 보기", "설정", "취소"] {
        assert_eq!(
            nodes.iter().find(|(l, _, _)| l == enabled_btn).map(|(_, d, _)| *d),
            Some(false),
            "{enabled_btn} must stay enabled"
        );
    }

    assert!(!app.heritage_run_requested, "a run started before the press");
    let at = bounds.expect("bounds").center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    window_frame(&mut app, &ctx, &mut open, vec![press(true)]);
    window_frame(&mut app, &ctx, &mut open, vec![press(false)]);

    assert!(open, "the modal closed on a press");
    assert!(
        app.heritage_run_requested,
        "the press did not ask for a run: {}",
        app.status_bar_hint
    );
    assert!(app.status_bar_hint.contains("경주시"), "{}", app.status_bar_hint);
}

/// With no account the button says which thing is missing instead of the old WB1 tooltip.
#[test]
fn without_an_account_the_button_names_what_is_missing() {
    let mut app = StrataAppState::new_startup();
    app.accounts.heritage_configured = false;
    app.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::HeritageBrowser);
    app.modals.heritage_sido = "경상북도".to_owned();
    app.modals.heritage_city = "경주시".to_owned();

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        s.interaction.show_tooltips_only_when_still = false;
    });
    let mut open = true;
    let mut nodes = Vec::new();
    for _ in 0..3 {
        (nodes, _) = window_frame(&mut app, &ctx, &mut open, vec![]);
    }
    let (_, disabled, bounds) = nodes
        .iter()
        .find(|(l, _, _)| l == "받기")
        .cloned()
        .unwrap_or_else(|| panic!("fetch button missing: {nodes:?}"));
    assert!(disabled, "받기 must be disabled without an account");

    let at = bounds.expect("bounds").center();
    let (_, hover1) = window_frame(&mut app, &ctx, &mut open, vec![Event::PointerMoved(at)]);
    let (_, hover2) = window_frame(&mut app, &ctx, &mut open, vec![Event::PointerMoved(at)]);
    let combined = format!("{hover1}\n{hover2}");
    assert!(combined.contains("아이디"), "the tooltip does not say what is missing:\n{combined}");
}
