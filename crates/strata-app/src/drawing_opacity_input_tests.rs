//! Drawing opacity card. C++ `KaDrawingStudio.cpp:1196-1234` and `:2563-2575`.
//! The fixed hint rectangle is not a card. The slider uses the layer the sheet remembered
//! and must not pan the sheet map.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Color32, Event, PointerButton, Pos2, Rect, Shape, Vec2};

use crate::app_state::picture_layers::PictureLayer;
use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use strata_types::schema::{DomainLayerKey, LegendGroup};

const A: &str = "현장사진";
const B: &str = "항공사진 1970";

fn app() -> (StrataAppState, egui::Context) {
    let mut app = StrataAppState::new_startup();
    app.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
    add_picture(&mut app, A, "picture:field");
    add_picture(&mut app, B, "picture:1970");
    app.home_map.viewport.center_xy = [111_111.0, 222_222.0];
    app.drawing.sheet_center = Some([250_000.0, 400_000.0]);
    app.drawing.sheet.scale_denominator = 5_000;
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    (app, ctx)
}

fn add_picture(app: &mut StrataAppState, title: &str, key: &str) {
    app.pictures.layers.push(PictureLayer::fixture(key));
    app.left_layers.entries.insert(
        0,
        LayerTreeEntry {
            layer_key: key.to_owned(),
            label_ko: title.to_owned(),
            group: LegendGroup::ReferenceMap,
            visible: true,
            opacity: 1.0,
            read_only: true,
            snap_enabled: false,
            feature_count: 0,
        },
    );
}

fn paint(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) -> egui::FullOutput {
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
        events,
        ..Default::default()
    };
    ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
    })
}

fn card_and_host(output: &egui::FullOutput) -> (Rect, Rect) {
    let card = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r) if r.fill == Color32::from_rgba_unmultiplied(252, 251, 247, 248) => Some(r.rect),
            _ => None,
        })
        .expect("drawing opacity card");
    let host = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r) if r.fill == Color32::from_rgb(245, 243, 236) => Some(r.rect),
            _ => None,
        })
        .expect("drawing desk");
    (host, card)
}

fn opacity_pos(card: Rect, t: f32) -> Pos2 {
    let (x0, x1) = (card.left() + 58.0, card.right() - 48.0);
    Pos2::new(x0 + (x1 - x0) * t, card.top() + 40.0)
}

fn brightness_pos(card: Rect, t: f32) -> Pos2 {
    let (x0, x1) = (card.left() + 58.0, card.right() - 48.0);
    Pos2::new(x0 + (x1 - x0) * t, card.top() + 64.0)
}

fn press(at: Pos2, down: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: down,
        modifiers: Default::default(),
    }
}

fn row(app: &StrataAppState, title: &str) -> f32 {
    app.left_layers.entries.iter().find(|e| e.label_ko == title).unwrap().opacity
}

fn picture<'a>(app: &'a StrataAppState, key: &str) -> &'a PictureLayer {
    app.pictures.layers.iter().find(|layer| layer.key == key).unwrap()
}

fn map_untouched(app: &StrataAppState, opacity: f32, bright: f32, basemap_opacity: f32, basemap_bright: f32) {
    assert_eq!(app.home_map.viewport.center_xy, [111_111.0, 222_222.0], "drawing card moved the map");
    assert!((app.home_map.real_survey.layer_opacity - opacity).abs() < 1e-6, "drawing card wrote the shared opacity");
    assert!(
        (app.home_map.real_survey.layer_brightness - bright).abs() < 1e-6,
        "drawing card wrote the shared brightness"
    );
    assert!((app.home_map.basemap_opacity - basemap_opacity).abs() < 1e-6, "drawing card wrote the background opacity");
    assert!(
        (app.home_map.basemap_brightness - basemap_bright).abs() < 1e-6,
        "drawing card wrote the background brightness"
    );
}

