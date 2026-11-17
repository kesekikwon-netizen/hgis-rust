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

// Free legend placement follows target/cpp-ref/src/app/KaDrawingStudio.cpp:3394-3401.
#[test]
fn legend_can_leave_paper_and_be_dragged_back() {
    let (mut app, ctx) = sheet();
    let at = place(&app, SheetItem::Legend);
    let paper = app.drawing.paper_on_screen.expect("painted paper")[0];
    let way = vec2(paper.right() - at.left() + 30.0, 0.0);
    drag(&mut app, &ctx, at.center(), way);
    let outside = place(&app, SheetItem::Legend);
    assert!(outside.left() > paper.right(), "legend is clamped to paper: {outside:?}");
    drag(&mut app, &ctx, outside.center(), -way);
    assert!(near(place(&app, SheetItem::Legend).min, at.min), "off-page legend cannot be recovered");
}

// Real corner interactions, target/cpp-ref/src/app/KaDrawingStudio.cpp:3370-3407.
#[test]
fn legend_corner_resizes_both_dimensions_without_moving_the_other_corner() {
    let (mut app, ctx) = sheet();
    app.drawing.selected_item = Some(SheetItem::Legend);
    frame(&mut app, &ctx, vec![]);
    let at = place(&app, SheetItem::Legend);
    drag(&mut app, &ctx, at.right_bottom() + vec2(3.0, 3.0), vec2(70.0, 50.0));
    let bigger = place(&app, SheetItem::Legend);
    assert!(near(bigger.min, at.min), "opposite corner moved");
    assert!(near(bigger.max, at.max + vec2(70.0, 50.0)), "handle only moves the item: {at:?} -> {bigger:?}");
    drag(&mut app, &ctx, bigger.right_bottom() + vec2(3.0, 3.0), vec2(-30.0, -20.0));
    let smaller = place(&app, SheetItem::Legend);
    assert!(near(smaller.max, bigger.max - vec2(30.0, 20.0)), "cannot shrink legend");
    app.store_sheet("resized legend");
    app.drawing.legend_size_mm = None;
    app.drawing.item_offsets_mm.clear();
    app.open_stored_sheet("resized legend");
    frame(&mut app, &ctx, vec![]);
    assert!(near(place(&app, SheetItem::Legend).max, smaller.max), "stored sheet lost the manual legend geometry");
}

#[test]
fn drawing_properties_do_not_extend_past_their_available_width() {
    for width in [230, 268, 320] {
        let mut app = StrataAppState::new_startup();
        let rendered = crate::test_mods::ui_snapshot_support::render([width, 720], |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_right_panel(ui));
        });
        for (text, rect) in rendered.text_rects() {
            assert!(rect.left() >= -0.5 && rect.right() <= width as f32 + 0.5, "{width}px: {text} extends outside panel: {rect:?}");
        }
        rendered.save_if_requested(&format!("drawing-properties-{width}"));
    }
}

#[test]
fn legend_letters_fit_the_resized_box_and_grow_with_it() {
    let mut app = StrataAppState::new_startup();
    for key in [strata_types::schema::DomainLayerKey::SurveyArea, strata_types::schema::DomainLayerKey::FeatureLine] {
        app.left_layers.ensure_domain_layer(key);
    }
    let mut sizes = Vec::new();
    for size in [[30.0, 18.0], [80.0, 60.0]] {
        app.drawing.legend_size_mm = Some(size);
        let rendered = crate::test_mods::ui_snapshot_support::render([1200, 900], |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
        });
        let card = place(&app, SheetItem::Legend);
        let (title, title_rect) = rendered.text_rects().into_iter().find(|(text, _)| text == "범례").expect("real legend title");
        assert!(!title.is_empty());
        sizes.push(title_rect.height());
        assert!(card.contains_rect(title_rect), "title outside resized legend");
        let px = app.drawing.screen_px_per_mm.expect("painted scale");
        assert!((card.width() / px - size[0]).abs() < 0.1, "manual width ignored");
        for name in app.sheet_legend_layer_names() {
            let (_, rect) = rendered.text_rects().into_iter().find(|(text, _)| *text == name).expect("real layer name");
            assert!(card.expand(0.5).contains_rect(rect), "{name} clips at the legend boundary: {rect:?}");
        }
        rendered.save_if_requested(&format!("legend-fit-{}", size[0]));
    }
    assert!(sizes[1] > sizes[0] * 1.2, "letters do not grow with the box: {sizes:?}");
}
