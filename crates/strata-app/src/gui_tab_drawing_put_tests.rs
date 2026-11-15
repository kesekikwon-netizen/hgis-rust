//! Selecting an item of a sheet, taking it off with Delete, and putting it back with its button — ported from
//! the C++ studio (KaDrawingStudio.cpp: handleDeleteKey :3512-3522, deleteSelectedItems :3213-3257, the place
//! buttons :1606-1660, undoLastCoordCallout :1806-1821; tests/test_workflow.cpp:5594-5675).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, vec2, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::{MainTab, StrataAppState};
use crate::tab_drawing::{CoordCallout, SheetItem};

const ALL: [SheetItem; 6] = [SheetItem::Legend, SheetItem::TitleBlock, SheetItem::NorthArrow, SheetItem::ScaleBar, SheetItem::ScaleText, SheetItem::CrsLabel];

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) {
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), events, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
    });
}

fn sheet() -> (StrataAppState, egui::Context) {
    let (mut app, ctx) = (StrataAppState::new_startup(), egui::Context::default());
    frame(&mut app, &ctx, vec![]);
    (app, ctx)
}

fn place(app: &StrataAppState, item: SheetItem) -> Option<Rect> {
    app.drawing.item_rects.iter().find(|(which, ..)| *which == item).map(|(_, placed, moved)| placed.translate(*moved))
}

fn click(app: &mut StrataAppState, ctx: &egui::Context, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() };
    frame(app, ctx, vec![Event::PointerMoved(at)]);
    frame(app, ctx, vec![button(true)]);
    frame(app, ctx, vec![button(false)]);
    frame(app, ctx, vec![]);
}

/// What each item's button does: the C++ words it shows.
fn put(app: &mut StrataAppState, item: SheetItem) -> &'static str {
    app.put_item_on_sheet(item);
    match item {
        SheetItem::Legend => "범례를 넣었습니다. 끌어 옮기세요.",
        SheetItem::TitleBlock => "표제란을 넣었습니다. 조사구역에 조사명·유적명이 없어 ―로 두었습니다.",
        SheetItem::NorthArrow => "방위표를 넣었습니다. 끌어 옮기세요.",
        SheetItem::ScaleBar => "축척자를 넣었습니다. 끌어 옮기세요.",
        SheetItem::ScaleText => "축척 글자를 넣었습니다. 끌어 옮기세요.",
        SheetItem::CrsLabel => "좌표계를 넣었습니다. 끌어 옮기세요.",
    }
}

/// A click on an item selects it; a click on bare paper (the middle of the map) lets go of it.
#[test]
fn a_click_selects_an_item_and_a_click_on_bare_paper_lets_it_go() {
    let (mut app, ctx) = sheet();
    for item in ALL {
        let at = place(&app, item).unwrap().center();
        click(&mut app, &ctx, at);
        assert_eq!(app.drawing.selected_item, Some(item));
    }
    let map_middle = place(&app, SheetItem::Legend).unwrap().left_bottom() + vec2(-80.0, 120.0);
    click(&mut app, &ctx, map_middle);
    assert_eq!(app.drawing.selected_item, None);
}

/// Delete takes the selected item off the paper, with the C++ words; its button puts it back where the sheet
/// places it, selected, with the C++ words of that button.
#[test]
fn delete_takes_the_selected_item_off_until_its_button_puts_it_back() {
    for item in ALL {
        let (mut app, ctx) = sheet();
        let first = place(&app, item).unwrap();
        click(&mut app, &ctx, first.center());
        app.delete_sheet_selection();
        frame(&mut app, &ctx, vec![]);
        assert_eq!((place(&app, item), app.status_left_message()), (None, "선택한 항목 1개를 지웠습니다."), "{item:?}");
        assert_eq!(app.drawing.selected_item, None);
        let words = put(&mut app, item);
        frame(&mut app, &ctx, vec![]);
        assert_eq!((place(&app, item), app.drawing.selected_item, app.status_left_message()), (Some(first), Some(item), words), "{item:?}");
    }
}

/// An item that is still on the paper keeps the place the user gave it when its button is pressed; one that
/// was taken off comes back at the sheet's own place (`placeScaleLabel`: the rect is kept when there is one).
#[test]
fn a_button_keeps_a_moved_item_where_it_is_unless_it_was_taken_off() {
    let (mut app, ctx) = sheet();
    let own = place(&app, SheetItem::ScaleText).unwrap();
    app.drawing.item_offsets_mm.insert(SheetItem::ScaleText, [12.0, -6.0]);
    frame(&mut app, &ctx, vec![]);
    let moved = place(&app, SheetItem::ScaleText).unwrap();
    assert_ne!(moved, own);
    let _ = put(&mut app, SheetItem::ScaleText);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(place(&app, SheetItem::ScaleText), Some(moved));
    app.delete_sheet_selection();
    let _ = put(&mut app, SheetItem::ScaleText);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(place(&app, SheetItem::ScaleText), Some(own));
}

