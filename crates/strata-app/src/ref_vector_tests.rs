//! Reference vector layers on the map: drawn only while their legend row is there and checked, as one
//! picture that is made again only when the view leaves it (owner 2026-11-15: 「목록에 줄이 없는 것은 그리지 않는다」).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, epaint::Shape, vec2, Color32, Pos2, RawInput, Rect, TextureId};
use strata_engine::{write_new_table, Cell, ColumnKind, Row, ShapeKind, Shapes, TableLayout};
use strata_types::schema::LegendGroup;

use super::*;
use crate::ref_vector_data::Look;

const KEY: &str = "ref_topographic";
const CENTRE: [f64; 2] = [220_000.0, 370_000.0];

fn topo_file(dir: &Path) -> PathBuf {
    let path = dir.join("수치지형도-합침-1.gpkg");
    let layout = TableLayout { table: "topo", epsg: 5187, kind: ShapeKind::Lines, columns: &[("sheet", ColumnKind::Text)] };
    let line = |x: f64| Row { shapes: Shapes::Lines(vec![vec![[x, 369_900.0], [x + 200.0, 370_100.0]]]), cells: vec![Cell::Text("359012".to_owned())] };
    write_new_table(&path, &layout, &mut vec![line(219_900.0), line(219_950.0), line(220_000.0)].into_iter()).unwrap();
    path
}

fn spec(file: PathBuf) -> LoadSpec {
    LoadSpec { file, table: "topo".to_owned(), work_epsg: 5187, look: Look::Lines { stroke: [128, 128, 128, 255], width_mm: 0.2 } }
}

/// A map app of 900 x 600 points looking at `CENTRE` with 2 m to the point; work runs at once.
fn map_app() -> (StrataAppState, Rect) {
    let mut app = StrataAppState::new_startup();
    app.home_map.viewport = strata_canvas2d::viewport::ViewportTransform::new(CENTRE, 2.0, [900.0, 600.0]);
    app.ref_vectors.run_at_once();
    (app, Rect::from_min_size(Pos2::ZERO, vec2(900.0, 600.0)))
}

/// Paints one frame; returns the picture rectangles drawn (screen points) with their tint.
fn frame(app: &mut StrataAppState, ctx: &egui::Context, rect: Rect) -> Vec<(Rect, Color32)> {
    let out = ctx.run(RawInput { screen_rect: Some(rect), ..RawInput::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_reference_vector_layers(ctx, &ui.painter_at(rect), rect));
    });
    let mut pictures = Vec::new();
    for clipped in &out.shapes {
        if let Shape::Mesh(mesh) = &clipped.shape {
            if mesh.texture_id != TextureId::default() && mesh.vertices.len() == 4 {
                let xs = mesh.vertices.iter().map(|v| v.pos.x);
                let ys = mesh.vertices.iter().map(|v| v.pos.y);
                let min = Pos2::new(xs.clone().fold(f32::INFINITY, f32::min), ys.clone().fold(f32::INFINITY, f32::min));
                let max = Pos2::new(xs.fold(f32::NEG_INFINITY, f32::max), ys.fold(f32::NEG_INFINITY, f32::max));
                pictures.push((Rect::from_min_max(min, max), mesh.vertices[0].color));
            }
        }
    }
    pictures
}

#[test]
fn a_shown_layer_gets_its_row_and_one_picture_around_the_view() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));

    let row = app.left_layers.entries.iter().find(|e| e.layer_key == KEY).expect("row");
    assert_eq!((row.label_ko.as_str(), row.group, row.read_only), ("수치지형도", LegendGroup::ReferenceMap, true));
    assert!(app.home_map.real_survey.is_layer_visible("수치지형도"));

    let ctx = egui::Context::default();
    let pictures = frame(&mut app, &ctx, rect);
    assert_eq!(pictures.len(), 1);
    // The picture reaches 256 pixels beyond the map on every side, so that a small pan needs no new one.
    let (drawn, tint) = pictures[0];
    for (got, want) in [(drawn.left(), -256.0), (drawn.top(), -256.0), (drawn.right(), 1156.0), (drawn.bottom(), 856.0)] {
        assert!((got - want).abs() < 0.01, "{drawn:?}");
    }
    assert_eq!(tint, Color32::WHITE);
    assert_eq!(app.ref_vectors.pictures_made(), 1);
    // The row shows how many rows the table has.
    assert_eq!(app.left_layers.entries.iter().find(|e| e.layer_key == KEY).unwrap().feature_count, 3);
}

