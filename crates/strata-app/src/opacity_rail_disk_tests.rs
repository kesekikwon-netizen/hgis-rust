//! Map and drawing card positions in the user ini. C++ keys `ui/opacityRail/map` and
//! `ui/opacityRail/drawing` (`KaLayerOpacityRail.cpp:13-15`, `:101-105`, `:248-249`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Color32, Event, PointerButton, Pos2, Rect, Shape, Vec2};
use strata_storage::qt_ini::{escaped_point, read_point, IniFile};

use crate::app_state::StrataAppState;

fn mark() -> Color32 {
    Color32::from_rgb(1, 2, 3)
}
fn card_fill() -> Color32 {
    Color32::from_rgba_unmultiplied(252, 251, 247, 248)
}

fn ini_with(map: Option<(i32, i32)>, drawing: Option<(i32, i32)>) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp ini dir");
    let path = dir.path().join("ka-hgis.ini");
    let mut file = IniFile::from_text("[ui]\r\nlook=paper\r\n".to_owned());
    if let Some((x, y)) = map {
        file.set_value("ui", "opacityRail\\map", &escaped_point(x, y));
    }
    if let Some((x, y)) = drawing {
        file.set_value("ui", "opacityRail\\drawing", &escaped_point(x, y));
    }
    file.write(&path).unwrap();
    (dir, path)
}

fn app_at(path: &std::path::Path) -> (StrataAppState, egui::Context) {
    let mut app = StrataAppState::new_startup();
    app.attach_user_settings(Some(path.to_path_buf()));
    let ctx = egui::Context::default();
    (app, ctx)
}

fn press(at: Pos2, down: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: down,
        modifiers: Default::default(),
    }
}

fn run(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>, drawing: bool) -> egui::FullOutput {
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
        events,
        ..Default::default()
    };
    ctx.run(raw, |ctx| {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            if drawing {
                app.paint_drawing_sheet(ui);
            } else {
                let host = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
                ui.painter().rect_filled(host, 0.0, mark());
                let _ = app.handle_map_opacity_pointer(ui, host, false);
                let p = ui.painter_at(host);
                app.paint_map_opacity_rail(ctx, ui, &p, host, false);
            }
        });
    })
}

fn painted(output: &egui::FullOutput, fill: Color32) -> Rect {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r) if r.fill == fill => Some(r.rect),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing rect {fill:?}"))
}

fn offset(card: Rect, host: Rect) -> Vec2 {
    card.min - host.min
}

fn near(got: Vec2, want: Vec2) {
    assert!((got - want).length() <= 0.5, "placement {got:?}, want {want:?}");
}

/// A temp ini restores the map point and the drawing point separately. The missing one stays lower left.
#[test]
fn ini_restores_map_and_drawing_points_independently() {
    let (_dir, path) = ini_with(Some((40, 90)), Some((70, 40)));
    let (mut app, ctx) = app_at(&path);
    let map = run(&mut app, &ctx, vec![], false);
    let (host, card) = (painted(&map, mark()), painted(&map, card_fill()));
    near(offset(card, host), Vec2::new(40.0, 90.0));

    let drawing = run(&mut app, &ctx, vec![], true);
    let (host, card) = (painted(&drawing, Color32::from_rgb(245, 243, 236)), painted(&drawing, card_fill()));
    near(offset(card, host), Vec2::new(70.0, 40.0));

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("look=paper"));
    assert_eq!(read_point(IniFile::from_text(text).value("ui", "opacityRail\\map").unwrap()), Some((40, 90)));
}

/// A drag keeps its place when the same key is read again, and a click that does not move is not saved.
#[test]
fn a_late_read_keeps_a_dragged_card_and_a_still_click_is_not_saved() {
    let (_dir, path) = ini_with(Some((40, 90)), Some((70, 40)));
    let (mut app, ctx) = app_at(&path);
    let first = run(&mut app, &ctx, vec![], true);
    let (host, card) = (painted(&first, Color32::from_rgb(245, 243, 236)), painted(&first, card_fill()));
    let from = card.min + Vec2::new(12.0, 10.0);
    let by = Vec2::new(50.0, -20.0);
    run(&mut app, &ctx, vec![Event::PointerMoved(from), press(from, true)], true);
    run(&mut app, &ctx, vec![Event::PointerMoved(from + by)], true);
    let mut file = IniFile::read(&path).unwrap();
    file.set_value("ui", "opacityRail\\drawing", &escaped_point(5, 5));
    file.write(&path).unwrap();
    app.reload_opacity_rails();
    let during = run(&mut app, &ctx, vec![], true);
    let (host_now, card_now) = (painted(&during, Color32::from_rgb(245, 243, 236)), painted(&during, card_fill()));
    near(offset(card_now, host_now), offset(card, host) + by);
    run(&mut app, &ctx, vec![press(from + by, false)], true);
    let saved = IniFile::read(&path).unwrap();
    assert_eq!(read_point(saved.value("ui", "opacityRail\\map").unwrap()), Some((40, 90)));
    assert_eq!(read_point(saved.value("ui", "opacityRail\\drawing").unwrap()), Some((120, 20)));
    assert_eq!(saved.value("ui", "look"), Some("paper"));

    let before = std::fs::read(&path).unwrap();
    let idle = run(&mut app, &ctx, vec![], false);
    let (host, card) = (painted(&idle, mark()), painted(&idle, card_fill()));
    let at = card.min + Vec2::new(8.0, 8.0);
    run(&mut app, &ctx, vec![Event::PointerMoved(at), press(at, true)], false);
    run(&mut app, &ctx, vec![press(at, false)], false);
    assert_eq!(std::fs::read(&path).unwrap(), before, "a click that did not move was saved");
    near(offset(card, host), Vec2::new(40.0, 90.0));
}
