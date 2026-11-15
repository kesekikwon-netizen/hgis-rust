//! 「DEM 표현」: what 적용 does to the DEM and its relief, and how the colours follow the map.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use strata_engine::ElevationGrid;

use super::*;
use crate::dialogs_modal::ActiveModalDialog;
use crate::ref_vector::data::Look;
use crate::ref_vector::elevation::Drawn;
use crate::topo_download::dem::{DEM_KEY, RELIEF_KEY};

/// The whole grid; its two northern rows and three western columns; its north-west cell; no cell at all.
const WHOLE: [f64; 4] = [0.0, 0.0, 900.0, 900.0];
const PART: [f64; 4] = [0.0, 720.0, 270.0, 900.0];
const ONE: [f64; 4] = [0.0, 810.0, 90.0, 900.0];
const BESIDE: [f64; 4] = [5000.0, 5000.0, 6000.0, 6000.0];

/// A survey whose DEM is on the map: 10 x 10 cells of 90 m from (0, 900) on. The north-west cell is 40 m;
/// the others rise by one metre from 32.5 m (row by row from the north) to 130.5 m.
fn app_with_dem() -> (StrataAppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    app.show_survey_dem();
    #[allow(clippy::cast_precision_loss)]
    let heights = (0..100).map(|i| if i == 0 { 40.0 } else { 31.5 + i as f32 }).collect();
    let grid = ElevationGrid { left: 0.0, top: 900.0, cell: 90.0, width: 10, height: 10, heights, shade: vec![181; 100] };
    app.ref_vectors.set_ready(DEM_KEY, Drawn::Heights(Arc::new(grid)));
    app.ref_vectors.map_box = Some(WHOLE);
    (app, dir)
}

fn relief(app: &StrataAppState) -> (f32, bool) {
    let row = app.left_layers.entries.iter().find(|e| e.layer_key == RELIEF_KEY).unwrap();
    (row.opacity, app.home_map.real_survey.is_layer_visible(&row.label_ko))
}

fn colours(app: &StrataAppState) -> Option<HeightRamp> {
    match app.ref_vectors.look_of(DEM_KEY) {
        Some(Look::Heights { ramp }) => Some(*ramp),
        _ => None,
    }
}

/// 적용 (`KaDemClassDialog::applyStyle`): the DEM takes the colour preset; the relief takes the
/// exaggeration, lies over the map as strongly as 「음영 세기」 says, and is shown while 「지형 음영 합성」 is
/// checked. Values beyond what the dialog offers (0.1 to 5 times, 0 to 80 %) are brought inside.
#[test]
fn applying_gives_the_dem_its_colours_and_the_relief_its_strength() {
    let (mut app, _dir) = app_with_dem();
    assert_eq!((colours(&app), relief(&app), app.dem_look), (Some(HeightRamp::National), (0.30, true), DemLook::default()));

    let wanted = DemLook { preset: DemPreset::Lowland, relief: true, z_factor: 2.0, strength_pct: 50 };
    assert_eq!(app.apply_dem_look(wanted), APPLIED);
    assert_eq!((colours(&app), relief(&app), app.dem_look), (Some(HeightRamp::Lowland), (0.5, true), wanted));
    assert_eq!(app.ref_vectors.look_of(RELIEF_KEY), Some(&Look::Relief { z_factor: 2.0 }));

    assert_eq!(app.apply_dem_look(DemLook { preset: DemPreset::National, relief: false, z_factor: 9.0, strength_pct: 95 }), APPLIED);
    assert_eq!((colours(&app), relief(&app)), (Some(HeightRamp::National), (0.8, false)));
    assert_eq!(app.ref_vectors.look_of(RELIEF_KEY), Some(&Look::Relief { z_factor: 5.0 }));
    assert_eq!(app.dem_look, DemLook { preset: DemPreset::National, relief: false, z_factor: 5.0, strength_pct: 80 });

    // A relief that is switched off stays off when the DEM is hidden and shown again with its button, and
    // one that is switched on is shown only while the DEM is (`ensureDemRelief`, BasemapOps.cpp:2106).
    app.handle_ribbon_click("DEM");
    app.handle_ribbon_click("DEM");
    assert!(app.dem_shown() && !relief(&app).1);
    app.handle_ribbon_click("DEM");
    assert_eq!(app.apply_dem_look(DemLook { relief: true, ..app.dem_look }), APPLIED);
    assert!(!app.dem_shown() && !relief(&app).1, "the DEM is hidden: so is its relief");
    app.handle_ribbon_click("DEM");
    assert!(app.dem_shown() && relief(&app).1);
}

