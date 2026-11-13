//! Submit shapefiles (EPSG:5179) of survey GPKG layers through GDAL, the writer QGIS uses.
//! ≡ cpp `ExportService.cpp:121-181` + `SubmitShp.cpp:134-215` for one source layer per key (R2-1a).
//! QGIS writer rules (proven by the R2 spike): `fid` N(20,0) first, TEXT C(254), REAL N(24,15),
//! `ENCODING=UTF-8`, features by fid, traditional GIS axis order.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gdal::spatial_ref::{AxisMappingStrategy, CoordTransform, SpatialRef};
use gdal::vector::{Feature, FieldDefn, LayerAccess, LayerOptions, OGRFieldType};
use gdal::{Dataset, DriverManager};

use crate::submit_shp_names::field_name_for;
use crate::EngineError;

/// SHP text encoding (`ExportService.cpp:117-120`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitEncoding {
    Utf8,
    /// CP949; written in R2-1b.
    EucKr,
}

/// One layer the app shows under a `layer_key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitSource {
    pub gpkg: PathBuf,
    pub table: String,
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

/// The field QGIS shows for the GPKG FID column.
const FID_FIELD: &str = "fid";

/// Writes `<out_dir>/<key>.shp` (+ .shx .dbf .prj .cpg) for each key in cpp order and returns the
/// README layer reports and the field-name notes (`"{key} {from}={to}"`, `SubmitShp.cpp:149`).
/// A key whose layers have no feature writes nothing (`ExportService.cpp:147-153`).
///
/// # Errors
/// The cpp texts for a layer without CRS, a write failure or a failed re-read; EUC-KR and several
/// layers per key until R2-1b.
pub fn write_submit_shapefiles(
    sources: &[(&'static str, Vec<SubmitSource>)],
    out_dir: &Path,
    enc: SubmitEncoding,
) -> Result<(Vec<ShpLayerReport>, Vec<String>), EngineError> {
    if enc == SubmitEncoding::EucKr {
        return Err(EngineError("EUC-KR SHP is written from R2-1b".to_owned()));
    }
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
                    src.table
                )));
            }
            picked.push(src);
        }
        let source = match picked.as_slice() {
            [] => continue,
            [one] => *one,
            more => {
                return Err(EngineError(format!("{key}: {} layers; merging lands in R2-1b", more.len())));
            }
        };
        let shp = out_dir.join(format!("{key}.shp"));
        let features = write_layer(key, source, &shp, &mut notes).map_err(|e| {
            let detail = if e.is_empty() { String::new() } else { format!("\n{e}") };
            EngineError(format!(
                "{key}.shp를 쓰지 못했습니다. 저장 위치의 쓰기 권한과 남은 공간을 확인하세요.{detail}"
            ))
        })?;
        verify_written(&shp, features)?;
        reports.push(ShpLayerReport { key, features });
    }
    Ok((reports, notes))
}

fn open(path: &Path) -> Result<Dataset, EngineError> {
    Dataset::open(path).map_err(|e| EngineError(format!("{}: {e}", path.display())))
}

