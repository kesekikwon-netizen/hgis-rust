//! 「좌표점」 on the sheet with the mouse — ported from tests/test_workflow.cpp:5511-5555 (the coordinate point
//! part of drawingStudioLayoutAndExport) and the behaviour of KaDrawingStudio.cpp:1662-1889.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use eframe::egui::{self, epaint::ClippedShape, vec2, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

use super::super::sheet_coord_points::xy_text;
use crate::app_state::{MainTab, StrataAppState};

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) -> Vec<ClippedShape> {
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), events, ..Default::default() };
    let shapes = ctx
        .run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
        })
        .shapes;
    // As the window does once a frame: the 도면 tab's status line takes the sheet's words.
    app.sync_sheet_status();
    shapes
}

fn click(app: &mut StrataAppState, ctx: &egui::Context, at: Pos2, button: PointerButton) {
    let press = |pressed: bool| Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::default() };
    frame(app, ctx, vec![Event::PointerMoved(at)]);
    frame(app, ctx, vec![press(true)]);
    frame(app, ctx, vec![press(false)]);
}

/// A new survey on the 도면 tab, painted once, with points being placed.
fn sheet(dir: &std::path::Path) -> (StrataAppState, egui::Context) {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.drawing.sheet_center = Some([220_000.0, 330_000.0]);
    app.open_studio_tab(MainTab::Drawing);
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, vec![]);
    app.begin_place_coord_point();
    app.sync_sheet_status();
    (app, ctx)
}

fn paper(app: &StrataAppState) -> [Rect; 2] {
    app.drawing.paper_on_screen.expect("the sheet was painted")
}

fn place_of_words(shapes: &[ClippedShape], words: &str) -> Option<Pos2> {
    shapes.iter().find_map(|clipped| match &clipped.shape {
        egui::Shape::Text(text) if text.galley.text() == words => Some(text.pos),
        _ => None,
    })
}

/// 「좌표점」 says how to place; a click on the map frame puts point A on the ground under it, with its X
/// (northing) and Y (easting) in the status line; a click on the white paper puts none and says why.
#[test]
fn a_click_on_the_map_frame_places_a_point_and_the_paper_says_no() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, ctx) = sheet(dir.path());
    assert!(app.drawing.coord_callout_mode);
    assert_eq!(app.status_left_message(), "지도 칸에서 찍으세요. 선·점에 자석이 붙습니다. 우클릭·Delete·Ctrl+Z는 마지막 점 지우기.");
    let [sheet, map] = paper(&app);
    let margin = Pos2::new(sheet.left() + 4.0 * app.drawing.screen_px_per_mm.unwrap(), sheet.center().y);
    click(&mut app, &ctx, margin, PointerButton::Primary);
    assert!(app.drawing.coord_callouts.is_empty());
    assert_eq!(app.status_left_message(), "지도 칸 안에서 찍으세요. 흰 용지에는 점이 안 생깁니다.");
    click(&mut app, &ctx, map.center(), PointerButton::Primary);
    let ground = app.drawing.coord_callouts[0].ground;
    assert!((ground[0] - 220_000.0).abs() < 2.0 && (ground[1] - 330_000.0).abs() < 2.0, "{ground:?}");
    assert_eq!(app.status_left_message(), format!("A점  X={:.3}  Y={:.3}", ground[1], ground[0]));
}

/// A point keeps its ground: when the map of the sheet moves 100 m east, its words move 100 m west on the
/// paper (`relayoutCoordCallouts`).
#[test]
fn a_point_follows_the_map() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, ctx) = sheet(dir.path());
    let middle = paper(&app)[1].center();
    click(&mut app, &ctx, middle, PointerButton::Primary);
    let ground = app.drawing.coord_callouts[0].ground;
    let words = xy_text(ground[0], ground[1]);
    let before = place_of_words(&frame(&mut app, &ctx, vec![]), &words).expect("the words of the point");
    app.drawing.sheet_center = Some([220_100.0, 330_000.0]);
    let after = place_of_words(&frame(&mut app, &ctx, vec![]), &words).expect("the words of the point");
    let per_metre = f64::from(app.drawing.screen_px_per_mm.unwrap()) * 1000.0 / f64::from(app.drawing.sheet.scale_denominator);
    assert!((f64::from(before.x - after.x) - 100.0 * per_metre).abs() < 0.6, "{before:?} {after:?}");
    assert!((before.y - after.y).abs() < 0.6);
}

/// The magnet takes the corner of a survey shape within 18 screen points of the click, and the status line
/// says 「(자석)」.
#[test]
fn the_magnet_takes_a_corner_of_the_survey() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, ctx) = sheet(dir.path());
    let (x, y) = (220_010.0, 330_005.0);
    let square = format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + 50.0, x + 50.0, y + 50.0, y + 50.0);
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::FeaturePoly);
    app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").unwrap().layer.add_unsaved_feature(square, BTreeMap::new());
    let middle = paper(&app)[1].center();
    click(&mut app, &ctx, middle, PointerButton::Primary);
    let ground = app.drawing.coord_callouts[0].ground;
    assert!((ground[0] - x).abs() < 1e-6 && (ground[1] - y).abs() < 1e-6, "{ground:?}");
    assert!(app.status_left_message().ends_with("  (자석)"));
}

/// A right click takes the last point off, with the C++ words; 「항목 이동」 ends the placing.
#[test]
fn a_right_click_takes_the_last_point_off() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, ctx) = sheet(dir.path());
    let map = paper(&app)[1];
    click(&mut app, &ctx, map.center(), PointerButton::Primary);
    click(&mut app, &ctx, map.center() + vec2(40.0, 30.0), PointerButton::Primary);
    assert_eq!(app.drawing.coord_callouts.len(), 2);
    click(&mut app, &ctx, map.center(), PointerButton::Secondary);
    assert_eq!((app.drawing.coord_callouts.len(), app.status_left_message()), (1, "마지막 좌표점을 지웠습니다. 남은 점 1개."));
    app.use_sheet_select_tool();
    assert!(!app.drawing.coord_callout_mode);
}

/// In the whole window, Ctrl+Z while points are placed on the sheet takes the last point, not a map edit.
#[test]
fn ctrl_z_while_points_are_placed_takes_the_last_point() {
    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Drawing);
    app.begin_place_coord_point();
    app.drawing.coord_callouts.push(crate::tab_drawing::CoordCallout { ground: [220_000.0, 330_000.0] });
    let ctx = egui::Context::default();
    let raw = |events: Vec<Event>| RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), events, modifiers: Modifiers::CTRL, ..Default::default() };
    let undo = Event::Key { key: egui::Key::Z, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::CTRL };
    let _ = ctx.run(raw(vec![undo]), |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
    assert!(app.drawing.coord_callouts.is_empty());
    assert_eq!(app.status_left_message(), "좌표점을 모두 지웠습니다.");
}
