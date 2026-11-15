//! The lines of a DXF drawing, each with its CAD layer and height, read through GDAL's DXF driver.
//!
//! The C++ reads received topographic sheets the same way: OGR's DXF driver, one layer `entities` with the
//! field `Layer` (`target/cpp-ref/src/core/TopographicCatalog.cpp:181-209`,
//! `TopographicShapefileCache.cpp:180-240`). A sheet's code page (`$DWGCODEPAGE ANSI_949`) is the driver's
//! business, so Korean layer names and texts arrive as UTF-8.

use std::path::Path;

use gdal::vector::{geometry_type_flatten, FieldValue, Geometry, LayerAccess, OGRwkbGeometryType};
use gdal::{Dataset, DatasetOptions, GdalOpenFlags};

use crate::EngineError;

#[cfg(test)]
#[path = "dxf_lines_tests.rs"]
mod tests;

/// One line of a drawing, in the drawing's own coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct DxfLine {
    /// The CAD layer of the entity (on a topographic sheet: the code of what the line is).
    pub layer: String,
    pub points: Vec<[f64; 2]>,
    /// The height of the line, when it has heights and every vertex has the same finite one, to a
    /// millimetre (`TopographicShapefileCache.cpp:219-227`): a contour line has one, a road has none.
    pub elevation: Option<f64>,
}

/// What a drawing held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxfFacts {
    /// Lines handed on (each part of a multi-part line counts).
    pub lines: u64,
    /// Entities that are no lines: points, texts, block references, filled areas.
    pub others: u64,
}

/// Reads the lines of a drawing one by one until `each` returns false.
///
/// # Errors
/// The file is missing or is no DXF drawing, or an entity cannot be read.
pub fn read_dxf_lines(path: &Path, each: &mut dyn FnMut(DxfLine) -> bool) -> Result<DxfFacts, EngineError> {
    let named = |e: &dyn std::fmt::Display| EngineError(format!("{}: {e}", path.display()));
    let options = DatasetOptions { open_flags: GdalOpenFlags::GDAL_OF_VECTOR | GdalOpenFlags::GDAL_OF_READONLY, allowed_drivers: Some(&["DXF"]), open_options: None, sibling_files: None };
    let dataset = Dataset::open_ex(path, options).map_err(|e| named(&e))?;
    // The driver hands every entity out through one layer.
    let mut layer = dataset.layer_by_name("entities").map_err(|e| named(&e))?;
    let layer_field = layer.defn().fields().position(|field| field.name() == "Layer").ok_or_else(|| named(&"the drawing names no CAD layers"))?;
    let mut facts = DxfFacts { lines: 0, others: 0 };
    for feature in layer.features() {
        let Some(geometry) = feature.geometry() else {
            facts.others += 1;
            continue;
        };
        let kind = geometry.geometry_type();
        let parts: Vec<Geometry> = match geometry_type_flatten(kind) {
            OGRwkbGeometryType::wkbLineString => vec![geometry.clone()],
            OGRwkbGeometryType::wkbMultiLineString => (0..geometry.geometry_count()).map(|i| geometry.get_geometry(i).clone()).collect(),
            _ => {
                facts.others += 1;
                continue;
            }
        };
        let cad_layer = match feature.field(layer_field).map_err(|e| named(&e))? {
            Some(FieldValue::StringValue(name)) => name,
            _ => String::new(),
        };
        // Flattening takes the height away from the kind: a kind it changes has heights.
        let has_heights = geometry_type_flatten(kind) != kind;
        for part in parts {
            let mut xyz = Vec::new();
            part.get_points(&mut xyz);
            let level = xyz.first().map(|first| first.2).filter(|z| has_heights && xyz.iter().all(|p| p.2.is_finite() && (p.2 - z).abs() < 0.001));
            facts.lines += 1;
            if !each(DxfLine { layer: cad_layer.clone(), points: xyz.into_iter().map(|(x, y, _)| [x, y]).collect(), elevation: level }) {
                return Ok(facts);
            }
        }
    }
    Ok(facts)
}