/// `withShapefileFieldNames` (SubmitShp.cpp:134-174) + `writeAsVectorFormatV3` to EPSG:5179.
/// Returns the feature count; the error string is GDAL's message.
fn write_layer(key: &str, source: &SubmitSource, shp: &Path, notes: &mut Vec<String>) -> Result<u64, String> {
    let ds = Dataset::open(&source.gpkg).map_err(|e| e.to_string())?;
    let mut layer = ds.layer_by_name(&source.table).map_err(|e| e.to_string())?;
    let mut src_srs = layer.spatial_ref().ok_or_else(String::new)?;
    src_srs.set_axis_mapping_strategy(AxisMappingStrategy::TraditionalGisOrder);
    let mut dst_srs = SpatialRef::from_epsg(5179).map_err(|e| e.to_string())?;
    dst_srs.set_axis_mapping_strategy(AxisMappingStrategy::TraditionalGisOrder);
    let ct = CoordTransform::new(&src_srs, &dst_srs).map_err(|e| e.to_string())?;
    let geom_ty = layer
        .defn()
        .geom_fields()
        .next()
        .map(|g| g.field_type())
        .ok_or_else(|| format!("{key}: no geometry column"))?;
    let src_fields: Vec<(String, OGRFieldType::Type, i32, i32)> = layer
        .defn()
        .fields()
        .map(|f| (f.name(), f.field_type(), f.width(), f.precision()))
        .collect();
    if let Some((name, ty, ..)) = src_fields
        .iter()
        .find(|(_, ty, ..)| *ty != OGRFieldType::OFTString && *ty != OGRFieldType::OFTReal)
    {
        return Err(format!("{key}.{name}: OGR field type {ty} is not written yet"));
    }
    let names = field_names(key, &src_fields, notes);

    let driver = DriverManager::get_driver_by_name("ESRI Shapefile").map_err(|e| e.to_string())?;
    let mut out = driver.create_vector_only(shp).map_err(|e| e.to_string())?;
    let dl = out
        .create_layer(LayerOptions { name: key, srs: Some(&dst_srs), ty: geom_ty, options: Some(&["ENCODING=UTF-8"]) })
        .map_err(|e| e.to_string())?;
    let fid = FieldDefn::new(&names[0], OGRFieldType::OFTInteger64).map_err(|e| e.to_string())?;
    fid.set_width(20);
    fid.add_to_layer(&dl).map_err(|e| e.to_string())?;
    for ((_, ty, width, precision), name) in src_fields.iter().zip(&names[1..]) {
        let fd = FieldDefn::new(name, *ty).map_err(|e| e.to_string())?;
        if *ty == OGRFieldType::OFTReal {
            let (w, p) = if *width > 0 { (*width, *precision) } else { (24, 15) };
            fd.set_width(w);
            fd.set_precision(p);
        } else {
            fd.set_width(if *width > 0 && *width < 254 { *width } else { 254 });
        }
        fd.add_to_layer(&dl).map_err(|e| e.to_string())?;
    }
    let mut feats: Vec<_> = layer.features().collect();
    feats.sort_by_key(|f| f.fid());
    for f in &feats {
        let mut nf = Feature::new(dl.defn()).map_err(|e| e.to_string())?;
        let id = f.fid().and_then(|v| i64::try_from(v).ok()).ok_or_else(|| format!("{key}: feature without fid"))?;
        nf.set_field_integer64(0, id).map_err(|e| e.to_string())?;
        for (i, (_, ty, ..)) in src_fields.iter().enumerate() {
            if *ty == OGRFieldType::OFTReal {
                if let Some(v) = f.field_as_double(i).map_err(|e| e.to_string())? {
                    nf.set_field_double(i + 1, v).map_err(|e| e.to_string())?;
                }
            } else if let Some(v) = f.field_as_string(i).map_err(|e| e.to_string())? {
                nf.set_field_string(i + 1, &v).map_err(|e| e.to_string())?;
            }
        }
        if let Some(g) = f.geometry() {
            let mut g = g.clone();
            g.transform_inplace(&ct).map_err(|e| e.to_string())?;
            nf.set_geometry(g).map_err(|e| e.to_string())?;
        }
        nf.create(&dl).map_err(|e| e.to_string())?;
    }
    Ok(feats.len() as u64)
}

/// SHP names for `fid` + the source fields; renamed only when some name passes 10 bytes (SubmitShp.cpp:136-150).
fn field_names(key: &str, src_fields: &[(String, OGRFieldType::Type, i32, i32)], notes: &mut Vec<String>) -> Vec<String> {
    let all: Vec<&str> = std::iter::once(FID_FIELD).chain(src_fields.iter().map(|(n, ..)| n.as_str())).collect();
    if all.iter().all(|n| n.len() <= 10) {
        return all.iter().map(|n| (*n).to_owned()).collect();
    }
    let mut used = HashSet::new();
    all.iter()
        .map(|name| {
            let mapped = field_name_for(name, &mut used);
            if mapped != *name {
                notes.push(format!("{key} {name}={mapped}"));
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
