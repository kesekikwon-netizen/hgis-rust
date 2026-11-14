//! 오른쪽 카드의 실제 입력 경로(화면 없는 egui) — 메모는 칸을 떠날 때만 쓴다, 도구가 바뀌면 고른 것을 다시 읽는다. demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, Event, Pos2, RawInput, Rect};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::checklist_state::attr;

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app
}

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) {
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))), events, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
}

fn note_of(app: &StrataAppState) -> Option<String> {
    let t = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    attr(t.layer.features.iter().find(|f| f.fid == 1).unwrap(), "note")
}

/// card_noteIsTwoLineAndCommitsOnFocusOut — 적기만 하면 아무것도 안 쓰고, 칸을 떠나면 쓴다.
#[test]
fn memo_commits_when_the_box_loses_focus() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let ctx = egui::Context::default();
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), 1));
    app.sync_card_with_selection();
    frame(&mut app, &ctx, vec![]);
    let id = egui::Id::new(("ka_card_box", "note"));
    ctx.memory_mut(|m| m.request_focus(id));
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![Event::Text("북서쪽 모서리".to_owned())]);
    assert_ne!(note_of(&app).as_deref(), Some("북서쪽 모서리"), "typing alone writes nothing");
    // 지도를 누르면 칸을 떠난다(FocusOut).
    let at = Pos2::new(600.0, 500.0);
    let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::default() };
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    frame(&mut app, &ctx, vec![button(true)]);
    frame(&mut app, &ctx, vec![button(false)]);
    frame(&mut app, &ctx, vec![]);
    assert!(note_of(&app).is_some_and(|n| n.contains("북서쪽 모서리")), "{:?}", note_of(&app));
}

/// mapToolSet — 그리기 도구를 내려놓으면 고른 것을 다시 읽는다.
#[test]
fn leaving_the_draw_tool_reads_the_selection_again() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.active_tool = MapToolKind::DrawPolygon;
    app.resync_card_on_tool_change();
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), 2));
    app.sync_card_with_selection();
    assert!(app.home_map.card.is_none(), "no change while drawing");
    app.active_tool = MapToolKind::Pan;
    app.resync_card_on_tool_change();
    assert_eq!(app.home_map.card.as_ref().map(|c| c.fid), Some(2));
}

// While DrawLine is active, the card does not follow the selection and shows the drawing sentence
// (target/cpp-ref/src/app/MainWindowFeatureCard.cpp:72-76, target/cpp-ref/src/app/KaInspectorPanel.cpp:166-169).
#[test]
fn line_draw_tool_holds_card_and_shows_drawing_sentence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.active_tool = MapToolKind::DrawLine;
    app.resync_card_on_tool_change();
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), 2));
    app.sync_card_with_selection();
    assert!(app.home_map.card.is_none(), "no selection follow while drawing a line");

    let ctx = egui::Context::default();
    let out = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_ri_attribute_card(ui);
        });
    });
    let texts: String = out
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        texts.contains("도형을 다 그리면 그 기록이 여기에 나옵니다."),
        "unexpected inspector sentence: {texts}"
    );

    app.active_tool = MapToolKind::Pan;
    app.resync_card_on_tool_change();
    assert_eq!(app.home_map.card.as_ref().map(|c| c.fid), Some(2));
}

fn rendered_card_texts(app: &mut StrataAppState) -> String {
    let ctx = egui::Context::default();
    let out = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.render_ri_attribute_card(ui);
        });
    });
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Ported from target/cpp-ref/tests/test_inspector_panel.cpp:103-121 (emptyAndDrawingSentences)
// and target/cpp-ref/src/app/KaInspectorPanel.cpp:162-173 for the four states supported before
// read-only reference cards are wired in the Rust feature card.
#[test]
fn inspector_panel_shows_empty_and_drawing_sentences() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let s_idle_empty = "지도에서 도형 하나를 고르면 기록이 여기에 나옵니다.";
    let s_draw_empty = "도형을 다 그리면 그 기록이 여기에 나옵니다.";
    let s_draw_edit = "그리는 중에도 여기서 바로 고칠 수 있습니다. 지도를 누르면 이어서 그립니다.";
    let s_draw_ref = "그리는 중입니다. 이 기록은 참조 자료라 고치지 않습니다.";

    // 1. Not drawing, no card.
    app.active_tool = MapToolKind::Pan;
    app.clear_card();
    let t1 = rendered_card_texts(&mut app);
    assert!(t1.contains(s_idle_empty), "{t1}");

    // 2. Drawing (line and polygon), no card.
    for tool in [MapToolKind::DrawLine, MapToolKind::DrawPolygon] {
        app.active_tool = tool;
        let t2 = rendered_card_texts(&mut app);
        assert!(t2.contains(s_draw_empty), "{tool:?}: {t2}");
        assert!(!t2.contains(s_idle_empty), "{tool:?}: {t2}");
    }

    // 3. Drawing (line and polygon) with an editable record on the card: card stays visible alongside sentence.
    app.set_card_target("feature_poly", 1, false);
    for tool in [MapToolKind::DrawLine, MapToolKind::DrawPolygon] {
        app.active_tool = tool;
        let t3 = rendered_card_texts(&mut app);
        assert!(t3.contains("1호 주거지"), "card must stay visible while drawing ({tool:?}): {t3}");
        assert!(t3.contains(s_draw_edit), "{tool:?}: {t3}");
        assert!(!t3.contains(s_draw_empty), "{tool:?}: {t3}");
    }

    // 4. Not drawing with a card: card stays visible and no inspector sentence is shown.
    app.active_tool = MapToolKind::Pan;
    let t4 = rendered_card_texts(&mut app);
    assert!(t4.contains("1호 주거지"), "{t4}");
    for s in [s_idle_empty, s_draw_empty, s_draw_edit, s_draw_ref] {
        assert!(!t4.contains(s), "unexpected sentence {s:?} when not drawing with card: {t4}");
    }
}
