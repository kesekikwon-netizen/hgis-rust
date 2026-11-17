//! 「도면 목록」 — ported from tests/test_drawing_studio.cpp:164-203 (sheetSet_storeAndReopenKeepsWorkingSheet)
//! and :135-162 (layerSets_applyOnlyWhenPicked), with the words of KaDrawingStudioSheets.cpp:71-170.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, vec2, Pos2, RawInput, Rect};

use super::SheetAsk;
use crate::app_state::StrataAppState;
use crate::tab_drawing::{CoordCallout, SheetItem};

/// 「지금 도면 보관…」 asks for a name, offering the drawing's title; the name given keeps the whole sheet and
/// says so; the same name again asks before it is replaced, and 「아니요」 keeps the first.
#[test]
fn a_sheet_is_kept_under_a_name_and_the_same_name_asks_first() {
    let mut app = StrataAppState::new_startup();
    app.ask_store_sheet();
    assert_eq!(app.drawing.sheet_ask, Some(SheetAsk::Name("조사도면".to_owned())));
    app.drawing.sheet.scale_denominator = 1_000;
    app.store_sheet_named("조사구역도 1:1000");
    assert_eq!(app.status_left_message(), "도면 「조사구역도 1:1000」을 보관했습니다. 「도면 목록」에서 다시 엽니다.");
    assert_eq!(app.sheet_set_rows(), ["조사구역도 1:1000"]);
    app.drawing.sheet.scale_denominator = 2_500;
    app.store_sheet_named("조사구역도 1:1000");
    assert_eq!(app.drawing.sheet_ask, Some(SheetAsk::Replace("조사구역도 1:1000".to_owned())));
    app.drawing.sheet_ask = None;
    assert_eq!(app.drawing.stored_sheets[0].sheet.scale_denominator, 1_000, "「아니요」 keeps the first");
    app.store_sheet("조사구역도 1:1000");
    assert_eq!((app.drawing.stored_sheets.len(), app.drawing.stored_sheets[0].sheet.scale_denominator), (1, 2_500));
    app.store_sheet_named("   ");
    assert_eq!(app.drawing.stored_sheets.len(), 1, "a blank name keeps nothing");
}

/// Opening a kept sheet brings back everything of it — paper, scale, the middle of its map, the items as
/// the user moved and took them off, its coordinate points, its title and the layers checked then — and the
/// kept copy stays in the list.
#[test]
fn opening_a_kept_sheet_brings_it_all_back() {
    let mut app = StrataAppState::new_startup();
    app.drawing.set_paper_and_orientation(strata_layout::sheet_composer::SheetPaperSize::A3, strata_layout::sheet_composer::SheetOrientation::Landscape);
    app.drawing.sheet.scale_denominator = 1_000;
    app.drawing.sheet_center = Some([220_000.0, 330_000.0]);
    app.drawing.item_offsets_mm.insert(SheetItem::Legend, [-20.0, 10.0]);
    app.drawing.removed_items.insert(SheetItem::CrsLabel);
    app.drawing.coord_callouts.push(CoordCallout { ground: [220_010.0, 330_020.0] });
    app.drawing.title_ko = "1호 주거지 실측도".to_owned();
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::SurveyArea);
    app.left_layers.entries[0].visible = false;
    app.home_map.real_survey.layer_visible.insert("조사구역".to_owned(), false);
    app.store_sheet("실측도");

    app.drawing = crate::tab_drawing::DrawingTabState { stored_sheets: app.drawing.stored_sheets.clone(), ..Default::default() };
    app.home_map.real_survey.layer_visible.insert("조사구역".to_owned(), true);
    app.left_layers.entries[0].visible = true;
    app.open_stored_sheet("실측도");

    let d = &app.drawing;
    assert_eq!((d.sheet.width_mm, d.sheet.height_mm, d.sheet.scale_denominator), (420.0, 297.0, 1_000));
    assert_eq!(d.sheet_center, Some([220_000.0, 330_000.0]));
    assert_eq!(d.item_offsets_mm.get(&SheetItem::Legend), Some(&[-20.0, 10.0]));
    assert!(d.removed_items.contains(&SheetItem::CrsLabel));
    assert_eq!(d.coord_callouts[0].ground, [220_010.0, 330_020.0]);
    assert_eq!(d.title_ko, "1호 주거지 실측도");
    assert!(!app.home_map.real_survey.is_layer_visible("조사구역"), "the layers checked then");
    assert_eq!(app.status_left_message(), "도면 「실측도」을 열고 그때의 레이어 켜짐을 적용했습니다.");
    assert_eq!(app.sheet_set_rows(), ["실측도"], "the kept copy stays");
}