#[test]
fn the_picture_is_made_again_only_when_the_view_leaves_it_or_changes_scale() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, rect);
    frame(&mut app, &ctx, rect);
    assert_eq!(app.ref_vectors.pictures_made(), 1, "a still view");

    // 100 points (200 m) to the east: still inside the margin. The old picture is drawn shifted.
    app.home_map.viewport.center_xy = [CENTRE[0] + 200.0, CENTRE[1]];
    let pictures = frame(&mut app, &ctx, rect);
    assert_eq!(app.ref_vectors.pictures_made(), 1);
    assert!((pictures[0].0.left() - (-356.0)).abs() < 0.01, "{:?}", pictures[0].0);

    // 300 points further: the view has left the picture.
    app.home_map.viewport.center_xy = [CENTRE[0] + 800.0, CENTRE[1]];
    let pictures = frame(&mut app, &ctx, rect);
    assert_eq!(app.ref_vectors.pictures_made(), 2);
    assert!((pictures[0].0.left() - (-256.0)).abs() < 0.01, "the new picture, {:?}", pictures[0].0);

    // Another scale: a new picture even though the old one covers the view.
    app.home_map.viewport.meters_per_pixel = 1.0;
    frame(&mut app, &ctx, rect);
    assert_eq!(app.ref_vectors.pictures_made(), 3);
}

#[test]
fn an_unchecked_row_is_not_drawn_and_a_removed_row_lets_the_layer_go() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    let ctx = egui::Context::default();
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);

    app.home_map.real_survey.layer_visible.insert("수치지형도".to_owned(), false);
    assert!(frame(&mut app, &ctx, rect).is_empty());
    assert!(!app.ref_vectors.holds_picture(KEY), "the picture of a hidden layer is not kept");
    assert!(app.ref_vectors.is_loaded(KEY), "its shapes are, for the next check mark");

    app.home_map.real_survey.layer_visible.insert("수치지형도".to_owned(), true);
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);

    // The user removes the row from the legend: nothing is drawn, nothing is kept in memory.
    let row = app.left_layers.entries.iter().position(|e| e.layer_key == KEY).unwrap();
    let removed = app.left_layers.entries.remove(row);
    assert!(frame(&mut app, &ctx, rect).is_empty());
    assert!(!app.ref_vectors.is_loaded(KEY) && !app.ref_vectors.holds_picture(KEY));
    // Undo puts the row back: the layer is read from its file again.
    app.left_layers.entries.insert(row, removed);
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);
}

#[test]
fn the_opacity_of_the_row_fades_the_picture() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    app.left_layers.entries.iter_mut().find(|e| e.layer_key == KEY).unwrap().opacity = 0.4;
    let ctx = egui::Context::default();
    let pictures = frame(&mut app, &ctx, rect);
    assert_eq!(pictures[0].1, Color32::from_white_alpha(102));
    assert_eq!(app.rail_opacity("수치지형도"), 0.4, "the 투명도 rail shows the row's own value");
}

#[test]
fn asking_again_replaces_the_layer_and_keeps_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, rect);
    app.left_layers.entries.iter_mut().find(|e| e.layer_key == KEY).unwrap().opacity = 0.5;
    app.home_map.real_survey.layer_visible.insert("수치지형도".to_owned(), false);

    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(other.path())));
    assert_eq!(app.left_layers.entries.iter().filter(|e| e.layer_key == KEY).count(), 1);
    assert!(app.home_map.real_survey.is_layer_visible("수치지형도"), "asked for again: checked again");
    assert_eq!(app.left_layers.entries.iter().find(|e| e.layer_key == KEY).unwrap().opacity, 0.5, "the row keeps what the user set");
    assert!(!app.ref_vectors.is_loaded(KEY), "the shapes of the old file are let go");
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);
    assert_eq!(app.ref_vectors.pictures_made(), 2);
}

#[test]
fn rows_of_an_earlier_survey_are_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    app.remove_reference_vector_rows();
    assert!(app.left_layers.entries.iter().all(|e| e.layer_key != KEY));
    assert!(!app.ref_vectors.has(KEY));
    assert!(frame(&mut app, &egui::Context::default(), rect).is_empty());
}

#[test]
fn a_file_that_cannot_be_read_draws_nothing_and_says_so_once() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(dir.path().join("gone.gpkg")));
    let ctx = egui::Context::default();
    assert!(frame(&mut app, &ctx, rect).is_empty());
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(OPEN_FAILED));
    app.home_map.draw_guide_message = None;
    assert!(frame(&mut app, &ctx, rect).is_empty());
    assert_eq!(app.home_map.draw_guide_message, None, "not again on every frame");
    assert_eq!(app.ref_vectors.pictures_made(), 0);
}

#[test]
fn a_copy_of_the_app_state_knows_the_layers_but_holds_none_of_their_memory() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    frame(&mut app, &egui::Context::default(), rect);
    let copy = app.ref_vectors.clone();
    assert!(copy.has(KEY) && !copy.is_loaded(KEY) && !copy.holds_picture(KEY));
    assert!(Arc::strong_count(&app.ref_vectors.shapes_of(KEY).unwrap()) >= 2, "the original still holds its shapes");
}
