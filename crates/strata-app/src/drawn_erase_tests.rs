//! Finishing a sketch over another shape of the same layer asks 「겹친 곳 지우기 / 그대로 두기」.
//! `target/cpp-ref/src/app/MainWindowErase.cpp:120-147`, `MainWindowEditing.cpp:842-849`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use eframe::egui::{self, epaint::Shape, Event, Pos2, RawInput, Rect};
use strata_canvas2d::map_tools::MapToolKind;

use super::DrawnErase;
use crate::app_state::{MainTab, StrataAppState};

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    app
}

fn rect(app: &mut StrataAppState, table: &str, [x0, y0, x1, y1]: [f64; 4]) -> i64 {
    let (x0, y0, x1, y1) = (
        200_000.0 + x0,
        450_000.0 + y0,
        200_000.0 + x1,
        450_000.0 + y1,
    );
    let wkt = format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))");
    let key = strata_types::schema::DomainLayerKey::from_id(table).unwrap();
    app.left_layers.ensure_domain_layer(key);
    let t = app
        .survey_tables
        .iter_mut()
        .find(|t| t.table == table)
        .unwrap();
    t.layer.add_unsaved_feature(wkt, BTreeMap::new())
}

fn count(app: &StrataAppState, table: &str) -> usize {
    app.survey_tables
        .iter()
        .find(|t| t.table == table)
        .map_or(0, |t| t.layer.features.len())
}

fn sketch_triangle(app: &mut StrataAppState, table: &str, pts: [[f64; 2]; 3]) {
    match table {
        "feature_poly" => app.start_edit_feature_poly(),
        _ => app.start_edit_survey_area(),
    }
    app.active_tool = MapToolKind::DrawPolygon;
    for p in pts {
        app.sketch_press([200_000.0 + p[0], 450_000.0 + p[1]]);
    }
    app.sketch_finish();
}

#[test]
fn a_same_layer_overlap_asks_and_keep_leaves_both() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let _outer = rect(&mut app, "feature_poly", [0.0, 0.0, 100.0, 100.0]);
    sketch_triangle(
        &mut app,
        "feature_poly",
        [[40.0, 40.0], [60.0, 40.0], [50.0, 60.0]],
    );
    assert!(app.home_map.drawn_erase.is_some(), "the menu is up");
    assert!(app.home_map.feature_form.is_none(), "the name form waits");
    app.answer_drawn_erase(false);
    assert!(app.home_map.drawn_erase.is_none());
    assert_eq!(count(&app, "feature_poly"), 2);
}

#[test]
fn erase_uses_the_same_status_sentence_as_the_button() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let outer = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0]);
    let _far = rect(&mut app, "survey_area", [300.0, 0.0, 400.0, 100.0]);
    let inner = rect(&mut app, "survey_area", [40.0, 40.0, 60.0, 60.0]);
    assert!(app.queue_drawn_erase("survey_area", inner, None));
    app.answer_drawn_erase(true);
    assert!(app
        .survey_tables
        .iter()
        .find(|t| t.table == "survey_area")
        .unwrap()
        .layer
        .features
        .iter()
        .all(|f| f.fid != inner));
    let area = strata_engine::wkt_area(
        &app.survey_tables
            .iter()
            .find(|t| t.table == "survey_area")
            .unwrap()
            .layer
            .features
            .iter()
            .find(|f| f.fid == outer)
            .unwrap()
            .wkt_geom,
    )
    .unwrap()
    .abs();
    assert!((area - 9600.0).abs() < 1e-4);
    assert_eq!(
        app.status_left_message(),
        "「조사구역」 도형 1개에서 겹친 자리를 지웠습니다. 면적 10,000.00 ㎡ → 9,600.00 ㎡. 위에 그린 도형은 없앴습니다. Ctrl+Z로 되돌립니다."
    );
}

#[test]
fn a_sketch_over_a_different_layer_does_not_ask() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let _zone = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0]);
    sketch_triangle(
        &mut app,
        "feature_poly",
        [[40.0, 40.0], [60.0, 40.0], [50.0, 60.0]],
    );
    assert!(app.home_map.drawn_erase.is_none());
    assert_eq!(count(&app, "feature_poly"), 1);
    assert_eq!(count(&app, "survey_area"), 1);
}

fn paint(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    events: Vec<Event>,
) -> Vec<egui::epaint::ClippedShape> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 700.0))),
        predicted_dt: 1.0 / 60.0,
        events,
        ..Default::default()
    };
    ctx.run(raw, |ctx| app.render_drawn_erase(ctx)).shapes
}

fn walk(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    f(shape);
    if let Shape::Vec(v) = shape {
        v.iter().for_each(|s| walk(s, f));
    }
}

fn center(shapes: &[egui::epaint::ClippedShape], needle: &str) -> Option<Pos2> {
    for clipped in shapes {
        let mut hit = None;
        walk(&clipped.shape, &mut |s| {
            if let Shape::Text(t) = s {
                if t.galley.text() == needle {
                    hit = Some(Rect::from_min_size(t.pos, t.galley.size()).center());
                }
            }
        });
        if hit.is_some() {
            return hit;
        }
    }
    None
}

#[test]
fn the_menu_paints_both_choices_and_keep_is_a_click_on_the_label() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let _outer = rect(&mut app, "feature_poly", [0.0, 0.0, 100.0, 100.0]);
    let inner = rect(&mut app, "feature_poly", [40.0, 40.0, 60.0, 60.0]);
    app.home_map.drawn_erase = Some(DrawnErase {
        key: "feature_poly".to_owned(),
        fid: inner,
        notice: None,
        armed: false,
    });
    let ctx = egui::Context::default();
    let _ = paint(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(Pos2::new(200.0, 200.0))],
    );
    let first = paint(&mut app, &ctx, Vec::new());
    assert!(
        center(&first, "겹친 곳 지우기").is_some(),
        "the menu is painted"
    );
    assert!(center(&first, "그대로 두기").is_some());
    let second = paint(&mut app, &ctx, Vec::new());
    let at = center(&second, "그대로 두기").unwrap();
    let _ = paint(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    let _ = paint(
        &mut app,
        &ctx,
        vec![Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let _ = paint(
        &mut app,
        &ctx,
        vec![Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.home_map.drawn_erase.is_none());
    assert_eq!(
        count(&app, "feature_poly"),
        2,
        "그대로 두기 keeps the new shape"
    );
}
