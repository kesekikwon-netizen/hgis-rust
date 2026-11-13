//! F1c-3 — 「PDF 내보내기」 (R3) and 「나눈 장을 PDF로 저장…」 (W-50) stay disabled with the deferred tooltip
//! and write nothing. cpp writes real PDFs (KaDrawingStudio.cpp:3819-3851, KaPrintDialog.cpp:592-620); the
//! Rust writer drew empty rectangles into %TEMP%\조사도면.pdf. Headless egui frames, accesskit tree + click.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::time::SystemTime;

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;

type Paint = fn(&mut StrataAppState, &mut egui::Ui);

fn frame(app: &mut StrataAppState, ctx: &egui::Context, paint: Paint, events: Vec<Event>) -> Vec<(String, bool, Option<Rect>)> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 1400.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| paint(app, ui));
    });
    out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
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
    })
}

fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Renders `paint`, checks `label` is disabled, clicks it, and checks nothing was written or said.
fn assert_deferred(paint: Paint, label: &str) {
    let mut app = StrataAppState::new_startup();
    let temp_pdf = std::env::temp_dir().join("조사도면.pdf");
    let before = modified(&temp_pdf);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut nodes = Vec::new();
    for _ in 0..3 {
        nodes = frame(&mut app, &ctx, paint, vec![]);
    }
    let (_, disabled, bounds) = nodes
        .iter()
        .find(|(l, _, _)| l == label)
        .cloned()
        .unwrap_or_else(|| panic!("{label} missing: {nodes:?}"));
    assert!(disabled, "{label} must be disabled until its order lands");
    let at = bounds.expect("bounds").center();
    let press = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() };
    frame(&mut app, &ctx, paint, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, paint, vec![press(true)]);
    frame(&mut app, &ctx, paint, vec![press(false)]);
    assert_eq!(modified(&temp_pdf), before, "no PDF in %TEMP%");
    assert_eq!(app.home_map.draw_guide_message, None, "no saved message");
}

#[test]
fn drawing_pdf_export_is_deferred_r3() {
    assert_deferred(|app, ui| app.paint_drawing_right_items_b(ui), "PDF 내보내기");
}

#[test]
fn print_save_tiles_pdf_is_deferred_w50() {
    assert_deferred(super::render_print_actions, "나눈 장을 PDF로 저장…");
}
