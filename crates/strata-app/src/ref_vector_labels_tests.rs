//! The labels of reference layers on the map: asked of the map engine when the view or the layers change,
//! written in the app's font where the engine placed them (`ref_vector_labels.rs`; C++
//! `target/cpp-ref/src/core/GeologyMapService.cpp:426-455`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui::{epaint::Shape, pos2, vec2, Pos2, RawInput};
use strata_engine::{write_new_table, Cell, ColumnKind, Row, ShapeKind, Shapes, TableLayout};

use super::*;
use crate::ref_vector::data::{Look, PolygonFill};

const SYMBOLS: LabelLook = LabelLook { column: "기호", size_pt: 8.0, ink: [40, 40, 40], halo_mm: 0.6, halo: [255, 255, 255, 220] };

/// Two units of 400 m in EPSG:5187: 「Qa」 around (219 800, 370 000) and 「Kqp」 around (220 400, 370 000).
fn units(dir: &Path) -> PathBuf {
    let path = dir.join("지질도_KIGAM.gpkg");
    let layout = TableLayout { table: "geology_map", epsg: 5187, kind: ShapeKind::Polygons, columns: &[("기호", ColumnKind::Text)] };
    let unit = |x: f64, symbol: &str| Row { shapes: Shapes::Polygons(vec![vec![vec![[x, 369_800.0], [x + 400.0, 369_800.0], [x + 400.0, 370_200.0], [x, 370_200.0], [x, 369_800.0]]]]), cells: vec![Cell::Text(symbol.to_owned())] };
    write_new_table(&path, &layout, &mut vec![unit(219_600.0, "Qa"), unit(220_200.0, "Kqp")].into_iter()).unwrap();
    path
}

fn spec(file: PathBuf) -> LoadSpec {
    let look = Look::Polygons { stroke: [80, 80, 80, 130], width_mm: 0.12, fill: PolygonFill::None };
    LoadSpec { file, table: "geology_map".to_owned(), work_epsg: 5187, look, label: Some(SYMBOLS) }
}

/// A map of 900 x 600 points whose top-left corner is at (10, 20), looking at (220 300, 370 000) with
/// 2 m to the point: its middle is the screen point (460, 320).
fn map() -> (Rect, ViewportTransform) {
    (Rect::from_min_size(pos2(10.0, 20.0), vec2(900.0, 600.0)), ViewportTransform::new([220_300.0, 370_000.0], 2.0, [900.0, 600.0]))
}

/// Work that runs inside the call that asks for it, counted.
fn counted() -> (Runner, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&count);
    let runner: Runner = Arc::new(move |work| {
        seen.fetch_add(1, Ordering::SeqCst);
        work();
    });
    (runner, count)
}

/// One frame; returns where letters of this colour were written (the top-left corner of each text).
fn frame(labels: &mut Labels, ctx: &egui::Context, vp: &ViewportTransform, sources: Vec<LabelSource>, runner: &Runner, colour: Color32) -> Vec<Pos2> {
    let (rect, _) = map();
    let out = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))), ..RawInput::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| labels.paint(ctx, &ui.painter_at(rect), rect, vp, 5187, sources.clone(), runner));
    });
    let text = |clipped: &egui::epaint::ClippedShape| match &clipped.shape {
        Shape::Text(text) if text.override_text_color == Some(colour) => Some(text.pos),
        _ => None,
    };
    out.shapes.iter().filter_map(text).collect()
}

/// The symbol of each unit is written in the middle of the unit, in the C++ look: 8 pt of Qt (10.67 points
/// here) in 40/40/40, with a white edge of 0.6 mm at alpha 220 written 24 times around the letters.
#[test]
fn labels_are_written_where_the_engine_placed_them_in_the_cpp_look() {
    let dir = tempfile::tempdir().unwrap();
    let source = source("ref_geology", &spec(units(dir.path())), &SYMBOLS);
    assert_eq!((source.look.column.as_str(), source.look.ink, source.look.bold), ("기호", [40, 40, 40, 255], false));
    assert!((source.look.size_pt - 8.0 * 96.0 / 72.0).abs() < 0.001);
    let (edge, width) = source.look.halo.unwrap();
    assert!(edge == [255, 255, 255, 220] && (width - 0.6 * 96.0 / 25.4).abs() < 0.001);

    let (mut labels, ctx, (_, vp), (runner, asked)) = (Labels::default(), egui::Context::default(), map(), counted());
    let ink = Color32::from_rgb(40, 40, 40);
    let mut written = frame(&mut labels, &ctx, &vp, vec![source.clone()], &runner, ink);
    written.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert_eq!(written.len(), 2);
    // The units' middles are 250 points left and 50 points right of the map's middle (460, 320); a text
    // starts half its size up and to the left of its middle.
    for (corner, middle) in written.iter().zip([pos2(210.0, 320.0), pos2(510.0, 320.0)]) {
        assert!(corner.x < middle.x && middle.x - corner.x < 20.0 && corner.y < middle.y && middle.y - corner.y < 12.0, "{corner:?} for {middle:?}");
    }
    let edge = frame(&mut labels, &ctx, &vp, vec![source.clone()], &runner, Color32::from_rgba_premultiplied(220, 220, 220, 220));
    assert_eq!(edge.len(), 48);
    let furthest = edge.iter().map(|at| (*at - written[0]).length().min((*at - written[1]).length())).fold(0.0, f32::max);
    assert!((furthest - 0.6 * 96.0 / 25.4).abs() < 0.01, "{furthest}");
    assert_eq!(asked.load(Ordering::SeqCst), 1, "a still view is asked once");
}

/// The engine is asked again when the view or the layers change, and only then. Without labelled layers
/// nothing is asked and nothing stays written.
#[test]
fn the_engine_is_asked_when_the_view_or_the_layers_change() {
    let dir = tempfile::tempdir().unwrap();
    let source = source("ref_geology", &spec(units(dir.path())), &SYMBOLS);
    let (mut labels, ctx, (_, mut vp), (runner, asked)) = (Labels::default(), egui::Context::default(), map(), counted());
    let ink = Color32::from_rgb(40, 40, 40);
    assert!(frame(&mut labels, &ctx, &vp, Vec::new(), &runner, ink).is_empty());
    assert_eq!(asked.load(Ordering::SeqCst), 0);

    assert_eq!(frame(&mut labels, &ctx, &vp, vec![source.clone()], &runner, ink).len(), 2);
    assert_eq!(frame(&mut labels, &ctx, &vp, vec![source.clone()], &runner, ink).len(), 2);
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    // The map is moved 400 m east: the west unit's 200 m left in view carry its symbol.
    vp.center_xy = [220_700.0, 370_000.0];
    let moved = frame(&mut labels, &ctx, &vp, vec![source.clone()], &runner, ink);
    assert_eq!((moved.len(), asked.load(Ordering::SeqCst)), (2, 2));

    assert!(frame(&mut labels, &ctx, &vp, Vec::new(), &runner, ink).is_empty(), "the row was unchecked");
    assert!(!labels.is_busy());
    assert_eq!(frame(&mut labels, &ctx, &vp, vec![source], &runner, ink).len(), 2, "and checked again");
    assert_eq!(asked.load(Ordering::SeqCst), 3);
}
