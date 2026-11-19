//! F1d-1 — title-block date line formats the given date as yyyy-MM-dd instead of a fixed sample date
//! (target/cpp-ref/src/app/KaDrawingStudioSheets.cpp:60, target/cpp-ref/src/app/KaTitleBlock.cpp:62-64;
//! expected values ported from target/cpp-ref/tests/test_drawing_studio.cpp:216-225).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::NaiveDate;
use eframe::egui::{self, FontFamily, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;
use crate::gui_theme_fonts::LAYOUT_FONT_FAMILY;

#[test]
fn title_block_date_line_formats_given_date_as_yyyy_mm_dd() {
    let mut app = StrataAppState::new_startup();
    app.drawing.title_ko = "유구배치도".to_owned();
    app.home_map.real_survey.survey_name = "왕복조사".to_owned();
    app.home_map.real_survey.site_name = "테스트유적".to_owned();
    app.drawing.sheet.scale_denominator = 1000;

    let date = NaiveDate::from_ymd_opt(2026, 9, 29).expect("valid date");
    let lines = app.sheet_title_block_lines_on(date);

    assert_eq!(lines[0], "도면명   유구배치도");
    assert_eq!(lines[1], "조사명   왕복조사");
    assert_eq!(lines[2], "유적명   테스트유적");
    assert_eq!(lines[3], "축  척   1 : 1000");
    assert_eq!(lines[4], "작성일   2026-09-29");
}

/// The title block uses Malgun Gothic when that face is registered (`LayoutService.cpp:394`).
#[test]
fn the_title_block_uses_malgun_gothic_when_the_face_is_registered() {
    let mut app = StrataAppState::new_startup();
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.insert(FontFamily::Name(LAYOUT_FONT_FAMILY.into()), vec!["Ubuntu-Light".to_owned()]);
    ctx.set_fonts(fonts);
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))), ..Default::default() };
    let mut family = None;
    for _ in 0..2 {
        let out = ctx.run(raw.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.paint_drawing_sheet(ui));
        });
        family = out.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if text.galley.text().starts_with("도면명") => Some(text.galley.job.sections[0].format.font_id.family.clone()),
            _ => None,
        });
    }
    assert_eq!(family, Some(FontFamily::Name(LAYOUT_FONT_FAMILY.into())));
}
