//! Pictures of the user on the map — C++ `addRasterFromPath` (MainWindow.cpp:2884-2937), `looksUnreferencedRaster`
//! (GeorefService.cpp:612-631), `knockOutRasterPaper` (BasemapOps.cpp:193-215).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::DriverManager;
use strata_engine::PictureInfo;
use strata_types::crs::WorkCrs;
use strata_types::schema::LegendGroup;

use super::{knock_out_paper, looks_unreferenced, PictureLayer, Wanted, UNREFERENCED};
use crate::app_state::StrataAppState;

/// A picture of `width` × `height` grey pixels written by GDAL as `driver` (GTiff, JPEG, PNG), placed by
/// `transform` in `epsg` when given.
fn picture(dir: &Path, name: &str, driver: &str, size: [usize; 2], transform: Option<[f64; 6]>, epsg: Option<u32>) -> PathBuf {
    let mut memory = DriverManager::get_driver_by_name("MEM").unwrap().create_with_band_type::<u8, _>("", size[0], size[1], 3).unwrap();
    for band in 1..=3 {
        memory.rasterband(band).unwrap().write((0, 0), (size[0], size[1]), &mut Buffer::new((size[0], size[1]), vec![90_u8; size[0] * size[1]])).unwrap();
    }
    if let Some(t) = transform {
        memory.set_geo_transform(&t).unwrap();
    }
    if let Some(code) = epsg {
        memory.set_spatial_ref(&SpatialRef::from_epsg(code).unwrap()).unwrap();
    }
    let path = dir.join(name);
    memory.create_copy(&DriverManager::get_driver_by_name(driver).unwrap(), &path, &Default::default()).unwrap();
    path
}

fn survey(dir: &Path) -> (StrataAppState, PathBuf) {
    let path = dir.join("현장.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("현장", &path, WorkCrs::Epsg5187);
    (app, path)
}

fn info(transform: Option<[f64; 6]>, size: [usize; 2]) -> PictureInfo {
    PictureInfo { width: size[0], height: size[1], bands: 3, transform, epsg: None }
}

/// The C++ rule of a picture without coordinates: no transform, or one that keeps it on its pixel plane, or a box
/// of about one unit a pixel near the pixel origin; 0.5 m or 1 m pixels in the work CRS are placed.
#[test]
fn a_picture_on_its_pixel_plane_has_no_coordinates() {
    assert!(looks_unreferenced(&info(None, [800, 600])));
    assert!(looks_unreferenced(&info(Some([0.0, 1.0, 0.0, 0.0, 0.0, 1.0]), [800, 600])));
    assert!(looks_unreferenced(&info(Some([0.0, 1.0, 0.0, 600.0, 0.0, -1.0]), [800, 600])));
    assert!(!looks_unreferenced(&info(Some([219_000.0, 0.5, 0.0, 360_400.0, 0.0, -0.5]), [800, 600])));
    assert!(!looks_unreferenced(&info(Some([200_000.0, 1.0, 0.0, 600_000.0, 0.0, -1.0]), [800, 600])));
}

/// A view needs the part of the picture under it with half its size around it, at the screen's fineness and
/// never finer than the file; a picture away from the view needs nothing.
#[test]
fn a_view_wants_the_part_under_it_at_screen_fineness() {
    // 1000 × 800 pixels of 0.5 m: x 219 000 … 219 500, y 360 000 … 360 400.
    let layer = PictureLayer::new("picture:평면도".to_owned(), PathBuf::from("평면도.tif"), &info(None, [1000, 800]), [219_000.0, 0.5, 0.0, 360_400.0, 0.0, -0.5], None, false, false);
    let view = [219_100.0, 360_100.0, 219_200.0, 360_200.0];
    assert_eq!(layer.wanted(view, 0.25, 5187), Some(Wanted { window: [100, 300, 400, 400], size: [400, 400] }));
    assert_eq!(layer.wanted(view, 2.0, 5187), Some(Wanted { window: [100, 300, 400, 400], size: [100, 100] }));
    assert_eq!(layer.wanted([300_000.0, 400_000.0, 300_100.0, 400_100.0], 0.25, 5187), None);
}

/// Paper white — red, green and blue within 16 of 255 — becomes see-through; colours stay.
#[test]
fn paper_white_becomes_see_through() {
    let mut rgba = vec![255, 255, 255, 255, 240, 241, 250, 255, 238, 255, 255, 255, 10, 20, 30, 255];
    knock_out_paper(&mut rgba);
    assert_eq!(rgba, [255, 255, 255, 0, 240, 241, 250, 0, 238, 255, 255, 255, 10, 20, 30, 255]);
}

/// A GeoTIFF opened in 파일함 becomes a checked row at the top of 주변·참조, the map goes to it, the status line says
/// the C++ 「그림 추가: <이름> · <좌표계> · 화면 <작업 좌표계>」, and the next frames draw it from the file.
#[test]
fn a_geotiff_goes_on_the_map_and_is_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let tif = picture(dir.path(), "평면도.tif", "GTiff", [400, 300], Some([219_000.0, 0.5, 0.0, 360_150.0, 0.0, -0.5]), Some(5187));
    app.activate_drawer_file(&tif);
    let row = app.left_layers.entries.iter().find(|e| e.layer_key == "picture:평면도").expect("the row");
    assert_eq!((row.group, row.read_only), (LegendGroup::ReferenceMap, true));
    assert!(app.home_map.real_survey.is_layer_visible("평면도"));
    assert_eq!(app.status_left_message(), "그림 추가: 평면도 · EPSG:5187 · 화면 EPSG:5187");
    let view = app.home_map.viewport.center_xy;
    assert!((view[0] - 219_100.0).abs() < 1.0 && (view[1] - 360_075.0).abs() < 1.0, "the map goes to it: {view:?}");
    assert!(app.is_dirty);

    let ctx = egui::Context::default();
    let end = Instant::now() + Duration::from_secs(30);
    while app.pictures.layers[0].shown().is_none() && Instant::now() < end {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                app.home_map.viewport.screen_size_px = [f64::from(rect.width()), f64::from(rect.height())];
                app.paint_picture_layers(ctx, &ui.painter_at(rect), rect);
            });
        });
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.pictures.layers[0].shown().is_some_and(|wanted| wanted.window[2] > 0), "drawn from the file");
}

