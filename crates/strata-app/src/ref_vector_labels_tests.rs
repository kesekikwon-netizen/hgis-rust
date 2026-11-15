//! Which labels of a reference layer are asked for in a view of the map, where their letters go, and
//! which of them are written (`ref_vector_labels.rs`; C++ `target/cpp-ref/src/core/GeologyMapService.cpp:426-455`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, pos2, vec2, Color32, RawInput, Rect, Vec2};
use strata_canvas2d::viewport::ViewportTransform;

use super::*;
use crate::ref_vector::data::{Item, PaintOf, Run};

const SYMBOLS: LabelLook = LabelLook { column: "기호", size_pt: 8.0, ink: [40, 40, 40], halo_mm: 0.6, halo: [255, 255, 255, 220] };

/// A closed square ring around a centre (metres from the layer's origin).
fn square(x: f32, y: f32, side: f32) -> Vec<[f32; 2]> {
    let h = side / 2.0;
    vec![[x - h, y - h], [x + h, y - h], [x + h, y + h], [x - h, y + h], [x - h, y - h]]
}

/// A polygon: its rings, the outer one first.
type Polygon = Vec<Vec<[f32; 2]>>;

/// Rows: the text of each and its polygons.
fn layer(rows: &[(&str, Vec<Polygon>)]) -> RefVectorData {
    let paint = PaintOf { fill: [0; 4], stroke: [0, 0, 0, 255], width_mm: 0.1 };
    let mut data = RefVectorData { origin: [220_000.0, 370_000.0], bbox: [0.0; 4], coords: Vec::new(), runs: Vec::new(), items: Vec::new(), paints: vec![paint], closed: true, rows: rows.len() as u64, labels: Vec::new() };
    for (text, polygons) in rows {
        let first_item = u32::try_from(data.items.len()).unwrap();
        for rings in polygons {
            let first_run = u32::try_from(data.runs.len()).unwrap();
            for ring in rings {
                data.runs.push(Run { start: u32::try_from(data.coords.len()).unwrap(), len: u32::try_from(ring.len()).unwrap() });
                data.coords.extend(ring);
            }
            let outer = &rings[0];
            let (xs, ys) = (outer.iter().map(|p| p[0]), outer.iter().map(|p| p[1]));
            let bbox = [xs.clone().fold(f32::INFINITY, f32::min), ys.clone().fold(f32::INFINITY, f32::min), xs.fold(f32::NEG_INFINITY, f32::max), ys.fold(f32::NEG_INFINITY, f32::max)];
            data.items.push(Item { first_run, run_count: u32::try_from(rings.len()).unwrap(), bbox, paint: 0 });
        }
        data.labels.push(Label { text: (*text).to_owned(), first_item, items: u32::try_from(polygons.len()).unwrap() });
    }
    data
}

/// A map of 900 x 600 points whose top-left corner is at (10, 20), looking at the layer's origin with 2 m
/// to the point: it shows 900 m to the east and west of the origin and 600 m to the north and south, and
/// its middle is the screen point (460, 320).
fn map() -> (Rect, ViewportTransform) {
    (Rect::from_min_size(pos2(10.0, 20.0), vec2(900.0, 600.0)), ViewportTransform::new([220_000.0, 370_000.0], 2.0, [900.0, 600.0]))
}

fn asked(data: &RefVectorData) -> Pending {
    let (rect, vp) = map();
    let mut pending = Pending::default();
    let _ = egui::Context::default().run(RawInput::default(), |ctx| pending.add(ctx, data, &SYMBOLS, rect, &vp));
    pending
}

/// The size of the letters 「Kqp」 on the screen.
fn letters() -> Vec2 {
    asked(&layer(&[("Kqp", vec![vec![square(0.0, 0.0, 400.0)]])])).draws[0].galley.rect.size()
}

