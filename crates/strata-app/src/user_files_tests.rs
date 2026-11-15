//! Files of the user put on the map — C++ `tryAddDroppedPaths` (MainWindowFiles.cpp:260-289), the 파일함's
//! `fileActivated` (MainWindow.cpp:902-914), `openVectorLayer` (:2592-2636) and `addVectorFromPath`
//! (:2939-3033); MORE0 To build #4.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Feature, Geometry, LayerAccess, LayerOptions, OGRwkbGeometryType};
use gdal::DriverManager;
use strata_types::crs::WorkCrs;
use strata_types::schema::LegendGroup;

use crate::app_state::app_state_save_before::SaveBeforeAnswer;
use crate::app_state::StrataAppState;
use crate::layer_menu_model::LayerMenuKind;
use crate::ref_vector::data::{Look, PolygonFill};

/// Near the middle of Gyeongju, in EPSG:5186 and in EPSG:5187 (the work CRS of the test survey).
const IN_5186: [f64; 2] = [400_500.0, 360_500.0];
const IN_5187: [f64; 2] = [219_800.0, 360_300.0];

/// A shapefile of `count` squares (or lines, or points) 100 m wide, 200 m apart, from `at`, in `epsg`
/// (none: no `.prj`).
fn shapefile(dir: &Path, stem: &str, ty: OGRwkbGeometryType::Type, epsg: Option<u32>, at: [f64; 2], count: usize) -> PathBuf {
    let path = dir.join(format!("{stem}.shp"));
    let mut file = DriverManager::get_driver_by_name("ESRI Shapefile").unwrap().create_vector_only(&path).unwrap();
    let crs = epsg.map(|code| SpatialRef::from_epsg(code).unwrap());
    let layer = file.create_layer(LayerOptions { name: stem, srs: crs.as_ref(), ty, options: None }).unwrap();
    for i in 0..count {
        let (x, y) = (at[0] + 200.0 * i as f64, at[1]);
        let wkt = match ty {
            OGRwkbGeometryType::wkbPolygon => {
                format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))", x - 50.0, y - 50.0, x + 50.0, y - 50.0, x + 50.0, y + 50.0, x - 50.0, y + 50.0, x - 50.0, y - 50.0)
            }
            OGRwkbGeometryType::wkbLineString => format!("LINESTRING ({} {}, {} {})", x - 50.0, y, x + 50.0, y),
            _ => format!("POINT ({x} {y})"),
        };
        let mut feature = Feature::new(layer.defn()).unwrap();
        feature.set_geometry(Geometry::from_wkt(&wkt).unwrap()).unwrap();
        feature.create(&layer).unwrap();
    }
    drop(file);
    path
}

fn survey(dir: &Path) -> (StrataAppState, PathBuf) {
    let path = dir.join("현장.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("현장", &path, WorkCrs::Epsg5187);
    (app, path)
}

fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

/// What the row draws, read as the map reads it: the number of shapes and their box in the work CRS.
fn drawn(app: &StrataAppState, key: &str) -> (usize, [f64; 4]) {
    let data = crate::ref_vector::data::load(app.ref_vectors.spec_of(key).expect("the row's layer")).expect("the layer reads");
    (data.items.len(), data.bbox)
}

fn near(a: f64, b: f64, by: f64) -> bool {
    (a - b).abs() < by
}

/// A shapefile of polygons dropped on the window becomes a row of 조사 데이터 named after the file, drawn in
/// black lines of 0.2 mm without fill, read where it lies (nothing is written beside it) and carried into
/// the work CRS; the map goes to it, and the status line says what the C++ says.
#[test]
fn a_dropped_shapefile_goes_on_the_map() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let files = dir.path().join("받은 자료");
    std::fs::create_dir_all(&files).unwrap();
    let shp = shapefile(&files, "시굴 범위", OGRwkbGeometryType::wkbPolygon, Some(5186), IN_5186, 3);
    let before = listing(&files);
    app.drop_paths(std::slice::from_ref(&shp));
    let row = app.left_layers.entries.iter().find(|e| e.layer_key == "user:시굴 범위").expect("the row");
    assert_eq!((row.label_ko.as_str(), row.group, row.read_only), ("시굴 범위", LegendGroup::SurveyData, true));
    let spec = app.ref_vectors.spec_of("user:시굴 범위").unwrap();
    assert_eq!(spec.file, shp);
    assert_eq!(spec.look, Look::Polygons { stroke: [0, 0, 0, 255], width_mm: 0.2, fill: PolygonFill::None });
    let (items, bbox) = drawn(&app, "user:시굴 범위");
    assert_eq!(items, 3);
    // The three squares span [x-50, y-50, x+450, y+50] in 5186; the zones turn by about 1.2° against each other.
    let source = [IN_5186[0] - 50.0, IN_5186[1] - 50.0, IN_5186[0] + 450.0, IN_5186[1] + 50.0];
    let carried = strata_engine::Transformer::new(5186, 5187).unwrap().transform_box(source).unwrap();
    assert!((0..4).all(|i| near(bbox[i], carried[i], 0.5)), "{bbox:?} against {carried:?}");
    let view = app.home_map.viewport.center_xy;
    assert!(view[0] > bbox[0] && view[0] < bbox[2] && view[1] > bbox[1] && view[1] < bbox[3], "the map goes to it: {view:?}");
    assert_eq!(app.status_left_message(), "레이어 1개 추가됨 (파일→지도)");
    assert!(app.is_dirty);
    assert_eq!(listing(&files), before, "nothing is written beside the user's file");
}