/// With nothing selected Delete says so; while coordinate points are placed it takes the last point.
#[test]
fn delete_without_a_selection_says_so_and_takes_the_last_coordinate_point_while_they_are_placed() {
    let (mut app, _ctx) = sheet();
    app.delete_sheet_selection();
    assert_eq!(app.status_left_message(), "지울 항목을 먼저 선택하세요.");
    app.drawing.coord_callout_mode = true;
    for _ in ["A", "B"] {
        app.drawing.coord_callouts.push(CoordCallout { ground: [200_000.0, 450_000.0] });
    }
    app.delete_sheet_selection();
    assert_eq!((app.drawing.coord_callouts.len(), app.status_left_message()), (1, "마지막 좌표점을 지웠습니다. 남은 점 1개."));
    app.delete_sheet_selection();
    assert_eq!((app.drawing.coord_callouts.len(), app.status_left_message()), (0, "좌표점을 모두 지웠습니다."));
    app.delete_sheet_selection();
    assert_eq!(app.status_left_message(), "지울 좌표점이 없습니다.");
}

/// In the whole window, Delete and Backspace on the 도면 tab go to the sheet, not to the map.
#[test]
fn delete_and_backspace_on_the_sheet_tab_go_to_the_sheet() {
    for key in [Key::Delete, Key::Backspace] {
        let (mut app, ctx) = (StrataAppState::new_startup(), egui::Context::default());
        app.open_studio_tab(MainTab::Drawing);
        let raw = |events: Vec<Event>| RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), events, ..Default::default() };
        let _ = ctx.run(raw(vec![]), |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
        app.drawing.selected_item = Some(SheetItem::NorthArrow);
        let press = Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() };
        let _ = ctx.run(raw(vec![press]), |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
        assert!(app.drawing.removed_items.contains(&SheetItem::NorthArrow), "{key:?}");
        assert_eq!(app.status_left_message(), "선택한 항목 1개를 지웠습니다.");
    }
}

/// What a taken-off item had painted is not on the sheet any more: no scale words, no coordinate system.
#[test]
fn a_taken_off_item_is_not_painted() {
    let (mut app, ctx) = sheet();
    let words = |app: &mut StrataAppState, ctx: &egui::Context| -> Vec<String> {
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), ..Default::default() };
        let out = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
        });
        out.shapes.iter().filter_map(|clipped| if let egui::Shape::Text(text) = &clipped.shape { Some(text.galley.text().to_owned()) } else { None }).collect()
    };
    let scale = crate::gui_tab_drawing::gui_tab_drawing_decoration::scale_text(app.drawing.sheet.scale_denominator);
    let before = words(&mut app, &ctx);
    assert!(before.contains(&scale) && before.iter().any(|w| w.contains("EPSG:")), "{before:?}");
    for item in [SheetItem::ScaleText, SheetItem::CrsLabel] {
        app.drawing.selected_item = Some(item);
        app.delete_sheet_selection();
    }
    let after = words(&mut app, &ctx);
    assert!(!after.contains(&scale) && !after.iter().any(|w| w.contains("EPSG:")), "{after:?}");
}

/// The status line of the 도면 tab shows the sheet's last words (the welcome before any), keeps them while
/// another tab is shown, and those words leave the other tab's status line (MainWindowExport.cpp:225-232).
#[test]
fn the_sheet_tab_status_line_shows_the_sheets_words() {
    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Drawing);
    app.sync_sheet_status();
    assert_eq!(app.status_left_message(), StrataAppState::DRAWING_STUDIO_WELCOME);
    app.pick_sheet_scale(2000);
    app.sync_sheet_status();
    assert_eq!(app.status_left_message(), "축척을 1 : 2000 로 맞췄습니다.");
    app.active_tab = MainTab::Map;
    app.sync_sheet_status();
    assert_ne!(app.status_left_message(), "축척을 1 : 2000 로 맞췄습니다.");
    app.active_tab = MainTab::Drawing;
    app.sync_sheet_status();
    assert_eq!(app.status_left_message(), "축척을 1 : 2000 로 맞췄습니다.");
}

/// A change of paper (C++ `resetPaper`, KaDrawingStudio.cpp:456-560) ends 「지도 조정」 and puts the four items
/// of the bottom band (north arrow, scale bar, scale words, coordinate system) at the new paper's own places;
/// the legend and the title block keep the way the user moved them.
#[test]
fn a_new_paper_puts_the_bottom_band_in_place_and_keeps_the_rest() {
    let mut app = StrataAppState::new_startup();
    for item in ALL {
        app.drawing.item_offsets_mm.insert(item, [5.0, -3.0]);
    }
    app.drawing.map_adjust_active = true;
    app.sheet_paper_changed();
    assert!(!app.drawing.map_adjust_active);
    for item in [SheetItem::NorthArrow, SheetItem::ScaleBar, SheetItem::ScaleText, SheetItem::CrsLabel] {
        assert!(!app.drawing.item_offsets_mm.contains_key(&item), "{item:?}");
    }
    for item in [SheetItem::Legend, SheetItem::TitleBlock] {
        assert_eq!(app.drawing.item_offsets_mm.get(&item), Some(&[5.0, -3.0]), "{item:?}");
    }
}