/// C++ places a unit's symbol over the unit (`:432`, OverPoint): the letters are centred on the centroid.
/// A row with nothing in view asks for no label.
#[test]
fn a_label_gets_a_box_centred_on_the_centroid_of_its_polygon() {
    // The second polygon is a 100 m square with a 40 m hole: (10000 * 0 - 1600 * -20) / 8400 = 3.81 m to
    // the north-east of its middle, both ways.
    let holed = vec![square(300.0, 200.0, 100.0), square(280.0, 180.0, 40.0)];
    let data = layer(&[("Kqp", vec![vec![square(0.0, 0.0, 400.0)]]), ("Qa", vec![holed]), ("Off", vec![vec![square(5000.0, 0.0, 400.0)]])]);
    let pending = asked(&data);

    assert_eq!((pending.reqs.len(), pending.draws.len()), (2, 2));
    let (big, holed) = (&pending.reqs[0], &pending.reqs[1]);
    let size = letters();
    assert!(size.x > 8.0 && size.y > 8.0, "{size:?}");
    assert!((big.candidates[0].center() - pos2(460.0, 320.0)).length() < 0.01, "{:?}", big.candidates);
    assert!((big.candidates[0].size() - size).length() < 0.01);
    // 300 m east and 200 m north are 150 points right and 100 up; the hole pushes 1.9 points further.
    assert!((holed.candidates[0].center() - pos2(611.905, 218.095)).length() < 0.01, "{:?}", holed.candidates);
    assert_eq!((big.candidates.len(), holed.candidates.len()), (1, 1));
    // The area decides which label is written first: what is in view, holes taken out.
    assert!((big.area - 160_000.0).abs() < 0.5 && (holed.area - 8400.0).abs() < 0.5, "{} {}", big.area, holed.area);
    // Qt's 8 pt are 8 * 96 / 72 points here (`:443-445`), in the colour 40/40/40, upright.
    let draw = &pending.draws[0];
    assert!((draw.galley.job.sections[0].format.font_id.size - 8.0 * 96.0 / 72.0).abs() < 0.001);
    assert_eq!(draw.galley.job.sections[0].format.font_id.family, egui::FontFamily::Proportional);
    assert_eq!((draw.galley.text(), draw.ink), ("Kqp", Color32::from_rgb(40, 40, 40)));
    assert!(draw.angle.abs() < f32::EPSILON && draw.origins == vec![big.candidates[0].min]);
}

/// QGIS cuts a shape to the map before it looks for the label's place: a unit larger than the map is
/// labelled over the part in view, and of a row of several polygons the one with most in view.
#[test]
fn a_unit_larger_than_the_map_is_labelled_over_the_part_in_view() {
    // East of x = -500 m the map shows 1400 m of this 4 km square, over its whole height of 1200 m.
    let wide = asked(&layer(&[("Qa", vec![vec![square(1500.0, 0.0, 4000.0)]])]));
    assert!((wide.reqs[0].candidates[0].center() - pos2(560.0, 320.0)).length() < 0.01, "{:?}", wide.reqs[0].candidates);
    assert!((wide.reqs[0].area - 1_680_000.0).abs() < 1.0, "{}", wide.reqs[0].area);
    assert!(wide.reqs[0].fits_inside);

    // A row of two: a 100 m square in full view (10 000 m2) and a 3 km square of which a strip of 200 m
    // is in view (240 000 m2). The strip carries the label: 800 m east of the middle.
    let two = asked(&layer(&[("Kan", vec![vec![square(-400.0, 0.0, 100.0)], vec![square(2200.0, 0.0, 3000.0)]])]));
    assert_eq!(two.reqs.len(), 1);
    assert!((two.reqs[0].candidates[0].center() - pos2(860.0, 320.0)).length() < 0.01, "{:?}", two.reqs[0].candidates);
    // And the other way round when the small one has more in view (a strip of 20 m: 24 000 m2 against 40 000).
    let small = asked(&layer(&[("Kan", vec![vec![square(-400.0, 0.0, 200.0)], vec![square(2380.0, 0.0, 3000.0)]])]));
    assert!((small.reqs[0].candidates[0].center() - pos2(260.0, 320.0)).length() < 0.01, "{:?}", small.reqs[0].candidates);
}

/// A symbol stands on its own unit: where the centroid lies outside the polygon (here a U open to the
/// north, centroid in the gap), the letters go to the widest stretch at that height, the western of equals.
#[test]
fn a_centroid_outside_its_polygon_moves_the_label_into_the_polygon() {
    // Bottom bar 600 x 100 m, two legs 100 x 300 m: the centroid is (0, -50), between the legs.
    let u = vec![[-300.0, -200.0], [300.0, -200.0], [300.0, 200.0], [200.0, 200.0], [200.0, -100.0], [-200.0, -100.0], [-200.0, 200.0], [-300.0, 200.0], [-300.0, -200.0]];
    let pending = asked(&layer(&[("Kbhgr", vec![vec![u]])]));
    // The western leg: 250 m west and 50 m south of the middle are 125 points left and 25 down.
    assert!((pending.reqs[0].candidates[0].center() - pos2(335.0, 345.0)).length() < 0.01, "{:?}", pending.reqs[0].candidates);
    // With a western leg of 100 m and an eastern one of 200 m the centroid (87.5, -37.5) is in the gap
    // again; the eastern leg is the wider stretch: 300 m east and 37.5 m south of the middle.
    let lopsided = vec![[-300.0, -200.0], [400.0, -200.0], [400.0, 200.0], [200.0, 200.0], [200.0, -100.0], [-200.0, -100.0], [-200.0, 200.0], [-300.0, 200.0], [-300.0, -200.0]];
    let pending = asked(&layer(&[("Kbhgr", vec![vec![lopsided]])]));
    assert!((pending.reqs[0].candidates[0].center() - pos2(610.0, 338.75)).length() < 0.01, "{:?}", pending.reqs[0].candidates);
    // A point in a hole is outside as well: a 400 m square with a 200 m hole in its middle.
    let ring = asked(&layer(&[("Qa", vec![vec![square(0.0, 0.0, 400.0), square(0.0, 0.0, 200.0)]])]));
    // The western side of the ring: 150 m west of the middle.
    assert!((ring.reqs[0].candidates[0].center() - pos2(385.0, 320.0)).length() < 0.01, "{:?}", ring.reqs[0].candidates);
}

