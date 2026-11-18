//! The field error window in a headless egui frame. AccessKit does not expose the dialog frame;
//! the test clicks the painted label, as the menu tests do.
//! `target/cpp-ref/tests/test_user_error.cpp:145-183`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, Event, Pos2, RawInput, Rect};

use super::super::user_error::FieldError;
use crate::app_state::StrataAppState;

fn frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    events: Vec<Event>,
) -> Vec<egui::epaint::ClippedShape> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 700.0))),
        predicted_dt: 1.0 / 60.0,
        events,
        ..Default::default()
    };
    ctx.run(raw, |ctx| app.render_field_error(ctx)).shapes
}

/// A new egui window has no body on the frame that opens it. The next frame paints the labels.
fn shown(app: &mut StrataAppState, ctx: &egui::Context) -> Vec<egui::epaint::ClippedShape> {
    let _ = frame(app, ctx, Vec::new());
    frame(app, ctx, Vec::new())
}

fn walk(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    f(shape);
    if let Shape::Vec(v) = shape {
        v.iter().for_each(|s| walk(s, f));
    }
}

fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    let mut out = Vec::new();
    for clipped in shapes {
        walk(&clipped.shape, &mut |s| {
            if let Shape::Text(t) = s {
                out.push(t.galley.text().to_owned());
            }
        });
    }
    out
}

fn center(shapes: &[egui::epaint::ClippedShape], needle: &str) -> Option<Pos2> {
    for clipped in shapes {
        let mut hit = None;
        walk(&clipped.shape, &mut |s| {
            if let Shape::Text(t) = s {
                if t.galley.text() == needle {
                    hit = Some(Rect::from_min_size(t.pos, t.galley.size()).center());
                }
            }
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

fn click(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    at: Pos2,
) -> Vec<egui::epaint::ClippedShape> {
    let _ = frame(app, ctx, vec![Event::PointerMoved(at)]);
    let _ = frame(
        app,
        ctx,
        vec![Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    frame(
        app,
        ctx,
        vec![Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    )
}

fn sample(app: &mut StrataAppState) {
    let mut spec = FieldError::warn(
        "지적도 받기",
        "지적도를 받지 못했습니다.",
        "서버가 <오류>를 돌려주었습니다.",
        "잠시 뒤 다시 받으세요.",
    );
    spec.details = "HTTP 503 Service Unavailable".to_owned();
    app.warn_field(spec);
}

#[test]
fn the_window_shows_the_three_headings_and_no_log_button_without_a_folder() {
    let mut app = StrataAppState::new_startup();
    app.log_folder.clear();
    sample(&mut app);
    let ctx = egui::Context::default();
    let shown = texts(&shown(&mut app, &ctx));
    assert!(shown.iter().any(|t| t == "무엇이 안 됐나"), "{shown:?}");
    assert!(shown.iter().any(|t| t == "왜 그런가"));
    assert!(shown.iter().any(|t| t == "어떻게 하나"));
    assert!(shown.iter().any(|t| t == "닫기"));
    assert!(shown.iter().any(|t| t == "자세히"));
    assert!(!shown.iter().any(|t| t == "로그 폴더 열기"));
}

#[test]
fn the_log_button_opens_the_folder_and_leaves_the_window_open() {
    let mut app = StrataAppState::new_startup();
    let dir = tempfile::tempdir().unwrap();
    app.log_folder = dir.path().to_string_lossy().into_owned();
    app.log_opener_records = Some(Vec::new());
    sample(&mut app);
    let ctx = egui::Context::default();
    let first = shown(&mut app, &ctx);
    let at = center(&first, "로그 폴더 열기").expect("painted 로그 폴더 열기");
    let still = click(&mut app, &ctx, at);
    let shown = texts(&still);
    assert!(
        shown.iter().any(|t| t == "무엇이 안 됐나"),
        "opening the folder must not close the window: {shown:?}"
    );
    assert_eq!(
        app.log_opener_records.clone().unwrap(),
        vec![dir.path().to_string_lossy().into_owned()]
    );
    let close_at = center(&still, "닫기").expect("닫기");
    let _ = click(&mut app, &ctx, close_at);
    assert!(app.field_error.is_none());
}

#[test]
fn escape_and_enter_dismiss_when_there_is_no_action() {
    let mut app = StrataAppState::new_startup();
    app.log_folder.clear();
    sample(&mut app);
    let ctx = egui::Context::default();
    let _ = frame(&mut app, &ctx, Vec::new());
    let _ = frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.field_error.is_none());
}

#[test]
fn an_action_button_is_what_enter_runs() {
    let mut app = StrataAppState::new_startup();
    app.log_folder.clear();
    let mut spec = FieldError::warn(
        "시험",
        "실패했습니다.",
        "시험용입니다.",
        "다시 입력 단추를 누르세요.",
    );
    spec.action_label = "다시 입력".to_owned();
    app.warn_field(spec);
    let ctx = egui::Context::default();
    let _ = frame(&mut app, &ctx, Vec::new());
    let _ = frame(
        &mut app,
        &ctx,
        vec![Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.field_action_chosen);
    assert!(app.field_error.is_none());
}
