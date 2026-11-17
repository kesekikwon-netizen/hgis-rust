//! Real raster and original pixel coordinates, target/cpp-ref/tests/test_align_image_view.cpp.
use super::*;

fn view(file: &std::path::Path) -> PixelView {
    let rgb: Vec<u8> = (0..5000).flat_map(|_| [20, 170, 40]).collect();
    image::save_buffer(file, &rgb, 100, 50, image::ColorType::Rgb8).expect("temporary image");
    let (layer, _) = super::super::picture_layer_of(file, "recorded-scan".to_owned(), 5187).expect("real GDAL metadata");
    PixelView::new(&layer)
}

#[test]
fn alignment_image_positions_remain_original_pixels_after_zoom_and_pan() {
    let temp = tempfile::tempdir().expect("temporary pixels");
    let mut view = view(&temp.path().join("scan.png"));
    let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(400.0, 200.0));
    assert_eq!(view.screen(rect, [10.0, 5.0]), Pos2::new(50.0, 30.0));
    view.zoom = 2.0;
    view.centre = [70.0, 30.0];
    assert_eq!(view.screen(rect, [50.0, 25.0]), Pos2::new(50.0, 70.0));
    assert_eq!(view.pixel(rect, Pos2::new(50.0, 70.0)), [50.0, 25.0]);
}

#[test]
fn alignment_image_paints_the_real_pixels_read_by_the_shared_region_worker() {
    let temp = tempfile::tempdir().expect("temporary pixels");
    let mut view = view(&temp.path().join("scan.png"));
    let ctx = egui::Context::default();
    let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(400.0, 200.0));
    let mut textures = crate::gui_tab_drawing::sheet_raster::Textures::default();
    let mut shapes = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let output = ctx.run(egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(420.0, 240.0))), ..Default::default() }, |ctx| {
            let p = ctx.layer_painter(egui::LayerId::background());
            p.rect_filled(ctx.screen_rect(), 0.0, egui::Color32::WHITE);
            view.paint(ctx, &p, rect);
        });
        textures.apply(&output.textures_delta);
        shapes = output.shapes;
        if view.reader.shown.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let primitives = ctx.tessellate(shapes, 1.0);
    let picture = crate::gui_tab_drawing::sheet_raster::paint(&primitives, &textures, 1.0, [420, 240]);
    assert_eq!(picture.at([210, 110]), [20, 170, 40], "source pane is blank or fabricated instead of the actual file");
    assert_eq!(picture.unknown_pictures, 0);
}