/// The card changes the remembered picture only, and drops that picture's cache.
#[test]
fn drawing_slider_changes_the_remembered_picture_and_redraws_its_cache() {
    let (mut app, ctx) = app();
    let (opacity, bright, basemap_opacity, basemap_bright) = (
        app.home_map.real_survey.layer_opacity,
        app.home_map.real_survey.layer_brightness,
        app.home_map.basemap_opacity,
        app.home_map.basemap_brightness,
    );
    app.right_inspector.selected_layer_key = Some(A.to_owned());
    let (_, card) = card_and_host(&paint(&mut app, &ctx, vec![]));
    let at = opacity_pos(card, 0.25);
    paint(&mut app, &ctx, vec![Event::PointerMoved(at), press(at, true)]);
    paint(&mut app, &ctx, vec![press(at, false)]);
    assert!((row(&app, A) - 0.25).abs() < 0.02, "picture A opacity {}", row(&app, A));
    assert!((app.rail_opacity(A) - row(&app, A)).abs() < 1e-6);
    assert_eq!(row(&app, B), 1.0);
    assert_eq!(row(&app, "조사구역"), 1.0);
    assert!(picture(&app, "picture:field").cache_epoch >= 1, "picture cache was not dropped");
    assert_eq!(picture(&app, "picture:1970").cache_epoch, 0);
    assert!(app.drawing.map_cache_epoch >= 1, "layout cache was not dropped");
    assert_eq!(app.drawing.sheet_center, Some([250_000.0, 400_000.0]));
    map_untouched(&app, opacity, bright, basemap_opacity, basemap_bright);

    let epoch = picture(&app, "picture:field").cache_epoch;
    let (_, card) = card_and_host(&paint(&mut app, &ctx, vec![]));
    let from = brightness_pos(card, 0.5);
    let to = brightness_pos(card, 0.8);
    paint(&mut app, &ctx, vec![Event::PointerMoved(from), press(from, true)]);
    paint(&mut app, &ctx, vec![Event::PointerMoved(to)]);
    paint(&mut app, &ctx, vec![press(to, false)]);
    assert_eq!(picture(&app, "picture:field").brightness, 153, "0.8 on −255..255 is 153");
    assert!(picture(&app, "picture:field").cache_epoch > epoch, "brightness did not redraw the cache");
    assert_eq!(picture(&app, "picture:1970").brightness, 0);
    map_untouched(&app, opacity, bright, basemap_opacity, basemap_bright);
}

/// A selection change during the drag still applies to the layer the card remembered.
#[test]
fn a_selection_change_during_the_drag_keeps_the_remembered_layer() {
    let (mut app, ctx) = app();
    app.right_inspector.selected_layer_key = Some(A.to_owned());
    let (_, card) = card_and_host(&paint(&mut app, &ctx, vec![]));
    let from = opacity_pos(card, 0.2);
    paint(&mut app, &ctx, vec![Event::PointerMoved(from), press(from, true)]);
    app.right_inspector.selected_layer_key = Some(B.to_owned());
    let to = opacity_pos(card, 0.7);
    paint(&mut app, &ctx, vec![Event::PointerMoved(to)]);
    paint(&mut app, &ctx, vec![press(to, false)]);
    assert!((row(&app, A) - 0.7).abs() < 0.02, "remembered picture {}", row(&app, A));
    assert_eq!(row(&app, B), 1.0, "the layer selected during the drag was changed");

    paint(&mut app, &ctx, vec![]);
    let (_, card) = card_and_host(&paint(&mut app, &ctx, vec![]));
    let at = opacity_pos(card, 0.4);
    paint(&mut app, &ctx, vec![Event::PointerMoved(at), press(at, true), press(at, false)]);
    assert!((row(&app, B) - 0.4).abs() < 0.02, "after the gesture the card follows the selection");
    assert!((row(&app, A) - 0.7).abs() < 0.02);
}

