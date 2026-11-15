//! The rows of the legend of a sheet.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, text::TextFormat, vec2, FontFamily, Pos2, RawInput, Rect};
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::gui_tab_map::LABEL_FONT_FAMILY;

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

/// The look of the legend's title as the sheet paints it, with a bold face in the fonts.
fn title_format(app: &mut StrataAppState) -> TextFormat {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(FontFamily::Name(LABEL_FONT_FAMILY.into()), vec!["Ubuntu-Light".to_owned()]);
    ctx.set_fonts(fonts);
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 900.0))), ..Default::default() };
    let mut found = None;
    // The fonts are taken at the start of the next pass.
    for _ in 0..2 {
        let out = ctx.run(raw.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
        });
        let title = app.drawing.sheet.legend_title.clone();
        found = out.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if text.galley.text() == title => Some(text.galley.job.sections[0].format.clone()),
            _ => None,
        });
    }
    found.expect("the legend has its title")
}

/// The 「B」 and 「I」 of the legend set its title as the C++ does (`KaDrawingStudio.cpp:2368-2369, 2372`): bold
/// while 「B」 is on, which it is at first, and slanted while 「I」 is on.
#[test]
fn bold_and_italic_set_the_title_of_the_legend() {
    let mut app = StrataAppState::new_startup();
    let first = title_format(&mut app);
    assert_eq!((first.font_id.family, first.italics), (FontFamily::Name(LABEL_FONT_FAMILY.into()), false));
    app.drawing.sheet.legend_bold = false;
    app.drawing.sheet.legend_italic = true;
    let changed = title_format(&mut app);
    assert_eq!((changed.font_id.family, changed.italics), (FontFamily::Proportional, true));
}
