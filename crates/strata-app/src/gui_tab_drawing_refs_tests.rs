//! The map frame of a sheet as a view of the map, for the reference layers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, epaint::Shape, pos2, vec2, RawInput, Rect};

use super::*;
use crate::app_state::StrataAppState;
use crate::gui_tab_map::w2s;
use crate::ref_vector::map::tests::{draw_survey_area, CENTRE};
use crate::survey_style::mm_to_points;

/// A frame at 1:5,000 shown at 4 points for a millimetre of paper: 0.8 points for a metre. The middle of the
/// frame is the middle of the view, north is up, and a millimetre of a line or a letter is one of paper.
/// The scale the labels are judged by (names of sites closer than 1:10,000) is the scale of the sheet,
/// however large the screen shows it.
#[test]
fn the_map_frame_of_a_sheet_is_a_view_of_the_map() {
    let rect = Rect::from_min_size(pos2(300.0, 120.0), vec2(640.0, 480.0));
    let (px_per_mm, scale) = (4.0_f32, 5000.0_f64);
    let px_per_meter = f64::from(px_per_mm) * 1000.0 / scale;
    let (vp, zoom) = sheet_reference_view(rect, [220_000.0, 370_000.0], px_per_meter, px_per_mm);

    assert_eq!(w2s(rect, &vp, [220_000.0, 370_000.0]), rect.center());
    let east = w2s(rect, &vp, [220_100.0, 370_000.0]);
    assert!((east.x - rect.center().x - 80.0).abs() < 1e-3 && (east.y - rect.center().y).abs() < 1e-3, "{east:?}");
    let north = w2s(rect, &vp, [220_000.0, 370_100.0]);
    assert!((rect.center().y - north.y - 80.0).abs() < 1e-3, "north is up: {north:?}");
    assert!((zoom - px_per_mm / mm_to_points(1.0)).abs() < 1e-6);

    let asked = crate::ref_vector::labels::engine_view(rect, &vp, zoom);
    let screen_point_m = 0.0254 / 96.0;
    assert!((asked.metres_per_point / screen_point_m / scale - 1.0).abs() < 1e-4, "the scale of the sheet: {}", asked.metres_per_point / screen_point_m);
    assert!((asked.width_pt - 640.0 / f64::from(zoom)).abs() < 1e-6 && (asked.height_pt - 480.0 / f64::from(zoom)).abs() < 1e-6, "the paper at its own size");
    assert_eq!(asked.centre, [220_000.0, 370_000.0]);

    // On the map nothing is zoomed.
    let on_map = crate::ref_vector::labels::engine_view(rect, &vp, 1.0);
    assert_eq!((on_map.metres_per_point, on_map.width_pt, on_map.height_pt), (vp.meters_per_pixel, 640.0, 480.0));
}

/// On a sheet the names of the heritage sites are not written by their layers: the numbers stand for them
/// (`HeritageLayoutNumbers.cpp:744, 963-965` switches the labels of the sheet's copies off). The letters of
/// the other reference layers (soil, geology, rivers) stay.
#[test]
fn a_sheet_writes_no_names_for_the_heritage_rows() {
    assert!(!sheet_writes_labels_of("ref_heritage:designated_heritage:국가지정유산"));
    assert!(!sheet_writes_labels_of("ref_heritage:excavation_survey_area:발굴유적위치도"));
    assert!(sheet_writes_labels_of("ref_soil"));
    assert!(sheet_writes_labels_of("ref_geology"));
}

/// The survey's own layers are drawn in the map frame as they are on the map, at the scale of the sheet: a
/// survey area of 100 m at 1:5,000, shown at 4 points for a millimetre of paper, is 80 points wide in the
/// middle of the frame, and its outline has its millimetres of paper.
#[test]
fn the_survey_area_is_drawn_in_the_map_frame_at_the_size_of_the_paper() {
    let mut app = StrataAppState::new_startup();
    draw_survey_area(&mut app, CENTRE, 5187);
    let rect = Rect::from_min_size(pos2(300.0, 120.0), vec2(640.0, 480.0));
    let (px_per_mm, scale) = (4.0_f32, 5000.0_f64);
    let (view, zoom) = sheet_reference_view(rect, CENTRE, f64::from(px_per_mm) * 1000.0 / scale, px_per_mm);
    let ctx = egui::Context::default();
    let out = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))), ..RawInput::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_survey_tables_in(ctx, &ui.painter_at(rect), rect, &view, zoom));
    });
    let outlines: Vec<(Rect, f32)> = out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Path(path) if path.closed => Some((Rect::from_points(&path.points), path.stroke.width)),
            _ => None,
        })
        .collect();
    assert_eq!(outlines.len(), 1, "the outline of the survey area");
    let (outline, width) = outlines[0];
    assert!((outline.width() - 80.0).abs() < 0.01 && (outline.height() - 80.0).abs() < 0.01, "{outline:?}");
    assert!((outline.center() - rect.center()).length() < 0.01, "{outline:?}");
    let on_map = mm_to_points(crate::survey_paint_layers::base_style_for_table("survey_area").stroke_width_mm);
    assert!((width - on_map * zoom).abs() < 1e-3, "{width} against {on_map} times {zoom}");
}
