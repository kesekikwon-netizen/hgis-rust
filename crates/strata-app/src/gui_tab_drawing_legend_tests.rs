//! The rows of the legend of a sheet.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;

/// Owner 2026-11-15: 「범례는 레이어가 선택일때 자동으로 배치되게하라」. The legend of a sheet lists the layers that
/// are checked in the legend panel, top to bottom, by the names the panel shows; unchecking a layer takes its
/// row out. The cadastral map is listed (its lines are on the sheet); the picture of a base map has no row,
/// and the rows of the 「유산」 button have none here: their sites are listed one by one.
#[test]
fn the_legend_lists_the_layers_checked_in_the_panel() {
    let mut app = StrataAppState::new_startup();
    app.left_layers.apply_default_basemaps();
    for key in [DomainLayerKey::ControlPoints, DomainLayerKey::FeatureLine, DomainLayerKey::FeaturePoly, DomainLayerKey::SurveyArea] {
        app.left_layers.ensure_domain_layer(key);
    }
    let panel: Vec<String> = app.left_layers.entries.iter().filter(|e| e.group != strata_types::schema::LegendGroup::Basemap).map(|e| e.label_ko.clone()).collect();
    assert_eq!(panel.len(), 5, "four layers of the survey and the cadastral map");
    assert!(app.left_layers.entries.iter().any(|e| e.group == strata_types::schema::LegendGroup::Basemap), "the satellite map is in the panel");
    assert_eq!(app.sheet_legend_layer_names(), panel, "the checked layers in the order of the panel, and no base map");

    // A ring has a row, its distance letters have none (the two rows the 버퍼 button makes, app_state_buffer.rs).
    for (key, name) in [("user:buffer_ring_500", "주변 500m"), ("user:buffer_label_500", "주변 500m 거리")] {
        let mut row = app.left_layers.entries.iter().find(|e| e.group == strata_types::schema::LegendGroup::SurveyData).unwrap().clone();
        (row.layer_key, row.label_ko) = (key.to_owned(), name.to_owned());
        app.left_layers.entries.insert(0, row);
    }
    assert_eq!(app.sheet_legend_layer_names()[..2], ["주변 500m".to_owned(), panel[0].clone()]);
    app.left_layers.entries.retain(|e| !e.layer_key.starts_with("user:buffer_"));

    app.home_map.real_survey.layer_visible.insert(panel[1].clone(), false);
    let checked: Vec<String> = panel.iter().filter(|name| **name != panel[1]).cloned().collect();
    assert_eq!(app.sheet_legend_layer_names(), checked, "an unchecked layer has no row");
}
