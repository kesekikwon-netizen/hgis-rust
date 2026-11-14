//! F1d-1 — with no file chosen, accepting the survey-contour dialog only shows 「좌표 파일을 고르세요.」
//! in the preview text and creates nothing (target/cpp-ref/src/app/KaSurveyContourDialog.cpp:126-131).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

fn collect_texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.job.text.clone()),
        Shape::Vec(shapes) => {
            for s in shapes {
                collect_texts(s, out);
            }
        }
        _ => {}
    }
}

fn frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    events: Vec<Event>,
) -> (Vec<(String, Option<Rect>)>, Vec<String>) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        events,
        ..RawInput::default()
    };
    let out = ctx.run(raw, |ctx| {
        let mut open = app.modals.active == Some(ActiveModalDialog::SurveyContour);
        if open {
            super::render_survey_contour(app, ctx, &mut open);
            if !open {
                app.modals.active = None;
            }
        }
    });
    let mut texts = Vec::new();
    for clipped in &out.shapes {
        collect_texts(&clipped.shape, &mut texts);
    }
    let nodes = out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
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
    });
    (nodes, texts)
}

#[test]
fn empty_path_accept_shows_choose_file_prompt_and_creates_nothing() {
    let mut app = StrataAppState::new_startup();
    assert_eq!(app.modals.contour_file_path, "");
    app.modals.active = Some(ActiveModalDialog::SurveyContour);

    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let (mut nodes, mut before_texts) = (Vec::new(), Vec::new());
    for _ in 0..3 {
        (nodes, before_texts) = frame(&mut app, &ctx, vec![]);
    }
    assert!(before_texts.iter().any(|t| t == "DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다."));
    assert!(!before_texts.iter().any(|t| t.contains("점 248개")));
    assert!(!before_texts.iter().any(|t| t == "좌표 파일을 고르세요."));

    let at = nodes
        .iter()
        .find(|(l, _)| l == "등고선 만들기")
        .and_then(|(_, r)| *r)
        .unwrap_or_else(|| panic!("accept button missing: {nodes:?}"))
        .center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, vec![press(true)]);
    frame(&mut app, &ctx, vec![press(false)]);
    let (_, after_texts) = frame(&mut app, &ctx, vec![]);

    assert!(
        after_texts.iter().any(|t| t == "좌표 파일을 고르세요."),
        "preview must show choose-file prompt: {after_texts:?}"
    );
    assert!(!after_texts.iter().any(|t| t.contains("점 248개")));
    assert_eq!(app.modals.active, Some(ActiveModalDialog::SurveyContour));
    assert!(app.left_layers.entries.is_empty());
    assert_eq!(app.home_map.draw_guide_message, None);
    assert!(!app.is_dirty);
}
