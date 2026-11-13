//! Submit shapefiles (EPSG:5179) of survey GPKG layers through GDAL, the writer QGIS uses.
//! ≡ cpp `ExportService.cpp:117-181` + `SubmitShp.cpp`: layers of one key merged (`mergeLayers`),
//! SHP field names (`withShapefileFieldNames`), written as QGIS writes them, re-read (`verifyWritten`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::LayerAccess;
use gdal::Dataset;

#[path = "submit_shp_read.rs"]
mod submit_shp_read;
#[path = "submit_shp_write.rs"]
mod submit_shp_write;

use crate::submit_shp_names::field_name_for;
use crate::EngineError;
use submit_shp_read::{merge_tables, read_table, Field};
use submit_shp_write::write_table;

/// SHP text encoding (`ExportService.cpp:117-120`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitEncoding {
    Utf8,
    /// CP949.
    EucKr,
}

/// One layer the app shows under a `layer_key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitSource {
    pub gpkg: PathBuf,
    pub table: String,
    /// The layer's display name (`vl->name()`), used in the missing-CRS text.
    pub title: String,
}

/// One written SHP: README line `"{key}.shp features={features} crs=EPSG:5179 verified"` (`ExportService.cpp:181`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShpLayerReport {
    pub key: &'static str,
    pub features: u64,
}

impl From<gdal::errors::GdalError> for EngineError {
    fn from(e: gdal::errors::GdalError) -> Self {
        Self(e.to_string())
    }
}

/// `ExportService.cpp:121-123` — key order of the package.
const SUBMIT_KEYS: [&str; 7] = [
    "survey_area",
    "feature_poly",
    "feature_line",
    "control_points",
    "section_line",
    "artifact_point",
    "trial_trench",
];

/// Writes `<out_dir>/<key>.shp` (+ .shx .dbf .prj .cpg) for each key in cpp order and returns the
/// README layer reports and the field notes (merge notes, then `"{key} {from}={to}"`, `SubmitShp.cpp:149`).
/// A key whose layers have no feature writes nothing (`ExportService.cpp:147-153`).
///
/// # Errors
/// The cpp texts for a layer without CRS, a failed merge, a write failure or a failed re-read.
pub fn write_submit_shapefiles(
    sources: &[(&'static str, Vec<SubmitSource>)],
    out_dir: &Path,
    enc: SubmitEncoding,
) -> Result<(Vec<ShpLayerReport>, Vec<String>), EngineError> {
    let mut reports = Vec::new();
    let mut notes = Vec::new();
    for key in SUBMIT_KEYS {
        let mut picked: Vec<&SubmitSource> = Vec::new();
        for src in sources.iter().filter(|(k, _)| *k == key).flat_map(|(_, list)| list) {
            let ds = open(&src.gpkg)?;
            let mut layer = ds.layer_by_name(&src.table)?;
            // :147 a layer with no feature is skipped before the CRS check.
            if layer.features().next().is_none() {
                continue;
            }
            if layer.spatial_ref().is_none() {
                return Err(EngineError(format!(
                    "「{}」({key}) 레이어에 좌표계가 없어 EPSG:5179로 바꿀 수 없습니다. 레이어 좌표계를 지정한 뒤 다시 만드세요.",
                    src.title
                )));
            }
            picked.push(src);
        }
        let write_failed = |e: String| {
            let detail = if e.is_empty() { String::new() } else { format!("\n{e}") };
            EngineError(format!("{key}.shp를 쓰지 못했습니다. 저장 위치의 쓰기 권한과 남은 공간을 확인하세요.{detail}"))
        };
        // :158-161 merge only when there are several layers.
        let table = match picked.as_slice() {
            [] => continue,
            [one] => read_table(key, one).map_err(write_failed)?,
            more => merge_tables(key, more, &mut notes).map_err(EngineError)?,
        };
        let names = field_names(key, &table.fields, &mut notes);
        let shp = out_dir.join(format!("{key}.shp"));
        write_table(key, &table, &names, &shp, enc).map_err(write_failed)?;
        let features = table.rows.len() as u64;
        verify_written(&shp, features)?;
        reports.push(ShpLayerReport { key, features });
    }
    Ok((reports, notes))
}

fn open(path: &Path) -> Result<Dataset, EngineError> {
    Dataset::open(path).map_err(|e| EngineError(format!("{}: {e}", path.display())))
}

/// SHP names for the fields; renamed only when some name passes 10 bytes (SubmitShp.cpp:136-150).
fn field_names(key: &str, fields: &[Field], notes: &mut Vec<String>) -> Vec<String> {
    if fields.iter().all(|f| f.name.len() <= 10) {
        return fields.iter().map(|f| f.name.clone()).collect();
    }
    let mut used = HashSet::new();
    fields
        .iter()
        .map(|f| {
            let mapped = field_name_for(&f.name, &mut used);
            if mapped != f.name {
                notes.push(format!("{key} {}={mapped}", f.name));
            }
            mapped
        })
        .collect()
}

/// `verifyWritten` (SubmitShp.cpp:176-215) — re-open, EPSG:5179, feature count.
pub(crate) fn verify_written(shp: &Path, expected: u64) -> Result<(), EngineError> {
    let name = shp.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let ds = Dataset::open(shp).map_err(|_| EngineError(format!("방금 쓴 {name}을(를) 다시 열지 못했습니다.")))?;
    let layer = ds.layer(0).ok();
    let count = layer.as_ref().map(LayerAccess::feature_count);
    let is_5179 = layer.as_ref().and_then(LayerAccess::spatial_ref).is_some_and(|srs| {
        let named = |s: &SpatialRef| s.auth_name().as_deref() == Some("EPSG") && s.auth_code().ok() == Some(5179);
        // Same identification fallback as the QGIS OGR provider for an ESRI .prj (SubmitShp.cpp:193-196).
        named(&srs) || {
            let mut probe = srs.clone();
            probe.auto_identify_epsg().is_ok() && named(&probe)
        }
    });
    if !is_5179 {
        return Err(EngineError(format!("{name}의 좌표계가 EPSG:5179로 기록되지 않았습니다.")));
    }
    match count {
        Some(n) if n == expected => Ok(()),
        n => Err(EngineError(format!(
            "{name}에 도형이 {expected}개 기록돼야 하는데 {}개입니다.",
            n.map_or(-1, |v| i64::try_from(v).unwrap_or(i64::MAX))
        ))),
    }
}

/// Points the PROJ that GDAL uses at `dirs` (each holding proj.db) through the GDAL configuration
/// option `PROJ_DATA` (a GDAL setting, not a process environment variable).
/// The caller passes the folder it ships.
///
/// # Errors
/// A folder whose path is not valid UTF-8, or GDAL refusing the option.
pub fn set_gdal_proj_search_paths(dirs: &[&Path]) -> Result<(), EngineError> {
    let joined = dirs
        .iter()
        .map(|d| d.to_str().ok_or_else(|| EngineError(format!("{}: not a UTF-8 path", d.display()))))
        .collect::<Result<Vec<&str>, EngineError>>()?
        .join(";");
    gdal::config::set_config_option("PROJ_DATA", &joined)?;
    Ok(())
}

#[cfg(test)]
#[path = "submit_shp_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "submit_shp_types_tests.rs"]
mod types_tests;