/// QGIS does not write a label the edge of the map would cut.
#[test]
fn a_label_the_edge_of_the_map_would_cut_is_left_out() {
    // 10 m of this square are in view at the east edge: the letters would reach beyond the map.
    assert!(asked(&layer(&[("Kqp", vec![vec![square(1890.0, 0.0, 2000.0)]])])).reqs.is_empty());
    // With 100 m in view the letters have room.
    assert_eq!(asked(&layer(&[("Kqp", vec![vec![square(1800.0, 0.0, 2000.0)]])])).reqs.len(), 1);
}

/// Letters that reach over the edge of their polygon do not count as fitting, although part of them is
/// inside: such a label gives way to one that fits.
#[test]
fn letters_that_reach_over_the_edge_of_their_polygon_do_not_fit() {
    // Half the letters, in metres (2 m to the point).
    let (w, h) = (letters().x, letters().y);
    // A right triangle with legs between 3 w and 3 (w + h): its centroid is a third along each leg, so
    // three corners of the letters are inside and the one towards the long side is not.
    let leg = 3.0 * w + 1.5 * h;
    let triangle = vec![[0.0, 0.0], [leg, 0.0], [0.0, leg], [0.0, 0.0]];
    assert!(!asked(&layer(&[("Kqp", vec![vec![triangle]])])).reqs[0].fits_inside);
    let roomy = vec![[0.0, 0.0], [6.0 * (w + h), 0.0], [0.0, 6.0 * (w + h)], [0.0, 0.0]];
    assert!(asked(&layer(&[("Kqp", vec![vec![roomy]])])).reqs[0].fits_inside);
}

/// Where the letters of this colour were written.
fn written(out: &egui::FullOutput, colour: Color32) -> Vec<egui::Pos2> {
    let text = |clipped: &egui::epaint::ClippedShape| match &clipped.shape {
        Shape::Text(text) if text.override_text_color == Some(colour) => Some(text.pos),
        _ => None,
    };
    out.shapes.iter().filter_map(text).collect()
}

/// The labels of all layers are chosen together, as QGIS does for a map: one that would cover a label
/// already placed is left out (the larger part first). Each layer keeps its own white edge (`:446-449`).
#[test]
fn labels_of_two_layers_do_not_cover_each_other() {
    let lower = layer(&[("Kqp", vec![vec![square(0.0, 0.0, 400.0)]])]);
    let upper = layer(&[("Soil", vec![vec![square(2.0, 0.0, 300.0)]]), ("Far", vec![vec![square(600.0, 0.0, 300.0)]])]);
    let other = LabelLook { ink: [200, 0, 0], halo_mm: 1.0, halo: [0, 0, 0, 255], ..SYMBOLS };
    let (rect, vp) = map();
    let out = egui::Context::default().run(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))), ..RawInput::default() }, |ctx| {
        let mut pending = Pending::default();
        pending.add(ctx, &lower, &SYMBOLS, rect, &vp);
        pending.add(ctx, &upper, &other, rect, &vp);
        egui::CentralPanel::default().show(ctx, |ui| pending.paint(&ui.painter_at(rect)));
    });
    let (kqp, far) = (written(&out, Color32::from_rgb(40, 40, 40)), written(&out, Color32::from_rgb(200, 0, 0)));
    assert_eq!((kqp.len(), far.len()), (1, 1), "Kqp, and Far only: Soil would cover Kqp");
    // The white edge is the letters written 24 times around their place, at most its width away: 0.6 mm
    // at alpha 220 for the first layer (Qt's premultiplied 220/220/220/220), 1 mm of black for the second.
    for (edge, letters, width_mm) in [(written(&out, Color32::from_rgba_premultiplied(220, 220, 220, 220)), kqp[0], 0.6), (written(&out, Color32::BLACK), far[0], 1.0)] {
        assert_eq!(edge.len(), 24);
        let furthest = edge.iter().map(|at| (*at - letters).length()).fold(0.0, f32::max);
        assert!((furthest - width_mm * 96.0 / 25.4).abs() < 0.01, "{furthest}");
    }
}
