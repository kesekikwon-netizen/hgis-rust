//! The received maps of a survey come back when it is opened again: owner 2026-11-15 「맵을받아 레이어에
//! 추가되는맵이면 받은것을 같은개발일때 그다음에 넣을때 적용되게하라」; MORE0 M6 and To build #1 (the C++
//! restores them with the project inside the survey file, MainWindowSession.cpp:299-368).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use super::list_path;
use crate::app_state::StrataAppState;
use crate::ref_tile_layers::{ensure_row, history::MAP_1919, TERRAIN_MAP};
use crate::ref_vector::cadastral::layer::{write_kept, Style, CADASTRAL_KEY};
use crate::ref_vector::map::SOIL;
use crate::topo_download::dem::{add_to_list, DemEntry, DEM_KEY, RELIEF_KEY};
use crate::topo_download::post::{topo_spec, TOPO_KEY, TOPO_TITLE};

fn survey(dir: &Path, name: &str) -> (StrataAppState, PathBuf) {
    let path = dir.join(format!("{name}.gpkg"));
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path(name, &path, crs);
    (app, path)
}

/// A kept file of the survey folder.
fn kept(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"kept").unwrap();
    path
}

const KEYS: [&str; 6] = [CADASTRAL_KEY, TOPO_KEY, "ref_soil", RELIEF_KEY, DEM_KEY, "ref_terrain_map"];

/// The rows of received maps, top to bottom: key, name, check mark, opacity.
fn rows(app: &StrataAppState) -> Vec<(String, String, bool, f32)> {
    let visible = |name: &str| app.home_map.real_survey.is_layer_visible(name);
    let received = app.left_layers.entries.iter().filter(|e| KEYS.contains(&e.layer_key.as_str()));
    received.map(|e| (e.layer_key.clone(), e.label_ko.clone(), visible(&e.label_ko), e.opacity)).collect()
}

fn file_of(app: &StrataAppState, key: &str) -> PathBuf {
    app.ref_vectors.spec_of(key).expect("the row's layer").file.clone()
}

/// Nothing is ordered, fetched or logged in to.
fn nothing_asked(app: &StrataAppState) -> bool {
    !app.topographic_download_busy() && !app.ref_maps.is_busy() && !app.cadastral.is_busy()
}

/// The received maps on the map when the survey was saved come back when it is opened again: the same rows
/// in the same order, with the names, check marks and opacity they had, drawing the kept files, the parcels
/// with their look; nothing is ordered or fetched, and nothing is left unsaved.
#[test]
fn the_received_maps_come_back_as_they_were_when_the_survey_is_opened_again() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path(), "현장");
    let folder = dir.path();
    let topo = kept(folder.join("지형도/합침/수치지형도-합침-100.gpkg"));
    let soil = kept(folder.join("ka-hgis-reference-ab12").join(SOIL.file_name));
    let parcels = kept(folder.join("지적도/표시/abc.gpkg"));
    let red = Style { colour: [200, 30, 30], labels: false };
    write_kept(&folder.join("지적도"), &["POLYGON((0 0,1 0,1 1,0 0))".to_owned()], 5187, 5000.0, &parcels, red);
    kept(folder.join("DEM/원본/a/35901.img"));
    let sheet = DemEntry { num: "35901".to_owned(), name: "경주".to_owned(), year: "2025".to_owned(), file: "원본/a/35901.img".into() };
    add_to_list(&folder.join("DEM"), &[sheet]).unwrap();
    app.show_cadastral_layer(parcels.clone(), Some(red));
    app.show_reference_vector_layer(TOPO_KEY, TOPO_TITLE, topo_spec(topo.clone(), 5187));
    let spec = app.load_spec(&SOIL, soil.clone());
    app.show_reference_vector_layer(SOIL.key, SOIL.title, spec);
    app.show_survey_dem();
    ensure_row(&mut app.left_layers, &TERRAIN_MAP);
    // What the user did to the rows: a new name, rows moved up, a hidden row, a lighter row.
    assert!(app.rename_layer_to(TOPO_KEY, "지형도 1:5천"));
    assert!(app.move_layer_row(SOIL.key, TOPO_KEY, false));
    assert!(app.move_layer_row(TERRAIN_MAP.key, TOPO_KEY, false));
    app.home_map.real_survey.layer_visible.insert(SOIL.title.to_owned(), false);
    app.left_layers.entries.iter_mut().find(|e| e.layer_key == RELIEF_KEY).unwrap().opacity = 0.5;
    let before = rows(&app);
    assert_eq!(before.iter().map(|row| row.0.as_str()).collect::<Vec<_>>(), [CADASTRAL_KEY, "ref_soil", "ref_terrain_map", TOPO_KEY, RELIEF_KEY, DEM_KEY]);
    assert!(app.save_survey_project());
    assert_eq!(list_path(&path), folder.join("현장.strata-maps.json"));
    assert!(list_path(&path).is_file());

    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&path));
    assert_eq!(rows(&opened), before);
    assert_eq!((file_of(&opened, TOPO_KEY), file_of(&opened, SOIL.key), file_of(&opened, CADASTRAL_KEY)), (topo, soil, parcels));
    assert_eq!(file_of(&opened, DEM_KEY), folder.join("DEM"));
    assert_eq!(Style::of(opened.ref_vectors.spec_of(CADASTRAL_KEY).unwrap()), red);
    assert!(nothing_asked(&opened));
    assert!(!opened.is_dirty);
}

