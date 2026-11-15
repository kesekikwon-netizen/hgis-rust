//! The picture of a reference layer, checked pixel by pixel. A picture pixel (x, y) covers the square from
//! (x, y) to (x + 1, y + 1); world y grows upwards, picture y downwards.

use super::*;
use crate::ref_vector::data::{Item, PaintOf, Run};
use crate::ref_vector::stroke::Polylines;

const GREY: [u8; 4] = [128, 128, 128, 255];

/// A layer of these shapes (world metres), each with its paint.
fn layer(shapes: &[(Vec<Vec<[f64; 2]>>, PaintOf)], closed: bool) -> RefVectorData {
    let origin = [1000.0, 2000.0];
    let mut data = RefVectorData { origin, bbox: [0.0; 4], coords: Vec::new(), runs: Vec::new(), items: Vec::new(), paints: Vec::new(), closed, rows: shapes.len() as u64, labels: Vec::new() };
    for (runs, paint) in shapes {
        let first_run = data.runs.len() as u32;
        let mut bbox = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        for run in runs {
            let start = data.coords.len() as u32;
            for p in run {
                let o = [(p[0] - origin[0]) as f32, (p[1] - origin[1]) as f32];
                bbox = [bbox[0].min(o[0]), bbox[1].min(o[1]), bbox[2].max(o[0]), bbox[3].max(o[1])];
                data.coords.push(o);
            }
            data.runs.push(Run { start, len: run.len() as u32 });
        }
        let paint_index = data.paints.iter().position(|p| p == paint).unwrap_or_else(|| {
            data.paints.push(*paint);
            data.paints.len() - 1
        });
        data.items.push(Item { first_run, run_count: runs.len() as u32, bbox, paint: paint_index as u16 });
    }
    data
}

fn line_paint(width_mm: f32) -> PaintOf {
    PaintOf { fill: [0; 4], stroke: GREY, width_mm }
}

fn square(x: f64, y: f64, side: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side], [x, y]]
}

/// 120 x 40 pixels of one metre, top left at world (990, 2020); four pixels to the millimetre.
fn view() -> RasterView {
    RasterView { left: 990.0, top: 2020.0, metres_per_pixel: 1.0, width: 120, height: 40, pixels_per_mm: 4.0 }
}

fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * raster.view.width + x) * 4) as usize;
    [raster.rgba[at], raster.rgba[at + 1], raster.rgba[at + 2], raster.rgba[at + 3]]
}

/// A line 0.5 mm wide (2 pixels) along world y = 2000 from x = 1000 to 1100: picture rows 19 and 20, columns
/// 10 to 109, and nothing beside it.
#[test]
fn a_line_is_drawn_where_it_lies() {
    let data = layer(&[(vec![vec![[1000.0, 2000.0], [1100.0, 2000.0]]], line_paint(0.5))], false);
    let raster = render(&data, view()).expect("picture");
    assert_eq!(raster.rgba.len(), 120 * 40 * 4);
    for x in [10, 60, 109] {
        assert_eq!(pixel(&raster, x, 19), GREY, "column {x}");
        assert_eq!(pixel(&raster, x, 20), GREY, "column {x}");
        assert_eq!(pixel(&raster, x, 17)[3], 0);
        assert_eq!(pixel(&raster, x, 22)[3], 0);
    }
    assert_eq!(pixel(&raster, 8, 20)[3], 0, "before the first point");
    assert_eq!(pixel(&raster, 111, 20)[3], 0, "after the last point");
}

/// A line thinner than a pixel (0.2 mm: 0.8 pixel here) is still there, lighter.
#[test]
fn a_hair_line_is_still_drawn() {
    let data = layer(&[(vec![vec![[1000.0, 2000.5], [1100.0, 2000.5]]], line_paint(0.2))], false);
    let raster = render(&data, view()).expect("picture");
    let on = pixel(&raster, 60, 19)[3];
    assert!((150..255).contains(&on), "alpha {on}");
    assert_eq!(pixel(&raster, 60, 17)[3], 0);
    assert_eq!(pixel(&raster, 60, 21)[3], 0);
}

/// Half the metres to the pixel: the same line is twice as long and lies twice as far from the corner.
#[test]
fn the_picture_follows_the_scale() {
    let data = layer(&[(vec![vec![[1000.0, 2010.0], [1040.0, 2010.0]]], line_paint(0.5))], false);
    let near = RasterView { metres_per_pixel: 0.5, ..view() };
    let raster = render(&data, near).expect("picture");
    // x: (1000 - 990) / 0.5 = 20 to (1040 - 990) / 0.5 = 100; y: (2020 - 2010) / 0.5 = 20.
    assert_eq!(pixel(&raster, 21, 20), GREY);
    assert_eq!(pixel(&raster, 98, 20), GREY);
    assert_eq!(pixel(&raster, 18, 20)[3], 0);
    assert_eq!(pixel(&raster, 102, 20)[3], 0);
}

