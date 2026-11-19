//! The map opacity rail changes the selected legend row only.
//! C++ `MainWindow.cpp:989-1000` calls `LayerOps::applyLayerOpacity` on `currentLayer()`.

use strata_types::schema::{DomainLayerKey, LegendGroup};

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::ref_tile_layers::{ensure_row, DAEDONGYEOJIDO};

const PICTURE: &str = "현장사진";

fn app_with_rows() -> StrataAppState {
    let mut app = StrataAppState::new_startup();
    app.left_layers.apply_default_basemaps();
    app.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
    ensure_row(&mut app.left_layers, &DAEDONGYEOJIDO);
    app.left_layers.entries.insert(
        0,
        LayerTreeEntry {
            layer_key: "picture_field".to_owned(),
            label_ko: PICTURE.to_owned(),
            group: LegendGroup::ReferenceMap,
            visible: true,
            opacity: 1.0,
            read_only: true,
            snap_enabled: false,
            feature_count: 0,
        },
    );
    app
}

fn row(app: &StrataAppState, label: &str) -> f32 {
    app.left_layers.entries.iter().find(|e| e.label_ko == label).unwrap_or_else(|| panic!("missing row {label}")).opacity
}

/// 0.25, 0.3, 0.4, 0.75 and 1 are exact f32 values, so this is an assignment check.
fn expect_same(errors: &mut Vec<String>, label: &str, got: f32, want: f32) {
    if (got - want).abs() >= 1e-6 {
        errors.push(format!("{label}: {got} != {want}"));
    }
}

fn assert_clear(errors: Vec<String>) {
    assert!(errors.is_empty(), "{}", errors.join("; "));
}

fn shared_slots(app: &StrataAppState) -> (f32, f32) {
    (app.home_map.real_survey.layer_opacity, app.home_map.basemap_opacity)
}

#[test]
fn setting_the_survey_area_to_a_quarter_leaves_the_satellite_and_background_alone() {
    let mut app = app_with_rows();
    let (global, background) = shared_slots(&app);
    let satellite = row(&app, "위성지도 (VWorld)");
    let cadastral = row(&app, "연속지적도 (VWorld WMS)");

    app.apply_rail_opacity("조사구역", 0.25);

    let mut errors = Vec::new();
    expect_same(&mut errors, "조사구역 row", row(&app, "조사구역"), 0.25);
    expect_same(&mut errors, "조사구역 rail", app.rail_opacity("조사구역"), 0.25);
    expect_same(&mut errors, "satellite row", row(&app, "위성지도 (VWorld)"), satellite);
    expect_same(&mut errors, "satellite rail", app.rail_opacity("위성지도 (VWorld)"), satellite);
    expect_same(&mut errors, "cadastral row", row(&app, "연속지적도 (VWorld WMS)"), cadastral);
    expect_same(&mut errors, "cadastral rail", app.rail_opacity("연속지적도 (VWorld WMS)"), cadastral);
    expect_same(&mut errors, "대동여지도 row", row(&app, "대동여지도"), 1.0);
    expect_same(&mut errors, "picture row", row(&app, PICTURE), 1.0);
    expect_same(&mut errors, "global layer_opacity", app.home_map.real_survey.layer_opacity, global);
    expect_same(&mut errors, "background opacity", app.home_map.basemap_opacity, background);
    assert_clear(errors);
}

#[test]
fn selecting_a_picture_row_changes_only_that_row() {
    let mut app = app_with_rows();
    let (global, background) = shared_slots(&app);

    app.apply_rail_opacity("대동여지도", 0.3);
    app.apply_rail_opacity(PICTURE, 0.4);

    let mut errors = Vec::new();
    expect_same(&mut errors, "대동여지도 row", row(&app, "대동여지도"), 0.3);
    expect_same(&mut errors, "대동여지도 rail", app.rail_opacity("대동여지도"), 0.3);
    expect_same(&mut errors, "picture row", row(&app, PICTURE), 0.4);
    expect_same(&mut errors, "picture rail", app.rail_opacity(PICTURE), 0.4);
    expect_same(&mut errors, "조사구역 row", row(&app, "조사구역"), 1.0);
    expect_same(&mut errors, "satellite row", row(&app, "위성지도 (VWorld)"), 1.0);
    expect_same(&mut errors, "satellite rail", app.rail_opacity("위성지도 (VWorld)"), 1.0);
    expect_same(&mut errors, "cadastral rail", app.rail_opacity("연속지적도 (VWorld WMS)"), 0.75);
    expect_same(&mut errors, "global layer_opacity", app.home_map.real_survey.layer_opacity, global);
    expect_same(&mut errors, "background opacity", app.home_map.basemap_opacity, background);
    assert_clear(errors);
}