/// A row whose kept file is gone is left out without a word; the others come back.
#[test]
fn a_kept_map_whose_file_is_gone_is_left_out() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path(), "현장");
    let soil = kept(dir.path().join("ka-hgis-reference-ab12").join(SOIL.file_name));
    let parcels = kept(dir.path().join("지적도/표시/abc.gpkg"));
    let spec = app.load_spec(&SOIL, soil.clone());
    app.show_reference_vector_layer(SOIL.key, SOIL.title, spec);
    app.show_cadastral_layer(parcels, None);
    assert!(app.save_survey_project());
    std::fs::remove_file(&soil).unwrap();
    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&path));
    assert_eq!(rows(&opened).iter().map(|row| row.0.as_str()).collect::<Vec<_>>(), [CADASTRAL_KEY]);
    assert_eq!(Style::of(opened.ref_vectors.spec_of(CADASTRAL_KEY).unwrap()), Style::default());
    assert!(nothing_asked(&opened));
}

/// A later 「수치지형」 run in the same folder deletes the older merged file and merges every sheet again:
/// the row then draws the newest merged file.
#[test]
fn a_topographic_row_draws_the_newest_merged_file_when_its_own_was_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path(), "현장");
    let merged = dir.path().join("지형도/합침");
    let old = kept(merged.join("수치지형도-합침-100.gpkg"));
    app.show_reference_vector_layer(TOPO_KEY, TOPO_TITLE, topo_spec(old.clone(), 5187));
    assert!(app.save_survey_project());
    std::fs::remove_file(&old).unwrap();
    let new = kept(merged.join("수치지형도-합침-200.gpkg"));
    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&path));
    assert_eq!(file_of(&opened, TOPO_KEY), new);
}

/// A received map leaves the survey unsaved, as in the C++ (`setDirty(true)` after each received map,
/// MainWindowCadastral.cpp:167, MainWindowDownloads.cpp:156, MainWindowOverlay.cpp:671), so that closing
/// asks and 저장 keeps it for the next open.
#[test]
fn a_received_map_leaves_the_survey_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path(), "현장");
    assert!(!app.is_dirty);
    app.show_reference_vector_layer(TOPO_KEY, TOPO_TITLE, topo_spec(dir.path().join("t.gpkg"), 5187));
    assert!(app.is_dirty);
}