/// A polygon is filled with its colour and opacity (stored premultiplied), its hole stays open.
#[test]
fn a_polygon_is_filled_and_its_hole_left_open() {
    let fill = PaintOf { fill: [250, 244, 180, 220], stroke: [0; 4], width_mm: 0.12 };
    let rings = vec![square(1000.0, 1985.0, 30.0), square(1010.0, 1995.0, 10.0)];
    let data = layer(&[(rings, fill)], true);
    let raster = render(&data, view()).expect("picture");
    // The square covers columns 10..40 and rows 5..35; the hole columns 20..30 and rows 15..25.
    let inside = pixel(&raster, 13, 8);
    assert_eq!(inside[3], 220);
    for (got, full) in inside[..3].iter().zip([250_u32, 244, 180]) {
        let want = (full * 220 + 127) / 255;
        assert!(u32::from(*got).abs_diff(want) <= 1, "{inside:?}");
    }
    assert_eq!(pixel(&raster, 25, 20)[3], 0, "the hole");
    assert_eq!(pixel(&raster, 5, 20)[3], 0, "left of the square");
    assert_eq!(pixel(&raster, 45, 20)[3], 0, "right of the square");
    assert_eq!(pixel(&raster, 25, 2)[3], 0, "above the square");
}

/// Two polygons of one look that overlap are both filled where they overlap (filling all shapes of a look
/// as one even-odd path would leave the overlap empty).
#[test]
fn overlapping_polygons_of_one_look_do_not_cancel() {
    let fill = PaintOf { fill: [10, 20, 30, 255], stroke: [0; 4], width_mm: 0.1 };
    let data = layer(&[(vec![square(1000.0, 1990.0, 20.0)], fill), (vec![square(1010.0, 1990.0, 20.0)], fill)], true);
    let raster = render(&data, view()).expect("picture");
    assert_eq!(pixel(&raster, 25, 20), [10, 20, 30, 255], "the overlap, columns 20..30");
    assert_eq!(pixel(&raster, 15, 20), [10, 20, 30, 255]);
    assert_eq!(pixel(&raster, 35, 20), [10, 20, 30, 255]);
}

/// The outline of a polygon is drawn over its fill, closed.
#[test]
fn the_outline_closes_and_lies_over_the_fill() {
    let paint = PaintOf { fill: [255, 255, 255, 255], stroke: [0, 0, 0, 255], width_mm: 0.5 };
    // Not closed in the data: the last side still has to be drawn.
    let open_ring = vec![[1000.0, 1990.0], [1040.0, 1990.0], [1040.0, 2010.0], [1000.0, 2010.0]];
    let data = layer(&[(vec![open_ring], paint)], true);
    let raster = render(&data, view()).expect("picture");
    assert_eq!(pixel(&raster, 30, 20), [255, 255, 255, 255], "fill");
    assert_eq!(pixel(&raster, 30, 10), [0, 0, 0, 255], "top side, world y 2010 is row 10");
    assert_eq!(pixel(&raster, 10, 20), [0, 0, 0, 255], "the side that closes the ring, world x 1000 is column 10");
}

/// Thousands of points closer together than a third of a pixel give the picture of the two end points.
#[test]
fn close_points_are_thinned_without_moving_the_line() {
    let dense: Vec<[f64; 2]> = (0..=10_000).map(|i| [1000.0 + f64::from(i) * 0.01, 2000.0]).collect();
    let thinned = render(&layer(&[(vec![dense], line_paint(0.5))], false), view()).expect("picture");
    let plain = render(&layer(&[(vec![vec![[1000.0, 2000.0], [1100.0, 2000.0]]], line_paint(0.5))], false), view()).expect("picture");
    let worst = thinned.rgba.iter().zip(&plain.rgba).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    assert!(worst <= 1, "differs by {worst}");
    assert!(plain.rgba.iter().any(|b| *b > 0));
    // A bend smaller than a pixel may go, the far end may not: the last point is always kept.
    let hook: Vec<[f64; 2]> = vec![[1000.0, 2000.0], [1000.1, 2000.1], [1000.2, 2000.0], [1100.0, 2000.0]];
    let hooked = render(&layer(&[(vec![hook], line_paint(0.5))], false), view()).expect("picture");
    assert_eq!(pixel(&hooked, 108, 20), GREY);
}