/// A JPEG without coordinates becomes an unchecked row that waits for 맞추기, with the C++ words (`:2916-2917`).
#[test]
fn a_picture_without_coordinates_waits_for_alignment() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let jpg = picture(dir.path(), "스캔.jpg", "JPEG", [300, 200], None, None);
    app.activate_drawer_file(&jpg);
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "picture:스캔"));
    assert!(!app.home_map.real_survey.is_layer_visible("스캔"));
    assert!(app.pictures.layers[0].align_pending);
    assert_eq!(app.status_left_message(), UNREFERENCED);
}

/// A GeoTIFF that names no CRS is taken in the work CRS (`:2896-2899`); the last words name it.
#[test]
fn a_geotiff_without_a_crs_is_taken_in_the_work_crs() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let tif = picture(dir.path(), "도면.tif", "GTiff", [200, 200], Some([219_000.0, 0.5, 0.0, 360_100.0, 0.0, -0.5]), None);
    app.activate_drawer_file(&tif);
    assert_eq!(app.pictures.layers[0].source_epsg, None);
    assert_eq!(app.status_left_message(), "그림 추가: 도면 · EPSG:5187 · 화면 EPSG:5187");
}

/// A picture row comes back when the survey is opened again, from the same file.
#[test]
fn a_picture_comes_back_when_the_survey_is_opened_again() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path());
    let tif = picture(dir.path(), "평면도.tif", "GTiff", [100, 100], Some([219_000.0, 0.5, 0.0, 360_050.0, 0.0, -0.5]), Some(5187));
    app.drop_paths(std::slice::from_ref(&tif));
    assert_eq!(app.status_left_message(), "레이어 1개 추가됨 (파일→지도)");
    assert!(app.save_survey_project());
    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&path));
    assert_eq!(opened.pictures.layers.iter().map(|l| l.file.clone()).collect::<Vec<_>>(), [tif]);
    assert!(opened.home_map.real_survey.is_layer_visible("평면도"));
}
