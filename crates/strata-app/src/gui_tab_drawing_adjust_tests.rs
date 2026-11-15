//! 「지도 조정」 and 「화면 맞춤」 of a sheet, on a map frame of 600 x 400 screen points that shows the paper
//! at 2 points to the millimetre.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, pos2, vec2, Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;

const PX_PER_MM: f32 = 2.0;

fn map_frame() -> Rect {
    Rect::from_min_size(pos2(100.0, 100.0), vec2(600.0, 400.0))
}

/// A sheet of 1:5,000 with its middle on a round place.
fn sheet() -> (StrataAppState, egui::Context) {
    let mut app = StrataAppState::new_startup();
    app.drawing.sheet_center = Some([250_000.0, 400_000.0]);
    app.drawing.sheet.scale_denominator = 5000;
    (app, egui::Context::default())
}

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>, modifiers: Modifiers) {
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))), events, modifiers, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.adjust_sheet_map(ui, map_frame(), PX_PER_MM));
    });
}

fn wheel(app: &mut StrataAppState, ctx: &egui::Context, notches: f32, modifiers: Modifiers) {
    frame(app, ctx, vec![Event::MouseWheel { unit: MouseWheelUnit::Line, delta: vec2(0.0, notches), modifiers }], modifiers);
}

/// The ground under a screen point of the frame.
fn ground(app: &StrataAppState, at: Pos2) -> [f64; 2] {
    let per_metre = 1000.0 / f64::from(app.drawing.sheet.scale_denominator) * f64::from(PX_PER_MM);
    let (centre, middle) = (app.drawing.sheet_center.unwrap(), map_frame().center());
    [centre[0] + f64::from(at.x - middle.x) / per_metre, centre[1] - f64::from(at.y - middle.y) / per_metre]
}

fn near(found: [f64; 2], wanted: [f64; 2], metres: f64) -> bool {
    (found[0] - wanted[0]).abs() < metres && (found[1] - wanted[1]).abs() < metres
}

/// Outside 「지도 조정」 the frame takes neither the wheel nor a drag: the sheet stays as it is composed.
#[test]
fn outside_the_mode_the_frame_takes_no_wheel_and_no_drag() {
    let (mut app, ctx) = sheet();
    let at = pos2(550.0, 200.0);
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)], Modifiers::default());
    wheel(&mut app, &ctx, 1.0, Modifiers::default());
    frame(&mut app, &ctx, vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }], Modifiers::default());
    frame(&mut app, &ctx, vec![Event::PointerMoved(pos2(600.0, 250.0))], Modifiers::default());
    assert_eq!((app.drawing.sheet.scale_denominator, app.drawing.sheet_center), (5000, Some([250_000.0, 400_000.0])));
}

/// The wheel zooms a fifth to the notch around the pointer, a third of that step with Ctrl, and never further
/// in than 1:10 (`KaDrawingTools.cpp:102-121`, `:138-150`; `LayoutService::zoomExtentAtAnchor`, ported from
/// tests/test_workflow.cpp:layoutWheelZoom_keepsPointUnderCursor: the ground under the pointer stays).
#[test]
fn the_wheel_zooms_around_the_pointer() {
    let (mut app, ctx) = sheet();
    app.drawing.map_adjust_active = true;
    let at = pos2(550.0, 200.0);
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)], Modifiers::default());
    let under_the_pointer = ground(&app, at);

    wheel(&mut app, &ctx, 1.0, Modifiers::default());
    assert_eq!(app.drawing.sheet.scale_denominator, 4167, "5000 / 1.2");
    assert!(near(ground(&app, at), under_the_pointer, 0.1), "{:?} for {under_the_pointer:?}", ground(&app, at));

    wheel(&mut app, &ctx, -1.0, Modifiers::default());
    assert_eq!(app.drawing.sheet.scale_denominator, 5000, "and out again");
    assert!(near(ground(&app, at), under_the_pointer, 0.1));

    wheel(&mut app, &ctx, -1.0, Modifiers::CTRL);
    assert_eq!(app.drawing.sheet.scale_denominator, 5350, "with Ctrl 7 % to the notch");

    app.drawing.sheet.scale_denominator = 11;
    wheel(&mut app, &ctx, 1.0, Modifiers::default());
    wheel(&mut app, &ctx, 1.0, Modifiers::default());
    assert_eq!(app.drawing.sheet.scale_denominator, 10);
}

/// A drag moves the map under the frame: the ground that was under the pointer is still under it
/// (`KaDrawingTools.cpp:55-100` moveContent).
#[test]
fn a_drag_moves_the_ground_with_the_pointer() {
    let (mut app, ctx) = sheet();
    app.drawing.map_adjust_active = true;
    let (from, to) = (pos2(300.0, 300.0), pos2(340.0, 280.0));
    let grabbed = ground(&app, from);
    frame(&mut app, &ctx, vec![Event::PointerMoved(from)], Modifiers::default());
    frame(&mut app, &ctx, vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }], Modifiers::default());
    frame(&mut app, &ctx, vec![Event::PointerMoved(to)], Modifiers::default());
    frame(&mut app, &ctx, vec![Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }], Modifiers::default());
    assert!(near(ground(&app, to), grabbed, 1e-6), "{:?} for {grabbed:?}", ground(&app, to));
    assert_eq!(app.drawing.sheet.scale_denominator, 5000);
}

/// 「조정 끝」 takes the next standard scale that still holds what the frame shows and says so
/// (`endActivateMap`, KaDrawingStudio.cpp:3588-3607; `snapMapScaleToNice`, :2830-2854); the button turns the
/// mode on with the C++ words (`beginActivateMap`, :3194-3210).
#[test]
fn ending_the_mode_takes_the_next_standard_scale() {
    let (mut app, _ctx) = sheet();
    app.toggle_sheet_map_adjust();
    assert!(app.drawing.map_adjust_active);
    assert_eq!(app.status_left_message(), "지도 조정: 드래그로 이동, 휠로 확대. 끝나면 「지도조정끝」.");
    app.drawing.sheet.scale_denominator = 4167;
    app.toggle_sheet_map_adjust();
    assert!(!app.drawing.map_adjust_active);
    assert_eq!(app.drawing.sheet.scale_denominator, 5000);
    assert_eq!(app.status_left_message(), "지도 조정을 끝냈습니다.");
}

/// 「화면 맞춤」 ends the adjusting and takes the middle and the scale of the map view, with the C++ words
/// (`fitToMapCanvas`, KaDrawingStudio.cpp:3623-3635; `applyCanvasViewToLayoutMap`, LayoutService.cpp:968-978).
#[test]
fn fit_takes_the_middle_and_the_scale_of_the_map_view() {
    let (mut app, _ctx) = sheet();
    app.drawing.map_adjust_active = true;
    app.home_map.viewport.center_xy = [260_000.0, 410_000.0];
    app.home_map.viewport.meters_per_pixel = 6108.0 * 0.0254 / 96.0;
    app.fit_sheet_to_map_view();
    assert!(!app.drawing.map_adjust_active);
    assert_eq!(app.drawing.sheet_center, Some([260_000.0, 410_000.0]));
    assert_eq!(app.drawing.sheet.scale_denominator, 6108);
    assert_eq!(app.status_left_message(), "지도 화면과 같은 범위·축척으로 맞췄습니다. 축척 1 : 6108");
}
