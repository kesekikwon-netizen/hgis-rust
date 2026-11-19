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

type AccessNode = (String, bool, Option<Rect>);

fn frame(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    events: Vec<Event>,
) -> (Vec<AccessNode>, Vec<String>) {
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
    let nodes = out
        .platform_output
        .accesskit_update
        .map_or_else(Vec::new, |update| {
            update
                .nodes
                .iter()
                .filter_map(|(_, n)| {
                    let bounds = n.bounds().map(|b| {
                        Rect::from_min_max(
                            Pos2::new(b.x0 as f32, b.y0 as f32),
                            Pos2::new(b.x1 as f32, b.y1 as f32),
                        )
                    });
                    n.label().map(|l| (l.to_owned(), n.is_disabled(), bounds))
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
    assert!(before_texts
        .iter()
        .any(|t| t == "DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다."));
    assert!(!before_texts.iter().any(|t| t.contains("점 248개")));
    assert!(!before_texts.iter().any(|t| t == "좌표 파일을 고르세요."));

    let (disabled, at) = nodes
        .iter()
        .find(|(l, _, _)| l == "등고선 만들기")
        .map(|(_, disabled, r)| (*disabled, r.expect("accept button bounds").center()))
        .unwrap_or_else(|| panic!("accept button missing: {nodes:?}"));
    assert!(!disabled, "an empty path can still be refused");
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

fn fixed_count_phrases(texts: &[String]) -> Vec<String> {
    const NEEDLES: [&str; 4] = ["점 248개", "24.5", "48.2", "표고점 248개"];
    texts
        .iter()
        .filter(|t| NEEDLES.iter().any(|n| t.contains(n)))
        .cloned()
        .collect()
}

fn open_contour(path: &str) -> (StrataAppState, egui::Context, Vec<String>) {
    let mut app = StrataAppState::new_startup();
    app.modals.contour_file_path = path.to_owned();
    app.modals.active = Some(ActiveModalDialog::SurveyContour);
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut texts = Vec::new();
    for _ in 0..3 {
        (_, texts) = frame(&mut app, &ctx, vec![]);
    }
    (app, ctx, texts)
}

/// Any typed path must not paint the invented preview, and accepting it must not
/// add an empty section line (`dlg_survey_contour.rs` used to do both).
#[test]
fn arbitrary_path_never_shows_fixed_counts_or_writes_section_line() {
    let (mut app, ctx, texts) = open_contour("not-a-survey-248.csv");
    assert!(
        fixed_count_phrases(&texts).is_empty(),
        "invented counts in {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "파일을 열지 못했습니다. 다른 프로그램이 파일을 열고 있는지 확인하세요."),
        "missing file must say so: {texts:?}"
    );

    let (nodes, _) = frame(&mut app, &ctx, vec![]);
    let (disabled, at) = nodes
        .iter()
        .find(|(l, _, _)| l == "등고선 만들기")
        .map(|(_, disabled, r)| (*disabled, r.expect("accept button bounds").center()))
        .unwrap_or_else(|| panic!("accept button missing: {nodes:?}"));
    assert!(disabled, "등고선 만들기 stays off until W-29");
    ctx.style_mut(|style| style.interaction.tooltip_delay = 0.0);
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    let (_, hovered) = frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    assert!(
        hovered
            .iter()
            .any(|t| t == "아직 Rust 판에서 연결하지 않았습니다 (지시 W-29)"),
        "disabled button must say generation is not connected: {hovered:?}"
    );
    frame(&mut app, &ctx, vec![press(true)]);
    frame(&mut app, &ctx, vec![press(false)]);
    let (_, after) = frame(&mut app, &ctx, vec![]);

    assert!(
        fixed_count_phrases(&after).is_empty(),
        "invented counts after click: {after:?}"
    );
    assert!(
        app.left_layers.entries.is_empty(),
        "no domain layer: {:?}",
        app.left_layers.entries
    );
    assert_eq!(app.home_map.draw_guide_message, None);
    assert!(!app.is_dirty);
}

/// Preview counts come from the file, in the C++ sentence
/// (`KaSurveyContourDialog.cpp` `showPreview`: 점 N개, 표고 min–max m, 등고선 약 L줄).
#[test]
fn sample_csv_preview_matches_point_count_and_elevation_range() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("points.csv");
    std::fs::write(
        &path,
        "점번호,X,Y,표고\n1,200000,500000,10\n2,200010,500000,12.5\n3,200000,500010,11\n4,200010,500010,15.25\n",
    )
    .unwrap();
    let (_, _, texts) = open_contour(&path.display().to_string());
    assert!(
        fixed_count_phrases(&texts).is_empty(),
        "invented counts for a real file: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t == "점 4개, 표고 10.0–15.3 m. 등고선 약 10줄."),
        "preview must match the four rows: {texts:?}"
    );
}
