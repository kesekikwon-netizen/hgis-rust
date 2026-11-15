//! The close prompt (`KaClosePrompt.cpp`): what its buttons do to the closing of the window.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui;

use crate::app_state::StrataAppState;

/// A frame in which the window is asked to close (its X, or the app's own close).
fn closing() -> egui::RawInput {
    let mut raw = egui::RawInput::default();
    raw.viewports.entry(egui::ViewportId::ROOT).or_default().events.push(egui::ViewportEvent::Close);
    raw
}

fn held_back(out: &egui::FullOutput) -> bool {
    out.viewport_output.values().any(|viewport| viewport.commands.contains(&egui::ViewportCommand::CancelClose))
}

/// 「저장하지 않고 끝내기」 ends the app. The close it asks for must not be answered by the prompt again
/// (owner 2026-11-15: 「x를누르고 창을 없애려할때 저장하지않고끄기가 안된다」: the prompt came back and the
/// window stayed open for ever, because the edits are still unsaved when the close arrives).
#[test]
fn closing_without_saving_lets_the_window_close() {
    let mut app = StrataAppState::new_startup();
    app.is_dirty = true;
    let ctx = egui::Context::default();

    let asked = ctx.run(closing(), |ctx| app.render_close_prompt(ctx));
    assert!(app.close_prompt_open && held_back(&asked), "the X of the window opens the prompt and the close waits");

    let chosen = ctx.run(egui::RawInput::default(), |ctx| app.close_without_saving(ctx));
    assert!(chosen.viewport_output.values().any(|viewport| viewport.commands.contains(&egui::ViewportCommand::Close)));

    let closed = ctx.run(closing(), |ctx| app.render_close_prompt(ctx));
    assert!(!app.close_prompt_open, "the prompt does not come back");
    assert!(!held_back(&closed), "the close is let through");
    assert!(app.is_dirty, "nothing was saved");
}

/// 취소 keeps the app open, and the next X asks again.
#[test]
fn a_cancelled_close_asks_again_the_next_time() {
    let mut app = StrataAppState::new_startup();
    app.is_dirty = true;
    let ctx = egui::Context::default();
    let _ = ctx.run(closing(), |ctx| app.render_close_prompt(ctx));
    app.close_prompt_open = false;
    let again = ctx.run(closing(), |ctx| app.render_close_prompt(ctx));
    assert!(app.close_prompt_open && held_back(&again));
}
