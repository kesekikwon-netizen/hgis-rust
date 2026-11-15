//! The picture a view needs, on the map and on a sheet.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{pos2, vec2, Rect};
use strata_canvas2d::viewport::ViewportTransform;

use super::*;

/// On the map a millimetre is one of the screen; on a sheet it is one of the paper, which the screen shows
/// `paper_zoom` times as large. Nothing else of the picture changes.
#[test]
fn a_view_of_a_sheet_counts_millimetres_of_paper() {
    let rect = Rect::from_min_size(pos2(100.0, 50.0), vec2(400.0, 300.0));
    let vp = ViewportTransform::new([220_000.0, 370_000.0], 2.0, [400.0, 300.0]);
    let (map, map_box) = wanted_view(rect, &vp, 1.0, 8192, 1.0);
    let (sheet, sheet_box) = wanted_view(rect, &vp, 1.0, 8192, 2.5);
    assert_eq!(map_box, sheet_box);
    assert_eq!((map.left, map.top, map.metres_per_pixel, map.width, map.height), (sheet.left, sheet.top, sheet.metres_per_pixel, sheet.width, sheet.height));
    assert!((map.pixels_per_mm - crate::survey_style::mm_to_points(1.0)).abs() < 1e-6);
    assert!((sheet.pixels_per_mm - 2.5 * map.pixels_per_mm).abs() < 1e-5, "{} against {}", sheet.pixels_per_mm, map.pixels_per_mm);
}
