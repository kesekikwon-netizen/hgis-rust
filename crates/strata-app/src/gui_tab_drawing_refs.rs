//! The reference layers in the map frame of a sheet: what is checked on the map (map sheets, soil, rivers,
//! the heritage rows …) is drawn on the sheet too, at the scale of the sheet, as the C++ sheet takes the
//! checked layers of the map (`target/cpp-ref/src/app/KaDrawingStudio.cpp:2121-2131`).

use eframe::egui::{Align2, Color32, FontFamily, FontId, Rect, Stroke};
use strata_canvas2d::viewport::ViewportTransform;

use super::gui_tab_drawing_frame::SheetFrame;
use crate::app_state::StrataAppState;
use crate::gui_tab_map::LABEL_FONT_FAMILY;
use crate::sheet_numbers::badge_look;
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
        self.paint_sheet_heritage_badges(f);
    }

    /// The numbers of the heritage sites near the survey area, each on its site: a circle in the colour
    /// of the kind with the number in it, and no line to it (`HeritageLayoutNumbers.cpp:1027-1110`).
    fn paint_sheet_heritage_badges(&mut self, f: &SheetFrame) {
        let (north_west, south_east) = (f.m2w(f.map_rect.left_top()), f.m2w(f.map_rect.right_bottom()));
        let badges = self.sheet_heritage_badges([north_west[0], south_east[1], south_east[0], north_west[1]]);
        let bold = FontFamily::Name(LABEL_FONT_FAMILY.into());
        let family = if f.map_p.ctx().fonts(|fonts| fonts.families().contains(&bold)) { bold } else { FontFamily::Proportional };
        for badge in &badges {
            let look = badge_look(badge.number, badge.colour, f.px_per_mm);
            let (at, [red, green, blue], [ink_red, ink_green, ink_blue]) = (f.w2m(badge.at), badge.colour, look.ink);
            f.map_p.circle(at, look.diameter * 0.5, Color32::from_rgb(red, green, blue), Stroke::NONE);
            f.map_p.text(at, Align2::CENTER_CENTER, badge.number.to_string(), FontId::new(look.letter, family.clone()), Color32::from_rgb(ink_red, ink_green, ink_blue));
        }
    }
}