/// Blank drag and a slider drag do not pan the sheet. A drag on the map frame still does.
#[test]
fn card_drag_and_slider_click_do_not_pan_the_sheet() {
    let (mut app, ctx) = app();
    app.drawing.map_adjust_active = true;
    app.right_inspector.selected_layer_key = Some(A.to_owned());
    let (host, card) = card_and_host(&paint(&mut app, &ctx, vec![]));
    let from = card.min + Vec2::new(14.0, 12.0);
    let by = Vec2::new(90.0, -40.0);
    paint(&mut app, &ctx, vec![Event::PointerMoved(from), press(from, true)]);
    paint(&mut app, &ctx, vec![Event::PointerMoved(from + by)]);
    paint(&mut app, &ctx, vec![press(from + by, false)]);
    let (_, moved) = card_and_host(&paint(&mut app, &ctx, vec![]));
    assert!((moved.min - card.min - by).length() <= 0.5, "card did not follow the blank drag");
    assert_eq!(app.drawing.sheet_center, Some([250_000.0, 400_000.0]), "blank drag panned the sheet");
    assert_eq!(app.home_map.viewport.center_xy, [111_111.0, 222_222.0]);

    let from = opacity_pos(moved, 0.3);
    let to = opacity_pos(moved, 0.55);
    paint(&mut app, &ctx, vec![Event::PointerMoved(from), press(from, true)]);
    paint(&mut app, &ctx, vec![Event::PointerMoved(to)]);
    paint(&mut app, &ctx, vec![press(to, false)]);
    assert!((row(&app, A) - 0.55).abs() < 0.02, "slider did not set opacity");
    assert_eq!(app.drawing.sheet_center, Some([250_000.0, 400_000.0]), "slider drag panned the sheet");

    let map = app.drawing.paper_on_screen.expect("map frame")[1];
    let items: Vec<Rect> = app.drawing.item_rects.iter().map(|(_, placed, off)| placed.translate(*off)).collect();
    let from = (0..8)
        .flat_map(|row| (0..8).map(move |col| Pos2::new(map.left() + map.width() * (col as f32 + 0.5) / 8.0, map.top() + map.height() * (row as f32 + 0.5) / 8.0)))
        .find(|at| map.contains(*at) && !moved.contains(*at) && items.iter().all(|item| !item.expand(12.0).contains(*at)))
        .expect("a free point on the map frame");
    let to = from + Vec2::new(36.0, -18.0);
    paint(&mut app, &ctx, vec![Event::PointerMoved(from), press(from, true)]);
    paint(&mut app, &ctx, vec![Event::PointerMoved(to)]);
    paint(&mut app, &ctx, vec![press(to, false)]);
    assert_ne!(app.drawing.sheet_center, Some([250_000.0, 400_000.0]), "map-frame drag should still pan");
    assert_eq!(app.home_map.viewport.center_xy, [111_111.0, 222_222.0], "sheet pan moved the map");
    assert!(host.contains_rect(moved));
}

/// With no layer chosen the card is the lower-left hint, not the old fixed rectangle.
#[test]
fn the_idle_card_sits_at_the_lower_left() {
    let (mut app, ctx) = app();
    app.right_inspector.selected_layer_key = None;
    let output = paint(&mut app, &ctx, vec![]);
    let (host, card) = card_and_host(&output);
    let toolbar = if host.height() < 460.0 { 34.0 } else { 42.0 };
    let gap = toolbar + 20.0;
    assert!((card.left() - host.left() - 12.0).abs() <= 0.5, "left {}", card.left() - host.left());
    assert!((host.bottom() - card.bottom() - gap).abs() <= 0.5, "bottom gap {}", host.bottom() - card.bottom());
    assert!(card.bottom() <= host.bottom() - toolbar - 4.0, "card covers the toolbar");
    let old_hint = output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, Shape::Rect(r) if (r.rect.width() - 196.0).abs() < 1.0 && (r.rect.height() - 30.0).abs() < 1.0));
    assert!(!old_hint, "the fixed hint rectangle is still painted");
    let text = output.shapes.iter().find_map(|s| match &s.shape {
        Shape::Text(t) if t.galley.text() == "투명도·밝기: 조절할 레이어를 고르세요" => Some(t.pos),
        _ => None,
    });
    assert!(text.is_some_and(|at| card.contains(at)), "hint is not on the card");
}
