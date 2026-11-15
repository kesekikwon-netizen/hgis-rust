//! The colour bar of the DEM in the legend (`target/cpp-ref/src/core/DemColorRampLegend.cpp`): 「표고 (m)」,
//! a bar in the DEM's own colours with ticks that stand evenly whatever their heights, and 「비선형 눈금」.

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, Stroke};

use super::DEM_KEY;
use crate::app_state::StrataAppState;
use crate::ref_vector::data::{HeightRamp, Look};
use crate::ref_vector::elevation::height_colour;

#[cfg(test)]
#[path = "dem_legend_tests.rs"]
mod tests;

/// The height of the picture and the place of the bar in it, logical pixels (`:21-26`).
pub(crate) const HEIGHT: f32 = 156.0;
const BAR_TOP: f32 = 24.0;
const BAR_HEIGHT: f32 = 108.0;
const BAR_LEFT: f32 = 5.0;
const BAR_WIDTH: f32 = 18.0;
/// The round heights that become ticks when they lie between the ends (`:76`).
const ROUND: [f64; 7] = [0.0, 100.0, 200.0, 500.0, 1000.0, 1500.0, 2000.0];

/// The ticks of the bar, from its foot to its top (`:63-88`): the lowest height, the round heights strictly
/// between, the highest; fewer than five become five even ones, more than seven are thinned to seven.
pub(crate) fn ticks(ramp: HeightRamp) -> Vec<f64> {
    // What the colours are spread over (`DemPresentation.cpp:50-64`).
    let (low, high) = match ramp {
        HeightRamp::National | HeightRamp::Lowland => (0.0, 2000.0),
        HeightRamp::Range { low, high } => (f64::from(low), f64::from(high)),
    };
    let mut all = vec![low];
    all.extend(ROUND.iter().copied().filter(|round| *round > low && *round < high));
    all.push(high);
    // At most nine ticks: small whole numbers, and a place among them rounded as qRound rounds.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    match all.len() {
        0..=4 => (0..5).map(|i| low + (high - low) * f64::from(i) / 4.0).collect(),
        5..=7 => all,
        many => (0..7).map(|i| all[(f64::from(i) * (many - 1) as f64 / 6.0).round() as usize]).collect(),
    }
}

/// The height at a place of the bar, 0 at its foot and 1 at its top: the ticks stand evenly (`:36-41`).
pub(crate) fn height_at(ticks: &[f64], place: f64) -> f64 {
    if ticks.len() < 2 {
        return ticks.first().copied().unwrap_or(0.0);
    }
    // A place between 0 and the number of ticks.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (along, lower) = {
        let along = place.clamp(0.0, 1.0) * (ticks.len() - 1) as f64;
        (along, (along as usize).min(ticks.len() - 2))
    };
    #[allow(clippy::cast_precision_loss)]
    let share = along - lower as f64;
    ticks[lower] + (ticks[lower + 1] - ticks[lower]) * share
}

/// The text of a tick, as `QString::number(value, 'g', 8)` writes a height: eight significant digits, no
/// zeros at the end (`:49`).
pub(crate) fn tick_text(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return "0".to_owned();
    }
    // The digits before the point of a height: a small whole number.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let decimals = (7 - value.abs().log10().floor() as i32).clamp(0, 16) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

impl StrataAppState {
    /// The colours of the DEM while it is in the legend.
    pub(crate) fn dem_legend_ramp(&self) -> Option<HeightRamp> {
        self.dem_row_name()?;
        match self.ref_vectors.look_of(DEM_KEY) {
            Some(Look::Heights { ramp }) => Some(*ramp),
            _ => None,
        }
    }

    /// Paints the bar with its left top corner at `at` (`paintLegend`, `:110-160`; the legend panel gives it
    /// `HEIGHT` of room under the DEM's row).
    pub(crate) fn paint_dem_legend(&self, painter: &egui::Painter, at: Pos2, ramp: HeightRamp) {
        let (ink, rule) = (Color32::from_rgb(31, 35, 40), Stroke::new(0.6, Color32::from_rgb(90, 95, 100)));
        let bold = FontId::new(11.0, FontFamily::Name(crate::gui_tab_map::LABEL_FONT_FAMILY.into()));
        painter.text(Pos2::new(at.x + 4.0, at.y + 9.0), egui::Align2::LEFT_CENTER, "표고 (m)", bold, ink);
        let marks = ticks(ramp);
        let (left, top) = (at.x + BAR_LEFT, at.y + BAR_TOP);
        // Whole rows of the screen, both end colours among them.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
        let rows = ((BAR_HEIGHT * painter.ctx().pixels_per_point()).round() as usize).clamp(2, 4096);
        #[allow(clippy::cast_precision_loss)]
        let row_height = BAR_HEIGHT / rows as f32;
        for row in 0..rows {
            #[allow(clippy::cast_precision_loss)]
            let (place, y) = (1.0 - row as f64 / (rows - 1) as f64, top + row as f32 * row_height);
            // Heights of a DEM fit a single-precision number.
            #[allow(clippy::cast_possible_truncation)]
            let [red, green, blue] = height_colour(height_at(&marks, place) as f32, ramp);
            painter.rect_filled(Rect::from_min_max(Pos2::new(left, y), Pos2::new(left + BAR_WIDTH, y + row_height)), 0.0, Color32::from_rgb(red, green, blue));
        }
        painter.rect_stroke(Rect::from_min_max(Pos2::new(left - 1.0, top - 1.0), Pos2::new(left + BAR_WIDTH + 1.0, top + BAR_HEIGHT + 1.0)), 0.0, rule, egui::StrokeKind::Middle);
        for (index, mark) in marks.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let y = top + (BAR_HEIGHT - 1.0) * (1.0 - index as f32 / (marks.len() - 1) as f32) + 0.5;
            painter.line_segment([Pos2::new(left + BAR_WIDTH + 1.0, y), Pos2::new(at.x + 28.0, y)], rule);
            painter.text(Pos2::new(at.x + 32.0, y), egui::Align2::LEFT_CENTER, tick_text(*mark), FontId::proportional(11.0), ink);
        }
        painter.text(Pos2::new(at.x + 4.0, at.y + 148.0), egui::Align2::LEFT_CENTER, "비선형 눈금", FontId::proportional(9.0), Color32::from_rgb(85, 90, 95));
    }
}
