//! Reading a reference-data table back (the other half of `vector_table.rs`).

use std::path::Path;

use gdal::vector::{FieldValue, Geometry, Layer, LayerAccess, OGRwkbGeometryType};

use super::{columns_of, named, open, Cell, Row, ShapeKind, Shapes, TableFacts};
use crate::EngineError;

/// What kind of shapes the table of a file holds; single-part tables are read as well.
pub(super) fn kind_of(layer: &Layer<'_>) -> Result<ShapeKind, EngineError> {
    let declared = layer.defn().geom_fields().next().map(|g| g.field_type());
    match declared.map(gdal::vector::geometry_type_flatten) {
        Some(OGRwkbGeometryType::wkbLineString | OGRwkbGeometryType::wkbMultiLineString) => Ok(ShapeKind::Lines),
        Some(OGRwkbGeometryType::wkbPolygon | OGRwkbGeometryType::wkbMultiPolygon) => Ok(ShapeKind::Polygons),
        _ => Err(EngineError(format!("table {} holds neither lines nor polygons", layer.name()))),
    }
}

fn points(of: &Geometry) -> Vec<[f64; 2]> {
    let mut xyz = Vec::new();
    of.get_points(&mut xyz);
    xyz.into_iter().map(|(x, y, _)| [x, y]).collect()
}

fn rings(polygon: &Geometry) -> Vec<Vec<[f64; 2]>> {
    (0..polygon.geometry_count()).map(|i| points(&polygon.get_geometry(i))).collect()
}

/// The shapes of a row; a row without a shape has none.
fn shapes(of: Option<&Geometry>, kind: ShapeKind) -> Result<Shapes, EngineError> {
    let flat = of.map(|g| gdal::vector::geometry_type_flatten(g.geometry_type()));
    match (kind, of, flat) {
        (ShapeKind::Lines, None, _) => Ok(Shapes::Lines(Vec::new())),
        (ShapeKind::Polygons, None, _) => Ok(Shapes::Polygons(Vec::new())),
        (ShapeKind::Lines, Some(g), Some(OGRwkbGeometryType::wkbLineString)) => Ok(Shapes::Lines(vec![points(g)])),
        (ShapeKind::Lines, Some(g), Some(OGRwkbGeometryType::wkbMultiLineString)) => Ok(Shapes::Lines((0..g.geometry_count()).map(|i| points(&g.get_geometry(i))).collect())),
        (ShapeKind::Polygons, Some(g), Some(OGRwkbGeometryType::wkbPolygon)) => Ok(Shapes::Polygons(vec![rings(g)])),
        (ShapeKind::Polygons, Some(g), Some(OGRwkbGeometryType::wkbMultiPolygon)) => Ok(Shapes::Polygons((0..g.geometry_count()).map(|i| rings(&g.get_geometry(i))).collect())),
        _ => Err(EngineError("a row holds another kind of shape than its table".to_owned())),
    }
}

fn cell(value: Option<FieldValue>) -> Result<Cell, EngineError> {
    match value {
        None => Ok(Cell::Null),
        Some(FieldValue::StringValue(v)) => Ok(Cell::Text(v)),
        Some(FieldValue::RealValue(v)) => Ok(Cell::Real(v)),
        Some(FieldValue::IntegerValue(v)) => Ok(Cell::Integer(i64::from(v))),
        Some(FieldValue::Integer64Value(v)) => Ok(Cell::Integer(v)),
        Some(_) => Err(EngineError("a cell holds a kind of value these tables do not have".to_owned())),
    }
}

/// Reads a table row by row, with the cells of the named columns in that order, until `each` returns false.
///
/// # Errors
/// The file, the table or a named column is missing, or a row cannot be read.
pub fn read_table(path: &Path, table: &str, columns: &[&str], each: &mut dyn FnMut(Row) -> bool) -> Result<TableFacts, EngineError> {
    let dataset = open(path, false)?;
    let mut layer = dataset.layer_by_name(table).map_err(|e| named(path, e))?;
    let kind = kind_of(&layer)?;
    let all = columns_of(&layer)?;
    let mut wanted = Vec::new();
    for name in columns {
        wanted.push(all.iter().position(|(n, _)| n == name).ok_or_else(|| named(path, format!("table {table} has no column {name}")))?);
    }
    let epsg = layer.spatial_ref().filter(|s| s.auth_name().as_deref() == Some("EPSG")).and_then(|s| s.auth_code().ok()).and_then(|code| u32::try_from(code).ok());
    let facts = TableFacts { epsg, kind, columns: all, rows: layer.feature_count() };
    for feature in layer.features() {
        let cells = wanted.iter().map(|i| cell(feature.field(*i)?)).collect::<Result<Vec<_>, _>>()?;
        if !each(Row { shapes: shapes(feature.geometry(), kind)?, cells }) {
            break;
        }
    }
    Ok(facts)
}
