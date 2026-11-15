//! The colour bar of the DEM in the legend: its ticks, the height at a place of the bar, and its texts.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::app_state::StrataAppState;

/// `DemColorRampLegend.cpp:63-88`: the lowest height, the round heights 0, 100, 200, 500, 1000, 1500 and
/// 2000 m that lie strictly between, the highest; fewer than five become five even ones, more than seven
/// are thinned to seven.
#[test]
fn the_ticks_of_the_bar() {
    assert_eq!(ticks(HeightRamp::National), [0.0, 100.0, 200.0, 500.0, 1000.0, 1500.0, 2000.0]);
    assert_eq!(ticks(HeightRamp::Lowland), [0.0, 100.0, 200.0, 500.0, 1000.0, 1500.0, 2000.0]);
    assert_eq!(ticks(HeightRamp::Range { low: 13.0, high: 725.0 }), [13.0, 100.0, 200.0, 500.0, 725.0]);
    assert_eq!(ticks(HeightRamp::Range { low: 32.0, high: 44.0 }), [32.0, 35.0, 38.0, 41.0, 44.0]);
    assert_eq!(ticks(HeightRamp::Range { low: 40.0, high: 41.0 }), [40.0, 40.25, 40.5, 40.75, 41.0]);
    // Eight ticks are thinned to seven: places 0, 1, 2, 4, 5, 6, 7 of the eight (3.5 rounds up, as qRound).
    assert_eq!(ticks(HeightRamp::Range { low: -20.0, high: 1900.0 }), [-20.0, 0.0, 100.0, 500.0, 1000.0, 1500.0, 1900.0]);
}

/// The ticks stand evenly along the bar, whatever their heights (「비선형 눈금」, `:36-41`): a place of the
/// bar, 0 at its foot and 1 at its top, has the height between its two ticks.
#[test]
fn the_height_at_a_place_of_the_bar() {
    let national = ticks(HeightRamp::National);
    for (place, height) in [(0.0, 0.0), (1.0, 2000.0), (0.5, 500.0), (0.25, 150.0), (11.0 / 12.0, 1750.0), (1.2, 2000.0), (-0.3, 0.0)] {
        assert!((height_at(&national, place) - height).abs() < 1e-9, "place {place}: {}", height_at(&national, place));
    }
}

/// A tick is written as `QString::number(value, 'g', 8)` writes it: eight significant digits, no zeros
/// at the end.
#[test]
fn the_text_of_a_tick() {
    for (value, text) in [(13.0, "13"), (2000.0, "2000"), (0.0, "0"), (-20.0, "-20"), (40.25, "40.25"), (100.0 / 3.0, "33.333333"), (1234.5678912, "1234.5679")] {
        assert_eq!(tick_text(value), text);
    }
}

/// The bar stands under the DEM's row while the DEM is in the legend, in the colours the DEM has now.
#[test]
fn the_bar_shows_the_colours_the_dem_has() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let mut app = StrataAppState::new_startup();
    assert_eq!(app.dem_legend_ramp(), None);
    app.survey_path = Some(survey);
    app.show_survey_dem();
    assert_eq!(app.dem_legend_ramp(), Some(HeightRamp::National));
    app.ref_vectors.set_look(crate::topo_download::dem::DEM_KEY, crate::ref_vector::data::Look::Heights { ramp: HeightRamp::Lowland });
    assert_eq!(app.dem_legend_ramp(), Some(HeightRamp::Lowland));
}
