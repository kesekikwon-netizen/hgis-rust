//! 「국토지리원 DEM 불러오기(.img)…」: a DEM file of the user becomes the DEM of the map.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use strata_engine::ElevationGrid;

use super::*;
use crate::ref_vector::data::{HeightRamp, LoadSpec, Look};
use crate::ref_vector::elevation::{load, Drawn};
use crate::topo_download::dem::{DEM_KEY, RELIEF_KEY};

fn said(app: &StrataAppState) -> Option<&str> {
    app.home_map.draw_guide_message.as_deref()
}

fn dem_rows(app: &StrataAppState) -> Vec<&str> {
    app.left_layers.entries.iter().filter(|e| e.layer_key.starts_with("ref_dem")).map(|e| e.layer_key.as_str()).collect()
}

/// `MainWindow::importDemElevationRaster` and `LayerOps::addDemElevationRaster` (`MainWindow.cpp:1686-1705`,
/// `BasemapOps.cpp:1996-2037`): a file that is not there, or that GDAL does not open as a DEM, is refused
/// with the C++ words and changes nothing; a good one becomes the DEM with its relief, and the status line
/// says so. No survey needs to be open, and the file stays where the user has it.
#[test]
fn a_dem_file_of_the_user_becomes_the_dem_of_the_map() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("내 DEM.img");
    std::fs::write(&file, b"heights").unwrap();
    let mut app = StrataAppState::new_startup();

    app.import_dem_file(&dir.path().join("없음.img"), |_| Ok(()));
    assert_eq!((said(&app), dem_rows(&app).len()), (Some("DEM 파일이 없습니다."), 0));
    app.import_dem_file(&file, |_| Err("not recognized as a supported file format".to_owned()));
    assert_eq!((said(&app), dem_rows(&app).len()), (Some("국토지리원 DEM(.img)을 열 수 없습니다: not recognized as a supported file format"), 0));
    assert!(!app.dem_zoom_wanted);

    app.import_dem_file(&file, |checked| if checked == file.as_path() { Ok(()) } else { Err("another file was checked".to_owned()) });
    assert_eq!(said(&app), Some("국토지리원 DEM을 올렸습니다. 범례에 높이(m)가 표시됩니다."));
    assert_eq!(dem_rows(&app), [RELIEF_KEY, DEM_KEY]);
    assert!(app.dem_shown() && app.dem_zoom_wanted);
    for key in [DEM_KEY, RELIEF_KEY] {
        assert_eq!(app.ref_vectors.spec_of(key).unwrap().file, file, "the layer reads the file where it lies");
    }
    assert_eq!(app.dem_legend_ramp(), Some(HeightRamp::National));
}

/// A layer whose source is one file reads that file, not the list of a DEM folder (here the file is no
/// raster: the error names it).
#[test]
fn a_layer_of_one_file_reads_that_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("내 DEM.img");
    std::fs::write(&file, b"heights").unwrap();
    let spec = |source: &std::path::Path| LoadSpec { file: source.to_path_buf(), table: String::new(), work_epsg: 5187, look: Look::Heights { ramp: HeightRamp::National }, label: None };
    let of_file = load(&spec(&file)).err().unwrap();
    assert!(of_file.contains("내 DEM.img"), "{of_file}");
    let of_folder = load(&spec(dir.path())).err().unwrap();
    assert!(!of_folder.contains("내 DEM.img"), "a folder is read by its list: {of_folder}");
}

/// A new DEM narrower and lower than 20 km is zoomed to once it is read; a larger one leaves the map
/// where it is (`BasemapOps.cpp:2029-2035`).
#[test]
fn the_map_goes_to_a_small_new_dem() {
    let grid = |cell: f64| Drawn::Heights(Arc::new(ElevationGrid { left: 200_000.0, top: 400_000.0 + 10.0 * cell, cell, width: 10, height: 10, heights: vec![50.0; 100], shade: vec![181; 100] }));
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("dem.img");
    std::fs::write(&file, b"heights").unwrap();
    let mut app = StrataAppState::new_startup();
    app.import_dem_file(&file, |_| Ok(()));
    let before = app.home_map.viewport.center_xy;

    app.zoom_to_new_dem();
    assert!(app.dem_zoom_wanted && app.home_map.viewport.center_xy == before, "not read yet: it waits");
    // 30 km across: the map stays.
    app.ref_vectors.set_ready(DEM_KEY, grid(3000.0));
    app.zoom_to_new_dem();
    assert!(!app.dem_zoom_wanted && app.home_map.viewport.center_xy == before);
    // 900 m across: the map goes to its middle.
    app.import_dem_file(&file, |_| Ok(()));
    app.ref_vectors.set_ready(DEM_KEY, grid(90.0));
    app.zoom_to_new_dem();
    let centre = app.home_map.viewport.center_xy;
    assert!(!app.dem_zoom_wanted && (centre[0] - 200_450.0).abs() < 1.0 && (centre[1] - 400_450.0).abs() < 1.0, "{centre:?}");
}
