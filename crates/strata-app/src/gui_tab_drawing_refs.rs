//! The reference layers in the map frame of a sheet: what is checked on the map (map sheets, soil, rivers,
//! the heritage rows …) is drawn on the sheet too, at the scale of the sheet, as the C++ sheet takes the
//! checked layers of the map (`target/cpp-ref/src/app/KaDrawingStudio.cpp:2121-2131`).

use eframe::egui::Rect;
use strata_canvas2d::viewport::ViewportTransform;

use super::gui_tab_drawing_frame::SheetFrame;
use crate::app_state::StrataAppState;
use crate::survey_style::mm_to_points;

#[cfg(test)]
#[path = "gui_tab_drawing_refs_tests.rs"]
mod tests;

/// The map frame as a view of the map, and how many times the screen shows the paper enlarged.
pub(crate) fn sheet_reference_view(map_rect: Rect, centre: [f64; 2], px_per_meter: f64, px_per_mm: f32) -> (ViewportTransform, f32) {
    let view = ViewportTransform::new(centre, 1.0 / px_per_meter, [f64::from(map_rect.width()), f64::from(map_rect.height())]);
    (view, px_per_mm / mm_to_points(1.0))
}

/// Whether the letters of a reference row are written on a sheet. The names of the heritage sites are not:
/// on a sheet the numbers stand for them (`HeritageLayoutNumbers.cpp:744, 963-965`).
pub(crate) fn sheet_writes_labels_of(key: &str) -> bool {
    !key.starts_with(crate::heritage_run::KEY_PREFIX)
}

impl StrataAppState {
    /// Draws what is checked on the map into the map frame: the reference rows, and over them the survey's
    /// own layers (survey area, trenches, features, the distance rings), as the map draws them.
    pub(crate) fn paint_sheet_reference_layers(&mut self, f: &SheetFrame) {
        let (view, paper_zoom) = sheet_reference_view(f.map_rect, f.m2w(f.map_rect.center()), f.px_per_meter, f.px_per_mm);
        let ctx = f.map_p.ctx().clone();
        self.paint_reference_vector_layers_in(&ctx, &f.map_p, f.map_rect, &view, paper_zoom, sheet_writes_labels_of);
        self.paint_survey_tables_in(&ctx, &f.map_p, f.map_rect, &view, paper_zoom);
    }
}
