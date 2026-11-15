//! Taking an item of a sheet with the mouse and putting it somewhere else on the paper.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, vec2, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

use super::*;
use crate::app_state::StrataAppState;
use crate::tab_drawing::SheetItem;

const ALL: [SheetItem; 6] = [SheetItem::Legend, SheetItem::TitleBlock, SheetItem::NorthArrow, SheetItem::ScaleBar, SheetItem::ScaleText, SheetItem::CrsLabel];

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) {
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), events, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
    });
}

/// A sheet that was painted once, so that its items are known.
fn sheet() -> (StrataAppState, egui::Context) {
    let (mut app, ctx) = (StrataAppState::new_startup(), egui::Context::default());
    frame(&mut app, &ctx, vec![]);
    (app, ctx)
}

/// Where an item stands on the screen now.
fn place(app: &StrataAppState, item: SheetItem) -> Rect {
    let (_, placed, moved) = app.drawing.item_rects.iter().find(|(which, ..)| *which == item).copied().unwrap_or_else(|| panic!("{item:?} is not on the sheet"));
    placed.translate(moved)
}

fn drag(app: &mut StrataAppState, ctx: &egui::Context, from: Pos2, by: Vec2) {
    let button = |at: Pos2, pressed: bool| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() };
    frame(app, ctx, vec![Event::PointerMoved(from)]);
    frame(app, ctx, vec![button(from, true)]);
    frame(app, ctx, vec![Event::PointerMoved(from + by * 0.5)]);
    frame(app, ctx, vec![Event::PointerMoved(from + by)]);
    frame(app, ctx, vec![button(from + by, false)]);
    frame(app, ctx, vec![]);
}

fn near(found: Pos2, wanted: Pos2) -> bool {
    (found - wanted).length() < 0.6
}

/// Owner 2026-11-15: 「조판에서 자동으로 배치를 하지만 모든것을 사용자가 움직일수있게하라」. Every item the sheet
/// places by itself (legend, title block, north arrow, scale bar, scale text, coordinate system) is taken
/// with the mouse and stays where it is let go; the others stay where they were. What is kept is the way
/// in millimetres of paper.
#[test]
fn every_item_of_the_sheet_is_taken_and_put_somewhere_else() {
    for item in ALL {
        let (mut app, ctx) = sheet();
        let before: Vec<Rect> = ALL.iter().map(|other| place(&app, *other)).collect();
        let at = place(&app, item);
        // Towards the middle of the paper, so that no edge of the paper is in the way.
        let way = vec2(if at.center().x < 600.0 { 40.0 } else { -40.0 }, if at.center().y < 430.0 { 30.0 } else { -30.0 });
        drag(&mut app, &ctx, at.center(), way);
        assert!(near(place(&app, item).min, at.min + way), "{item:?}: {:?} from {:?}", place(&app, item), at);
        for (other, was) in ALL.iter().zip(&before).filter(|(other, _)| **other != item) {
            assert!(near(place(&app, *other).min, was.min), "{other:?} moved with {item:?}");
        }
        let [x, y] = app.drawing.item_offsets_mm[&item];
        assert!((x / y).abs() > 1.32 && (x / y).abs() < 1.35 && x.signum() == way.x.signum() && y.signum() == way.y.signum(), "{item:?}: {x} {y} mm");
    }
}

/// The frame of the legend is painted where the legend was put.
#[test]
fn a_moved_item_is_painted_at_its_new_place() {
    let (mut app, ctx) = sheet();
    let at = place(&app, SheetItem::Legend);
    drag(&mut app, &ctx, at.center(), vec2(-60.0, 45.0));
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), ..Default::default() };
    let out = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
    });
    let wanted = at.translate(vec2(-60.0, 45.0));
    let frames: Vec<Rect> = out.shapes.iter().filter_map(|clipped| if let egui::Shape::Rect(rect) = &clipped.shape { Some(rect.rect) } else { None }).collect();
    assert!(frames.iter().any(|rect| near(rect.min, wanted.min) && near(rect.max, wanted.max)), "no frame at {wanted:?}");
}

/// An item does not leave the paper, and what is kept is where it stands, not how far the mouse went: a
/// small way back is seen at once.
#[test]
fn an_item_stays_on_the_paper() {
    let (mut app, ctx) = sheet();
    let at = place(&app, SheetItem::NorthArrow);
    drag(&mut app, &ctx, at.center(), vec2(900.0, 0.0));
    let held = place(&app, SheetItem::NorthArrow);
    assert!(held.left() > at.left() && held.left() < at.left() + 400.0, "kept on the paper: {held:?}");
    drag(&mut app, &ctx, held.center(), vec2(-10.0, 0.0));
    assert!(near(place(&app, SheetItem::NorthArrow).min, held.min + vec2(-10.0, 0.0)));
}

/// One tool of the sheet at a time: while the map is adjusted or coordinate points are placed, the items
/// stay. 「항목 이동」 goes back to taking items (`useSelectTool`, KaDrawingStudio.cpp:1891-1897): not while
/// the map is adjusted, and it ends the placing of points.
#[test]
fn items_stay_while_another_tool_of_the_sheet_is_on() {
    let (mut app, ctx) = sheet();
    let at = place(&app, SheetItem::TitleBlock);
    app.drawing.map_adjust_active = true;
    drag(&mut app, &ctx, at.center(), vec2(-40.0, -30.0));
    assert!(near(place(&app, SheetItem::TitleBlock).min, at.min));
    app.use_sheet_select_tool();
    assert!(app.drawing.map_adjust_active, "the adjusting is ended with its own button");

    app.drawing.map_adjust_active = false;
    app.drawing.coord_callout_mode = true;
    drag(&mut app, &ctx, at.center(), vec2(-40.0, -30.0));
    assert!(near(place(&app, SheetItem::TitleBlock).min, at.min));
    app.use_sheet_select_tool();
    assert!(!app.drawing.coord_callout_mode);
    assert_eq!(app.status_left_message(), "항목을 눌러 끌면 옮겨집니다.");
}

/// The offset that keeps an item on the paper.
#[test]
fn the_way_of_an_item_ends_at_the_edge_of_the_paper() {
    let paper = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(500.0, 700.0));
    let placed = Rect::from_min_max(Pos2::new(400.0, 600.0), Pos2::new(480.0, 650.0));
    assert_eq!(kept_on_paper(vec2(-50.0, -20.0), placed, paper), vec2(-50.0, -20.0));
    assert_eq!(kept_on_paper(vec2(300.0, 300.0), placed, paper), vec2(20.0, 50.0));
    assert_eq!(kept_on_paper(vec2(-900.0, -900.0), placed, paper), vec2(-300.0, -500.0));
}
