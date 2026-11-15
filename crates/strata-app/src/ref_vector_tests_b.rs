//! Reference vector layers where the plain way does not work: a map wider than the largest picture the
//! graphics card takes, and a picture that cannot be made (review of REF-VEC1).

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// A picture is never larger than the graphics card takes. A map wider than that is drawn coarser, not cut
/// off: the picture still reaches the margin beyond every side of the map, and it serves the next frame.
#[test]
fn a_map_wider_than_the_largest_picture_is_drawn_coarser_not_cut_off() {
    let dir = tempfile::tempdir().unwrap();
    let file = topo_file(dir.path());
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(file.clone()));
    let ctx = egui::Context::default();
    // 900 + 2 * 256 = 1412 pixels are wanted across and 1112 down; the card takes 1024.
    let input = || RawInput { screen_rect: Some(rect), max_texture_side: Some(1024), ..RawInput::default() };
    let pictures = frame_with(&mut app, &ctx, rect, input());
    assert_eq!(pictures.len(), 1);
    let drawn = pictures[0].0;
    for (got, want) in [(drawn.left(), -256.0), (drawn.top(), -256.0), (drawn.right(), 1156.0), (drawn.bottom(), 856.0)] {
        assert!((got - want).abs() < 1.5, "{drawn:?}");
    }
    let view = app.ref_vectors.picture_view(KEY).unwrap();
    assert_eq!((view.width, view.height), (1024, 807), "1412 : 1024 on both sides");
    // Lines keep their width on the screen: fewer pixels to the millimetre in the coarser picture.
    assert!((view.pixels_per_mm - 96.0 / 25.4 * 1024.0 / 1412.0).abs() < 0.001, "{}", view.pixels_per_mm);
    let _ = frame_with(&mut app, &ctx, rect, input());
    assert_eq!(app.ref_vectors.pictures_made(), 1, "the picture covers the map: no second one");

    // A tall map, in an app of its own (the wide picture above would still serve it): the longer side decides.
    let (mut app, _) = map_app();
    let tall = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 900.0));
    app.home_map.viewport.screen_size_px = [600.0, 900.0];
    app.show_reference_vector_layer(KEY, "수치지형도", spec(file));
    let pictures = frame_with(&mut app, &ctx, tall, RawInput { screen_rect: Some(tall), max_texture_side: Some(1024), ..RawInput::default() });
    let view = app.ref_vectors.picture_view(KEY).unwrap();
    assert_eq!((view.width, view.height), (807, 1024));
    assert!((pictures[0].0.bottom() - 1156.0).abs() < 1.5, "{:?}", pictures[0].0);
}

/// A picture that cannot be made for a view is asked for once, not in every frame; another view is tried.
#[test]
fn a_picture_that_cannot_be_made_is_asked_for_once_for_a_view() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    let works = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&works);
    app.ref_vectors.runner = Arc::new(move |work| {
        counted.fetch_add(1, Ordering::SeqCst);
        work();
    });
    // A view without a scale: no picture can be made for it.
    app.home_map.viewport.meters_per_pixel = f64::NAN;
    let ctx = egui::Context::default();
    for _ in 0..3 {
        assert!(frame(&mut app, &ctx, rect).is_empty());
    }
    assert_eq!(works.load(Ordering::SeqCst), 2, "the reading, and one try at the picture");
    app.home_map.viewport.meters_per_pixel = 2.0;
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1);
}

/// 「선택한 레이어로 이동」 on a reference vector row goes to the shapes of the layer, as the C++ zooms a vector
/// layer to its extent (`LayerOps::zoomToLayerMax`), not to the fixed scale it uses for pictures.
#[test]
fn zooming_to_a_reference_vector_row_shows_its_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    frame(&mut app, &egui::Context::default(), rect);
    app.home_map.viewport.center_xy = [CENTRE[0] + 50_000.0, CENTRE[1]];
    app.right_inspector.selected_layer_key = Some("수치지형도".to_owned());
    app.zoom_to_current_legend_layer();
    // The three lines lie between x 219 900 and 220 200, y 369 900 and 370 100.
    let (centre, scale) = (app.home_map.viewport.center_xy, app.home_map.viewport.meters_per_pixel);
    assert!((centre[0] - 220_050.0).abs() < 1.0 && (centre[1] - 370_000.0).abs() < 1.0, "{centre:?}");
    assert!(scale > 0.3 && scale < 0.5, "300 m and a margin across 900 points: {scale}");
}

/// QGIS carries every layer into the project CRS as it draws (C++ `ensureOtfEnabled`). Here a layer is
/// kept in the work CRS, so when the user changes the work CRS it is read again into the new one, and
/// stands at the same place on the ground.
#[test]
fn a_layer_follows_a_change_of_the_work_crs() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, rect) = map_app();
    app.show_reference_vector_layer(KEY, "수치지형도", spec(topo_file(dir.path())));
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, rect);
    let before = app.ref_vectors.shapes_of(KEY).unwrap();
    assert!((before.bbox[0] - 219_900.0).abs() < 0.01, "{:?}", before.bbox);

    app.work_crs = strata_types::crs::WorkCrs::Epsg5186;
    let moved = strata_engine::Transformer::new(5187, 5186).unwrap().transform([219_900.0, 369_900.0]).unwrap();
    app.home_map.viewport.center_xy = moved;
    assert_eq!(frame(&mut app, &ctx, rect).len(), 1, "drawn at its place in the new CRS");
    let after = app.ref_vectors.shapes_of(KEY).unwrap();
    assert!((after.bbox[0] - moved[0]).abs() < 0.5 && (after.bbox[1] - moved[1]).abs() < 0.5, "{:?} against {moved:?}", after.bbox);
    assert_eq!(app.ref_vectors.spec_of(KEY).unwrap().work_epsg, 5186);
}
