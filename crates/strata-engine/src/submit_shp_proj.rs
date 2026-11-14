//! The proj.db GDAL uses for the submit files. The `.prj` text follows the proj.db version: the bundled PROJ's own
//! database writes `GEOGCS["GCS_KGD2002"…`, the cpp packages hold `GEOGCS["KGD2002"…` — PROJ 9.8.1's database
//! (OSGeo4W proj-runtime-data-9.8.1-1, EPSG v12.029, ESRI ArcGIS Pro 3.6). That database ships inside the
//! program and is written to a folder on first use: one file, no install step, and the folder may hold Korean
//! letters (GDAL gets it through its `PROJ_DATA` setting in UTF-8, not through an environment variable).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::EngineError;

/// PROJ 9.8.1 proj.db, 10 280 960 bytes, sha256 37165492257d87ad504c0f7097b96f3bd504d1238bed292ecb73273ab5ae4ebf.
pub(crate) const SUBMIT_PROJ_DB: &[u8] = include_bytes!("../data/proj-9.8.1/proj.db");

/// Folder under the system temp folder the database is written to when the caller chose none.
const DEFAULT_DIR: &str = "strata-proj-9.8.1";

/// One writer at a time inside this process.
static INSTALL: Mutex<()> = Mutex::new(());

/// Writes the shipped proj.db into `dir` unless an identical copy is already there, then hands `dir` to GDAL.
///
/// # Errors
/// The folder or the file cannot be written, or GDAL refuses the setting.
pub fn install_submit_proj_db(dir: &Path) -> Result<(), EngineError> {
    let _one = INSTALL.lock().unwrap_or_else(PoisonError::into_inner);
    let db = dir.join("proj.db");
    let same = |p: &Path| std::fs::read(p).is_ok_and(|bytes| bytes == SUBMIT_PROJ_DB);
    if !same(&db) {
        let fail = |e: std::io::Error| EngineError(format!("{}: {e}", db.display()));
        std::fs::create_dir_all(dir).map_err(fail)?;
        let tmp = dir.join(format!("proj.db.{}.tmp", std::process::id()));
        std::fs::write(&tmp, SUBMIT_PROJ_DB).map_err(fail)?;
        if let Err(e) = std::fs::rename(&tmp, &db) {
            let _ = std::fs::remove_file(&tmp);
            // Another program run may have put the same file there first (and may hold it open).
            if !same(&db) {
                return Err(fail(e));
            }
        }
    }
    super::set_gdal_proj_search_paths(&[dir])
}

/// Before writing submit files: keep the caller's PROJ folder, else use the shipped database in
/// `<temp>/strata-proj-9.8.1`.
pub(crate) fn ensure_submit_proj_db() -> Result<(), EngineError> {
    if !gdal::config::get_config_option("PROJ_DATA", "")?.is_empty() {
        return Ok(());
    }
    let dir: PathBuf = std::env::temp_dir().join(DEFAULT_DIR);
    install_submit_proj_db(&dir)
}