/// Surveys in one folder share the received files, but each keeps its own list; a survey saved without
/// received maps keeps none, and an older list of it is taken away.
#[test]
fn each_survey_keeps_its_own_list_and_none_without_received_maps() {
    let dir = tempfile::tempdir().unwrap();
    let topo = kept(dir.path().join("지형도/합침/수치지형도-합침-100.gpkg"));
    let (mut first, first_path) = survey(dir.path(), "가");
    first.show_reference_vector_layer(TOPO_KEY, TOPO_TITLE, topo_spec(topo, 5187));
    assert!(first.save_survey_project());
    let (mut second, second_path) = survey(dir.path(), "나");
    assert!(second.save_survey_project());
    assert!(list_path(&first_path).is_file());
    assert!(!list_path(&second_path).exists());
    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&second_path));
    assert!(rows(&opened).is_empty());

    assert!(first.remove_layers(&[TOPO_KEY.to_owned()]));
    assert!(first.save_survey_project());
    assert!(!list_path(&first_path).exists());
}

/// A row the user took away before saving stays away, also when it came with another: the relief of a DEM
/// does not come back with the DEM.
#[test]
fn a_row_taken_away_before_saving_stays_away() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path(), "현장");
    kept(dir.path().join("DEM/원본/a/35901.img"));
    let sheet = DemEntry { num: "35901".to_owned(), name: "경주".to_owned(), year: "2025".to_owned(), file: "원본/a/35901.img".into() };
    add_to_list(&dir.path().join("DEM"), &[sheet]).unwrap();
    app.show_survey_dem();
    assert!(app.remove_layers(&[RELIEF_KEY.to_owned()]));
    assert!(app.save_survey_project());
    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&path));
    assert_eq!(rows(&opened).iter().map(|row| row.0.as_str()).collect::<Vec<_>>(), [DEM_KEY]);
}

/// The survey folder copied or moved as a whole (to another drive, another PC) keeps its received maps:
/// the list names the kept files from the survey folder.
#[test]
fn the_received_maps_move_with_the_survey_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("가")).unwrap();
    let (mut app, path) = survey(&dir.path().join("가"), "현장");
    let soil = kept(dir.path().join("가/ka-hgis-reference-ab12").join(SOIL.file_name));
    let spec = app.load_spec(&SOIL, soil);
    app.show_reference_vector_layer(SOIL.key, SOIL.title, spec);
    assert!(app.save_survey_project());
    drop(app);
    std::fs::rename(dir.path().join("가"), dir.path().join("나")).unwrap();
    let moved = dir.path().join("나").join(path.file_name().unwrap());
    let mut opened = StrataAppState::new_startup();
    assert!(opened.open_survey_gpkg(&moved));
    assert_eq!(file_of(&opened, SOIL.key), dir.path().join("나/ka-hgis-reference-ab12").join(SOIL.file_name));
}

/// MainWindow.cpp:1425-1436 greys the 1919 menu and leaves the layer. Restoring without the key
/// keeps the row, and the next save still has it.
#[test]
fn restoring_the_1919_row_without_the_history_key_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = survey(dir.path(), "현장");
    app.history_gis_key_reader = String::new;
    ensure_row(&mut app.left_layers, &MAP_1919);
    assert!(app.save_survey_project());
    assert!(std::fs::read_to_string(list_path(&path)).unwrap().contains(MAP_1919.key));

    let mut opened = StrataAppState::new_startup();
    opened.history_gis_key_reader = String::new;
    assert!(opened.open_survey_gpkg(&path));
    assert!(opened.left_layers.entries.iter().any(|entry| entry.layer_key == MAP_1919.key));
    assert!(opened.history_gis_key.is_empty());
    assert!(opened.save_survey_project());
    let again = std::fs::read_to_string(list_path(&path)).unwrap();
    assert!(again.contains(MAP_1919.key), "{again}");
}
