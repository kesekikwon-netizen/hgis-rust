//! Actual print-scene consumer, target/cpp-ref/src/app/KaDrawingStudio.cpp:3758-3785.
use super::*;

#[test]
fn legend_frame_print_renderer_retains_curved_contours_at_native_double_dpi() {
    let mut h = start();
    let before = snapshot(&h);
    let screen_items = h.app.drawing.item_rects.clone();
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &h.app.theme);
    let scale = 1.65;
    let paper = Rect::from_min_size(Pos2::ZERO, egui::vec2(210.0, 297.0) * scale);
    let mut textures = crate::gui_tab_drawing::sheet_raster::Textures::default();
    let mut output = egui::FullOutput::default();
    for frame in 0..6 {
        let mut input = egui::RawInput {
            screen_rect: Some(paper),
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        input.viewports.get_mut(&egui::ViewportId::ROOT).expect("native viewport").native_pixels_per_point = Some(2.0);
        output = ctx.run(input, |ctx| h.app.paint_sheet_for_print(ctx, paper, scale));
        textures.apply(&output.textures_delta);
    }
    assert_eq!(output.pixels_per_point, 2.0);
    assert_eq!(h.app.drawing.item_rects, screen_items, "off-screen print changed screen selection geometry");
    assert!(texts(&output).iter().any(|text| text == "범례"));
    assert!(snapshot(&h) == before, "print scene changed original records/history/bytes");
    crate::test_mods::ui_snapshot_support::save_frame_if_requested(&ctx, &output, &textures, [693, 981], "legend-frame-print-dpi2");
    rounded_geometry(&ctx, &output);
}