/// 「현재 화면 맞춤」 (`DemPresentation.cpp:53-64`): the colours reach from the lowest height of the map,
/// rounded down, to the highest, rounded up; one height alone gets a metre of room. A map without any
/// height of the DEM changes nothing and says so.
#[test]
fn the_view_preset_takes_its_colours_from_the_heights_in_the_map() {
    let (mut app, _dir) = app_with_dem();
    let fitted = DemLook { preset: DemPreset::Viewport, ..DemLook::default() };
    assert_eq!(app.apply_dem_look(fitted), APPLIED);
    assert_eq!(colours(&app), Some(HeightRamp::Range { low: 32.0, high: 131.0 }));
    app.ref_vectors.map_box = Some(PART);
    assert_eq!(app.apply_dem_look(fitted), APPLIED);
    assert_eq!(colours(&app), Some(HeightRamp::Range { low: 32.0, high: 44.0 }));
    app.ref_vectors.map_box = Some(ONE);
    assert_eq!(app.apply_dem_look(fitted), APPLIED);
    assert_eq!(colours(&app), Some(HeightRamp::Range { low: 40.0, high: 41.0 }));

    assert_eq!(app.apply_dem_look(DemLook { preset: DemPreset::Lowland, ..DemLook::default() }), APPLIED);
    for nowhere in [Some(BESIDE), None] {
        app.ref_vectors.map_box = nowhere;
        assert_eq!(app.apply_dem_look(DemLook { strength_pct: 70, ..fitted }), NO_HEIGHTS_HERE);
        assert_eq!((colours(&app), app.dem_look.preset, relief(&app).0), (Some(HeightRamp::Lowland), DemPreset::Lowland, 0.30), "nothing is changed");
    }
}

/// With 「현재 화면 맞춤」 the colours follow the map (`DemPresentation::followCanvas`): 350 ms after the map
/// last moved they are taken again from what it shows. Over ground without heights the colours stay and
/// the status line says why. The other presets do not follow.
#[test]
fn the_colours_follow_the_map_once_it_stands_still() {
    let (mut app, _dir) = app_with_dem();
    assert!(!app.follow_dem_view(1.0), "the national colours follow nothing");
    assert_eq!(app.apply_dem_look(DemLook { preset: DemPreset::Viewport, ..DemLook::default() }), APPLIED);
    let whole = Some(HeightRamp::Range { low: 32.0, high: 131.0 });
    assert!(app.follow_dem_view(10.0), "the map as it stands is noted");

    app.ref_vectors.map_box = Some(PART);
    assert!(app.follow_dem_view(10.10), "the map moved");
    assert!(app.follow_dem_view(10.30));
    assert_eq!(colours(&app), whole, "0.2 s after the move: not yet");
    assert!(!app.follow_dem_view(10.46));
    assert_eq!(colours(&app), Some(HeightRamp::Range { low: 32.0, high: 44.0 }));
    assert!(!app.follow_dem_view(11.0), "nothing more to wait for");

    app.ref_vectors.map_box = Some(BESIDE);
    assert!(app.follow_dem_view(12.0));
    assert!(!app.follow_dem_view(12.4));
    assert_eq!(colours(&app), Some(HeightRamp::Range { low: 32.0, high: 44.0 }));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(KEPT_RAMP));

    assert_eq!(app.apply_dem_look(DemLook::default()), APPLIED);
    app.ref_vectors.map_box = Some(PART);
    assert!(!app.follow_dem_view(13.0) && !app.follow_dem_view(14.0));
    assert_eq!(colours(&app), Some(HeightRamp::National));
}

/// 「DEM 표현…」 opens over a DEM only (`MainWindowOverlay.cpp:285-290`) and shows what the DEM has now. Over
/// ground whose heights differ by less than 100 m it points at 「현재 화면에 색 맞추기」
/// (`KaDemClassDialog::suggestViewport`), unless the colours are fitted to the view already.
#[test]
fn the_dialog_opens_over_a_dem_and_shows_what_it_has() {
    let mut bare = StrataAppState::new_startup();
    bare.run_dem_menu_action("DEM 표현…");
    assert_eq!((bare.modals.active, bare.home_map.draw_guide_message.as_deref()), (None, Some(NEEDS_DEM)));
    assert_eq!(bare.apply_dem_look(DemLook::default()), NEEDS_DEM);

    let (mut app, _dir) = app_with_dem();
    assert_eq!(app.apply_dem_look(DemLook { preset: DemPreset::Lowland, relief: false, z_factor: 1.5, strength_pct: 40 }), APPLIED);
    app.run_dem_menu_action("DEM 표현…");
    assert_eq!(app.modals.active, Some(ActiveModalDialog::DemClass));
    assert_eq!((app.modals.dem_preset_idx, app.modals.dem_relief_enabled, app.modals.dem_exaggeration, app.modals.dem_relief_strength_pct), (1, false, 1.5, 40));
    assert_eq!(app.modals.dem_status_text, "지금 화면의 높이차는 약 98 m뿐이라 이 색 표현으로는 거의 한 색으로 보입니다. 「현재 화면에 색 맞추기」를 누르면 화면 안의 높이로 색을 나눕니다.");

    // One height alone: the difference is said as one metre at least. Fitted colours need no pointer.
    app.ref_vectors.map_box = Some(ONE);
    assert_eq!(app.dem_view_hint().as_deref(), Some("지금 화면의 높이차는 약 1 m뿐이라 이 색 표현으로는 거의 한 색으로 보입니다. 「현재 화면에 색 맞추기」를 누르면 화면 안의 높이로 색을 나눕니다."));
    assert_eq!(app.apply_dem_look(DemLook { preset: DemPreset::Viewport, ..app.dem_look }), APPLIED);
    assert_eq!(app.dem_view_hint(), None);
    app.modals.active = None;
    app.run_dem_menu_action("DEM 표현…");
    assert_eq!((app.modals.dem_preset_idx, app.modals.dem_status_text.as_str()), (2, ""));
}
