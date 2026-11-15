//! 「국토지리원 DEM 불러오기(.img)…」 (`target/cpp-ref/src/app/MainWindow.cpp:1686-1705`
//! importDemElevationRaster, `src/core/BasemapOps.cpp:1996-2037` addDemElevationRaster): a DEM file the user
//! picks takes the place of the DEM on the map, with its relief. The file stays where the user has it.

use std::path::Path;

use super::DEM_KEY;
use crate::app_state::{MainTab, StrataAppState};

#[cfg(test)]
#[path = "dem_import_tests.rs"]
mod tests;

/// `MainWindow.cpp:1702`.
const IMPORTED: &str = "국토지리원 DEM을 올렸습니다. 범례에 높이(m)가 표시됩니다.";
/// `BasemapOps.cpp:2004`.
const NO_FILE: &str = "DEM 파일이 없습니다.";
/// A new DEM narrower and lower than this is zoomed to (`BasemapOps.cpp:2030-2032`), metres.
const ZOOM_BELOW_M: f64 = 20_000.0;

impl StrataAppState {
    /// The menu row: the file dialog of the C++ (`MainWindow.cpp:1689-1691`), then the import.
    pub(crate) fn pick_dem_file(&mut self) {
        let picked = rfd::FileDialog::new().set_title("국토지리원 DEM").add_filter("국토지리원 DEM", &["img", "IMG", "tif", "tiff", "TIF", "TIFF"]).pick_file();
        if let Some(file) = picked {
            self.import_dem_file(&file, |file| strata_engine::check_elevation_file(file).map_err(|e| e.0));
        }
    }

    /// The file becomes the DEM of the map when it is there and `check` (GDAL) takes it as a DEM; else the
    /// status line says why and nothing changes.
    pub(crate) fn import_dem_file(&mut self, file: &Path, check: impl Fn(&Path) -> Result<(), String>) {
        if !file.is_file() {
            return self.show_status_message(NO_FILE);
        }
        if let Err(reason) = check(file) {
            // `BasemapOps.cpp:2010-2011`.
            return self.show_status_message(format!("국토지리원 DEM(.img)을 열 수 없습니다: {reason}"));
        }
        self.active_tab = MainTab::Map;
        self.show_dem_layers(file);
        self.dem_zoom_wanted = true;
        self.show_status_message(IMPORTED);
    }

    /// Every frame: once a newly imported DEM is read, the map goes to it if it is a small one.
    pub(crate) fn zoom_to_new_dem(&mut self) {
        let (true, Some(extent)) = (self.dem_zoom_wanted, self.ref_vectors.extent_of(DEM_KEY)) else {
            return;
        };
        self.dem_zoom_wanted = false;
        let (across, down) = (extent[2] - extent[0], extent[3] - extent[1]);
        if across > 0.0 && across < ZOOM_BELOW_M && down < ZOOM_BELOW_M {
            self.home_map.zoom_to_layer_max(Some(extent), true, self.work_crs);
        }
    }
}
