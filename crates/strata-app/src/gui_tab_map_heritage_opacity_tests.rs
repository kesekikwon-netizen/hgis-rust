//! Buffer rings painted on the map use the selected row's opacity.
//! C++ `MainWindow.cpp:994` sets that layer only (`LayerOps::applyLayerOpacity`).

use eframe::egui::{self, epaint::ColorMode, Color32, Rect, Shape};
use strata_types::buffer::DistanceRing;

use crate::app_state::StrataAppState;

fn paint(app: &mut StrataAppState) -> Vec<egui::epaint::ClippedShape> {
    let ctx = egui::Context::default();
    let rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 600.0));
    ctx.run(egui::RawInput { screen_rect: Some(rect), ..egui::RawInput::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.paint_gyeongju_heritage_canvas(ctx, &ui.painter_at(rect), rect, false);
        });
    })
    .shapes
}

fn path_strokes(shapes: &[egui::epaint::ClippedShape]) -> Vec<Color32> {
    let mut out = Vec::new();
    for shape in shapes {
        if let Shape::Path(path) = &shape.shape {
            if let ColorMode::Solid(color) = path.stroke.color {
                out.push(color);
            }
        }
    }
    out
}

fn ring_at(app: &StrataAppState, dx: f64, dy: f64) -> DistanceRing {
    let c = app.home_map.viewport.center_xy;
    DistanceRing {
        crs: app.work_crs,
        lines: vec![vec![[c[0] + dx, c[1] + dy], [c[0] + dx + 40.0, c[1] + dy]]],
        labels: Vec::new(),
    }
}

#[test]
fn a_buffer_ring_fades_with_its_own_row_and_the_other_ring_stays_solid() {
    let mut app = StrataAppState::new_startup();
    app.home_map.viewport.meters_per_pixel = 1.0;
    app.home_map.viewport.screen_size_px = [900.0, 600.0];
    app.install_distance_ring(500, ring_at(&app, 0.0, 0.0));
    app.install_distance_ring(1000, ring_at(&app, 0.0, 80.0));
    app.right_inspector.selected_layer_key = Some("주변 500m".to_owned());
    let background = app.home_map.basemap_opacity;
    let global = app.home_map.real_survey.layer_opacity;

    app.apply_rail_opacity("주변 500m", 0.25);

    let colors = path_strokes(&paint(&mut app));
    // 255 * 0.25 rounds to 64. egui may store that stroke premultiplied, so match the alpha.
    assert!(colors.iter().any(|c| c.a() == 64), "500 m ring did not fade: {colors:?}");
    assert!(
        colors.iter().any(|c| c.a() == 255 && (c.r(), c.g(), c.b()) == (184, 58, 24)),
        "1000 m ring changed: {colors:?}"
    );
    assert!((app.rail_opacity("주변 1000m") - 1.0).abs() < 1e-6);
    assert!((app.home_map.real_survey.layer_opacity - global).abs() < 1e-6);
    assert!((app.home_map.basemap_opacity - background).abs() < 1e-6);
}