/// Shapes beside the view leave the picture empty; a view without pixels gives no picture.
#[test]
fn nothing_outside_the_view_is_drawn() {
    let data = layer(&[(vec![vec![[5000.0, 2000.0], [5100.0, 2000.0]]], line_paint(0.5))], false);
    let raster = render(&data, view()).expect("picture");
    assert!(raster.rgba.iter().all(|b| *b == 0));
    assert!(render(&data, RasterView { width: 0, ..view() }).is_none());
    assert!(render(&data, RasterView { metres_per_pixel: 0.0, ..view() }).is_none());
}

/// A line that only crosses the view, with both ends far outside, is drawn across it.
#[test]
fn a_line_crossing_the_view_is_drawn() {
    let data = layer(&[(vec![vec![[-50_000.0, 2000.0], [60_000.0, 2000.0]]], line_paint(0.5))], false);
    let raster = render(&data, view()).expect("picture");
    for x in [0, 60, 119] {
        assert_eq!(pixel(&raster, x, 20), GREY, "column {x}");
    }
}

/// Thinning leaves out only what cannot be seen: of ten thousand points a hundredth of a pixel apart at most
/// 300 are kept (one every 0.35 pixel over 100 pixels), and a zig-zag two pixels high keeps every corner.
#[test]
fn thinning_keeps_what_shows_and_drops_what_does_not() {
    let dense: Vec<[f64; 2]> = (0..=10_000).map(|i| [10.0 + f64::from(i) * 0.01, 20.0]).collect();
    let mut kept = Polylines::default();
    Pen::new(&mut kept).trace(&dense, false, None);
    assert!(kept.points.len() <= 300 && kept.points.len() >= 250, "{} points kept", kept.points.len());
    assert_eq!(kept.points[kept.points.len() - 1], [110.0, 20.0], "the last point stays");

    // Corners three pixels apart along x, two pixels up and down: world y 2000 and 2002 are rows 20 and 18.
    let zig_zag: Vec<[f64; 2]> = (0..=30).map(|i| [1000.0 + f64::from(i) * 3.0, if i % 2 == 0 { 2000.0 } else { 2002.0 }]).collect();
    let raster = render(&layer(&[(vec![zig_zag], line_paint(0.25))], false), view()).expect("picture");
    // The corner at world (1003, 2002) is picture point (13, 18). The pixel (12, 18) has its centre
    // (12.5, 18.5) at 0.14 from the side that climbs to it from (10, 20): covered 0.86 of a one-pixel line.
    assert!(pixel(&raster, 12, 18)[3] > 200, "the side up to the peak: {:?}", pixel(&raster, 12, 18));
    // Under the peak, at the height of the valleys, there is no line: the centre (13.5, 20.5) lies 1.8 from
    // the side that comes down. A line flattened by coarse thinning would run right through it.
    assert_eq!(pixel(&raster, 13, 20)[3], 0, "under a peak the line is not at the valley height");
}

/// Zoomed in to a millimetre a pixel, a line 140 km long crosses the picture; its ends lie fifty million
/// pixels away. What is handed to the line drawer is the piece inside the box 64 pixels around the picture
/// (two points, on the box), not a line the drawer would have to walk for a hundred million pixels; and it
/// is still the same line: through picture point (60, 20), one pixel up for each pixel to the right.
#[test]
fn a_line_from_far_away_is_cut_to_the_picture_before_it_is_drawn() {
    let far = vec![[1000.0 - 50_000.0, 2000.0 - 50_000.0], [1000.0 + 50_000.0, 2000.0 + 50_000.0]];
    let close_up = RasterView { left: 999.94, top: 2000.02, metres_per_pixel: 0.001, width: 120, height: 40, pixels_per_mm: 4.0 };
    let data = layer(&[(vec![far], line_paint(0.25))], false);
    let frame = Frame::new(&data, &close_up);
    let lines = frame.lines_of((GREY, 0.25_f32.to_bits()));
    assert_eq!(lines.points.len(), 2, "{:?}", lines.points);
    for p in &lines.points {
        assert!((-64.0..=184.0).contains(&p[0]) && (-64.0..=104.0).contains(&p[1]), "{p:?} lies outside the box around the picture");
        assert!((p[0] + p[1] - 80.0).abs() < 0.01, "{p:?} is not on the line");
    }
    // The picture itself: the pixels (60 + k, 19 - k) have their centres on the line.
    let raster = render(&data, close_up).expect("picture");
    for (x, y) in [(60, 19), (45, 34), (75, 4)] {
        assert_eq!(pixel(&raster, x, y), GREY, "({x}, {y})");
        assert_eq!(pixel(&raster, x + 2, y)[3], 0, "({}, {y})", x + 2);
    }
}
