//! Survey points of a DXF, read through GDAL's DXF driver.
//!
//! `target/cpp-ref/src/core/SurveyPointReader.cpp:161-224`: POINT and the first point of a
//! MULTIPOINT, and a CAD layer with no elevated point is dropped when another layer has one.

use std::collections::HashMap;
use std::path::Path;

use gdal::vector::{geometry_type_flatten, FieldValue, Geometry, LayerAccess, OGRwkbGeometryType};
use gdal::{Dataset, DatasetOptions, GdalOpenFlags};

use crate::EngineError;

#[cfg(test)]
#[path = "contour_dxf_tests.rs"]
mod tests;

/// Points in file order after millimetre duplicate averaging (`SurveyPointReader.cpp:244-250`).
pub fn read_dxf_survey_points(path: &Path) -> Result<Vec<[f64; 3]>, EngineError> {
    let named = |e: &dyn std::fmt::Display| EngineError(format!("{}: {e}", path.display()));
    let options = DatasetOptions {
        open_flags: GdalOpenFlags::GDAL_OF_VECTOR | GdalOpenFlags::GDAL_OF_READONLY,
        allowed_drivers: Some(&["DXF"]),
        open_options: None,
        sibling_files: None,
    };
    let dataset = Dataset::open_ex(path, options).map_err(|e| named(&e))?;
    let mut layer = dataset.layer(0).map_err(|e| named(&e))?;
    let layer_field = layer
        .defn()
        .fields()
        .position(|field| field.name() == "Layer");
    let mut raw = Vec::new();
    for feature in layer.features() {
        let Some(geometry) = feature.geometry() else {
            continue;
        };
        let Some(point) = first_point(geometry) else {
            continue;
        };
        let cad = match layer_field.and_then(|index| feature.field(index).ok().flatten()) {
            Some(FieldValue::StringValue(name)) => name,
            _ => String::new(),
        };
        raw.push((cad, point));
    }
    let elevated = raw.iter().any(|(_, p)| p[2].is_finite() && p[2] != 0.0);
    let kept: Vec<[f64; 3]> = if elevated {
        let mut layer_has = HashMap::<String, bool>::new();
        for (cad, p) in &raw {
            let up = layer_has.entry(cad.clone()).or_insert(false);
            *up = *up || (p[2].is_finite() && p[2] != 0.0);
        }
        raw.into_iter()
            .filter(|(cad, _)| layer_has.get(cad).copied().unwrap_or(false))
            .map(|(_, p)| p)
            .collect()
    } else {
        raw.into_iter().map(|(_, p)| p).collect()
    };
    Ok(average(kept))
}

fn first_point(geometry: &Geometry) -> Option<[f64; 3]> {
    let flat = geometry_type_flatten(geometry.geometry_type());
    let point = if flat == OGRwkbGeometryType::wkbMultiPoint {
        geometry.get_geometry(0).clone()
    } else if flat == OGRwkbGeometryType::wkbPoint {
        geometry.clone()
    } else {
        return None;
    };
    let mut xyz = Vec::new();
    point.get_points(&mut xyz);
    xyz.first()
        .copied()
        .filter(|p| p.0.is_finite() && p.1.is_finite() && p.2.is_finite())
        .map(|p| [p.0, p.1, p.2])
}

fn average(points: Vec<[f64; 3]>) -> Vec<[f64; 3]> {
    let mut buckets: HashMap<(i64, i64), (f64, f64, f64, u32)> = HashMap::new();
    for p in points {
        let key = (
            (p[0] * 1000.0).round() as i64,
            (p[1] * 1000.0).round() as i64,
        );
        let entry = buckets.entry(key).or_insert((p[0], p[1], 0.0, 0));
        entry.2 += p[2];
        entry.3 += 1;
    }
    buckets
        .into_values()
        .map(|(x, y, z_sum, n)| [x, y, z_sum / f64::from(n)])
        .collect()
}
