//! Which picture a view of the map needs, and whether a picture at hand still serves.

use eframe::egui::Rect;
use strata_canvas2d::viewport::ViewportTransform;

use crate::gui_tab_map::s2w;
use crate::ref_vector::raster::RasterView;

/// The picture reaches this many pixels beyond the map on every side: a small pan needs no new picture.
const MARGIN: f32 = 256.0;

/// The picture to make for this view of the map, and the world box of the map itself
/// (`[min x, min y, max x, max y]`). The picture is the map's own pixels plus the margin on every side.
/// When that is more than the graphics card takes as one picture (`max_side`), the same part of the world
/// is drawn with fewer and coarser pixels: a picture cut off at the limit would leave the far side of a
/// wide map empty. `paper_zoom` is 1 on the map.
pub(super) fn wanted_view(rect: Rect, vp: &ViewportTransform, scale: f32, max_side: usize, paper_zoom: f32) -> (RasterView, [f64; 4]) {
    let metres_per_pixel = vp.meters_per_pixel / f64::from(scale);
    let (top_left, bottom_right) = (s2w(rect, vp, rect.left_top()), s2w(rect, vp, rect.right_bottom()));
    let wanted = |points: f32| (points * scale).ceil() + 2.0 * MARGIN;
    let (across, down) = (wanted(rect.width()), wanted(rect.height()));
    let coarser = (across.max(down) / max_side as f32).max(1.0);
    // Whole pixels, at least one.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = |pixels: f32| (pixels / coarser).ceil().max(1.0) as u32;
    let view = RasterView {
        left: top_left[0] - f64::from(MARGIN) * metres_per_pixel,
        top: top_left[1] + f64::from(MARGIN) * metres_per_pixel,
        metres_per_pixel: metres_per_pixel * f64::from(coarser),
        width: side(across),
        height: side(down),
        // On a sheet a millimetre is one of the paper, which the screen shows `paper_zoom` times as large.
        pixels_per_mm: crate::survey_style::mm_to_points(1.0) * paper_zoom * scale / coarser,
    };
    (view, [top_left[0], bottom_right[1], bottom_right[0], top_left[1]])
}

/// Whether a picture still serves: the same scale as the wanted one (to a thousandth), and the map lies
/// inside it.
pub(super) fn covers(have: &RasterView, wanted: &RasterView, map: &[f64; 4]) -> bool {
    let same_scale = (have.metres_per_pixel / wanted.metres_per_pixel - 1.0).abs() < 1e-3;
    let have = have.world();
    same_scale && map[0] >= have[0] && map[1] >= have[1] && map[2] <= have[2] && map[3] <= have[3]
}

#[cfg(test)]
#[path = "ref_vector_view_tests.rs"]
mod tests;