/// 「벡터·도면 불러오기」 with a shapefile that names no CRS: it is read in the work CRS, as the C++ attaches the
/// work CRS (`:2618-2624`), and the status line says 「지도를 올렸습니다: <name>」 (`:2635`).
#[test]
fn a_picked_shapefile_without_a_crs_is_read_in_the_work_crs() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let shp = shapefile(dir.path(), "도면선", OGRwkbGeometryType::wkbLineString, None, IN_5187, 2);
    assert!(!shp.with_extension("prj").exists());
    app.open_vector_file(&shp);
    let (items, bbox) = drawn(&app, "user:도면선");
    assert_eq!(items, 2);
    assert!(near(bbox[0], IN_5187[0] - 50.0, 0.01) && near(bbox[1], IN_5187[1], 0.01), "not carried: {bbox:?}");
    assert_eq!(app.status_left_message(), "지도를 올렸습니다: 도면선");
}

/// What is not carried over yet is refused in words and adds nothing: a shapefile of points, the tables of a
/// GeoPackage that is no survey, GeoJSON and CAD drawings; a picture GDAL cannot open adds nothing either. A
/// broken shapefile picked with 「벡터·도면 불러오기」 says the C++ 「파일을 열지 못했습니다」 (`:2603`).
#[test]
fn files_not_carried_over_yet_are_refused_in_words() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let rows = app.left_layers.entries.len();
    let points = shapefile(dir.path(), "유물점", OGRwkbGeometryType::wkbPoint, Some(5187), IN_5187, 2);
    app.drop_paths(&[points]);
    assert_eq!(app.status_left_message(), "「유물점」은(는) 점 레이어라 아직 지도에 올리지 못합니다.");
    let other = dir.path().join("참고.gpkg");
    let mut gpkg = DriverManager::get_driver_by_name("GPKG").unwrap().create_vector_only(&other).unwrap();
    gpkg.create_layer(LayerOptions { name: "도로", srs: Some(&SpatialRef::from_epsg(5187).unwrap()), ty: OGRwkbGeometryType::wkbLineString, options: None }).unwrap();
    drop(gpkg);
    app.drop_paths(&[other]);
    assert_eq!(app.status_left_message(), "GPKG 파일의 표는 아직 지도에 올리지 못합니다: 참고.gpkg");
    for (name, words) in [
        ("a.geojson", "GeoJSON 파일은 아직 지도에 올리지 못합니다: a.geojson"),
        ("b.dxf", "CAD 도면(DXF·DWG)은 아직 지도에 올리지 못합니다: b.dxf"),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, b"x").unwrap();
        app.drop_paths(&[path]);
        assert_eq!(app.status_left_message(), words);
    }
    let broken_picture = dir.path().join("c.tif");
    std::fs::write(&broken_picture, b"x").unwrap();
    app.drop_paths(&[broken_picture]);
    assert_eq!(app.left_layers.entries.len(), rows, "no row");
    assert!(!app.is_dirty, "nothing added, nothing unsaved");
    let broken = dir.path().join("깨짐.shp");
    std::fs::write(&broken, b"no shapefile").unwrap();
    app.open_vector_file(&broken);
    assert_eq!(app.status_left_message(), "파일을 열지 못했습니다: 깨짐.shp");
    assert_eq!(app.left_layers.entries.len(), rows);
}

/// A survey file dropped on the window is opened as a survey (`addVectorFromPath` → `openSurveyGpkg`,
/// :2982-2993), asking first when work is unsaved.
#[test]
fn a_dropped_survey_file_is_opened_as_a_survey() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let other = dir.path().join("다른 조사.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(&other, WorkCrs::Epsg5187).unwrap();
    app.is_dirty = true;
    app.drop_paths(std::slice::from_ref(&other));
    assert!(app.save_before.is_some(), "unsaved work: it asks first");
    app.answer_save_before(SaveBeforeAnswer::Discard);
    assert_eq!(app.survey_path.as_deref(), Some(other.as_path()));
}