/// A kept sheet goes with its layer set; the working sheet stays.
#[test]
fn a_kept_sheet_can_be_taken_out_of_the_list() {
    let mut app = StrataAppState::new_startup();
    app.store_sheet("나");
    app.store_sheet("가");
    assert_eq!(app.sheet_set_rows(), ["가", "나"], "sorted by name");
    app.drawing.sheet.scale_denominator = 750;
    app.remove_stored_sheet("가");
    assert_eq!(app.sheet_set_rows(), ["나"]);
    assert_eq!(app.drawing.sheet.scale_denominator, 750);
    assert_eq!(app.status_left_message(), "보관한 도면 「가」을 지웠습니다.");
}

fn menu_labels(app: &mut StrataAppState) -> Vec<(String, bool)> {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    app.drawing.sheet_set_menu_open = true;
    let mut labels = Vec::new();
    for _ in 0..3 {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), ..Default::default() };
        let out = ctx.run(raw, |ctx| app.render_sheet_set_menu(ctx, Pos2::new(800.0, 700.0)));
        labels = out.platform_output.accesskit_update.map_or_else(Vec::new, |update| {
            update.nodes.iter().filter_map(|(_, node)| node.label().map(|label| (label.to_owned(), node.is_disabled()))).collect()
        });
    }
    labels
}

/// The menu as the C++ fills it (KaDrawingSheetSet.cpp:131-162): keep, then one 「열기:」 row a kept sheet and
/// the delete rows, or a greyed 「보관한 도면이 없습니다」, and the greyed note about the submit PDF.
#[test]
fn the_menu_lists_the_kept_sheets() {
    let mut app = StrataAppState::new_startup();
    let empty = menu_labels(&mut app);
    for (label, disabled) in [("지금 도면 보관…", false), ("보관한 도면이 없습니다", true), ("제출 PDF는 지금 열린 도면으로 만듭니다", true)] {
        assert!(empty.contains(&(label.to_owned(), disabled)), "{label} in {empty:?}");
    }
    app.store_sheet("조사구역도");
    let kept = menu_labels(&mut app);
    for label in ["열기: 조사구역도", "보관한 도면 지우기"] {
        assert!(kept.iter().any(|(l, disabled)| l == label && !disabled), "{label} in {kept:?}");
    }
}

/// The name window: what is typed and Enter keep the sheet under that name; Esc keeps nothing.
#[test]
fn the_name_window_keeps_the_sheet_under_the_typed_name() {
    use eframe::egui::{Event, Key, Modifiers};
    let run = |app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>| {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), events, ..Default::default() };
        let _ = ctx.run(raw, |ctx| app.render_sheet_ask(ctx));
    };
    let key = |key: Key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() };
    let (mut app, ctx) = (StrataAppState::new_startup(), egui::Context::default());
    app.drawing.sheet_ask = Some(SheetAsk::Name(String::new()));
    run(&mut app, &ctx, vec![]);
    run(&mut app, &ctx, vec![Event::Text("1호 주거지 실측도".to_owned())]);
    run(&mut app, &ctx, vec![key(Key::Enter)]);
    assert_eq!(app.sheet_set_rows(), ["1호 주거지 실측도"]);
    assert_eq!(app.drawing.sheet_ask, None);
    app.drawing.sheet_ask = Some(SheetAsk::Name("다른 이름".to_owned()));
    run(&mut app, &ctx, vec![]);
    run(&mut app, &ctx, vec![key(Key::Escape)]);
    assert_eq!((app.drawing.sheet_ask.clone(), app.sheet_set_rows().len()), (None, 1));
}