/// A file opened in 파일함 goes on the map with the words of `addVectorFromPath` (:3027-3031); a file that can
/// be no layer says the C++ words of the drawer (MainWindow.cpp:905-912).
#[test]
fn the_drawer_puts_a_file_on_the_map_or_says_why_not() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let shp = shapefile(dir.path(), "구획", OGRwkbGeometryType::wkbPolygon, Some(5187), IN_5187, 2);
    app.activate_drawer_file(&shp);
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "user:구획"));
    assert_eq!(app.status_left_message(), "레이어 1개 추가 · 도형 2개 · 화면 EPSG:5187");
    let text = dir.path().join("메모.txt");
    std::fs::write(&text, b"x").unwrap();
    app.activate_drawer_file(&text);
    assert_eq!(app.status_left_message(), "선택한 파일을 지도 레이어로 열지 못했습니다. SHP/DXF/DWG/GPKG/GeoTIFF/JPG만 지도에 올릴 수 있습니다.");
}

/// Two files of one name are two rows: the second is named apart, so that each keeps its own check mark.
#[test]
fn two_files_of_one_name_are_two_rows() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let (a, b) = (dir.path().join("가"), dir.path().join("나"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let first = shapefile(&a, "구획", OGRwkbGeometryType::wkbPolygon, Some(5187), IN_5187, 1);
    let second = shapefile(&b, "구획", OGRwkbGeometryType::wkbPolygon, Some(5187), IN_5187, 2);
    app.drop_paths(&[first.clone(), second.clone()]);
    assert_eq!(app.status_left_message(), "레이어 2개 추가됨 (파일→지도)");
    assert_eq!(app.ref_vectors.spec_of("user:구획").unwrap().file, first);
    assert_eq!(app.ref_vectors.spec_of("user:구획 (2)").unwrap().file, second);
    assert!(app.left_layers.entries.iter().any(|e| e.label_ko == "구획 (2)"));
}

/// A user row is no survey table: saving writes no table for it, 편집 from the legend does not start on it,
/// its legend menu is that of an outside vector file, and Delete in the legend takes it off; it comes back
/// when the survey is opened again, drawing the same file.
#[test]
fn a_user_row_is_no_survey_table_and_comes_back_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path());
    let files = tempfile::tempdir().unwrap();
    let shp = shapefile(files.path(), "구획", OGRwkbGeometryType::wkbPolygon, Some(5187), IN_5187, 2);
    app.drop_paths(std::slice::from_ref(&shp));
    app.start_edit_from_legend("구획");
    assert_eq!(app.edit_layer_key, None);
    assert_eq!(app.layer_menu_input("user:구획").map(|input| input.kind), Some(LayerMenuKind::ExternalVector));
    assert!(app.save_survey_project());
    let tables = strata_storage::survey_gpkg_read::read_survey_tables(&path).unwrap();
    assert!(tables.iter().all(|t| !t.table.starts_with("user")), "{:?}", tables.iter().map(|t| &t.table).collect::<Vec<_>>());

    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&path));
    assert_eq!(opened.ref_vectors.spec_of("user:구획").map(|spec| spec.file.clone()), Some(shp));
    assert_eq!(drawn(&opened, "user:구획").0, 2);
    assert!(!opened.is_dirty);

    assert!(app.remove_layers(&["user:구획".to_owned()]));
    assert!(app.left_layers.entries.iter().all(|e| e.layer_key != "user:구획"));
}

/// The legend row shows the look of the layer, a black outline without fill (`applySimpleVectorStyle`), not
/// the blue of a survey table, and no 「글자」 switch: the names of a user file are not written yet.
#[test]
fn the_legend_row_shows_the_black_outline_and_no_label_switch() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path());
    let shp = shapefile(dir.path(), "구획", OGRwkbGeometryType::wkbPolygon, Some(5187), IN_5187, 1);
    app.drop_paths(&[shp]);
    let rows = crate::gui_layer_panel::gui_layer_panel_row_spec::layer_rows(&app.left_layers, eframe::egui::Color32::BLACK, eframe::egui::Color32::BLACK);
    let row = rows.iter().find(|row| row.key == "user:구획").expect("the row");
    let black = eframe::egui::Color32::from_rgb(0, 0, 0);
    assert!(matches!(row.swatch, crate::gui_layer_panel::gui_layer_panel_paint::SwatchKind::DomainFill { fill, stroke, stroke_mm } if fill == eframe::egui::Color32::TRANSPARENT && stroke == black && (stroke_mm - 0.2).abs() < 1e-6));
    assert_eq!(row.right_label, None);
}
