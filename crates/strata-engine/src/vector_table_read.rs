//! Reading a reference-data table back (the other half of `vector_table.rs`).

use std::path::Path;

use gdal::vector::{FieldValue, Geometry, Layer, LayerAccess, OGRwkbGeometryType};
use gdal::{Dataset, DatasetOptions, GdalOpenFlags};

use super::{columns_of, named, open, Cell, ColumnKind, Row, ShapeKind, Shapes, TableFacts};
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
    let facts = facts_of(&layer)?;
    let mut wanted = Vec::new();
    for name in columns {
        wanted.push(facts.columns.iter().position(|(n, _)| n == name).ok_or_else(|| named(path, format!("table {table} has no column {name}")))?);
    }
    for feature in layer.features() {
        let cells = wanted.iter().map(|i| cell(feature.field(*i)?)).collect::<Result<Vec<_>, _>>()?;
        if !each(Row { shapes: shapes(feature.geometry(), facts.kind)?, cells }) {
            break;
        }
    }
    Ok(facts)
}

fn facts_of(layer: &Layer<'_>) -> Result<TableFacts, EngineError> {
    let epsg = layer.spatial_ref().filter(|s| s.auth_name().as_deref() == Some("EPSG")).and_then(|s| s.auth_code().ok()).and_then(|code| u32::try_from(code).ok());
    Ok(TableFacts { epsg, kind: kind_of(layer)?, columns: columns_of(layer)?, rows: layer.feature_count() })
}

/// What a table is, without reading its rows (a reader needs the CRS before the first row).
///
/// # Errors
/// The file or the table is missing, or the table is not one of these tables.
pub fn table_facts(path: &Path, table: &str) -> Result<TableFacts, EngineError> {
    let dataset = open(path, false)?;
    let layer = dataset.layer_by_name(table).map_err(|e| named(path, e))?;
    facts_of(&layer)
}

/// Opens a shapefile, and nothing else, for reading; its text is decoded from `encoding` ("CP949", "UTF-8").
/// Korean public shapefiles come in CP949 without a `.cpg` (`LayerOps.cpp:359-450` decides which).
fn open_shapefile(path: &Path, encoding: &str) -> Result<Dataset, EngineError> {
    let decode = format!("ENCODING={encoding}");
    let options = DatasetOptions { open_flags: GdalOpenFlags::GDAL_OF_VECTOR | GdalOpenFlags::GDAL_OF_READONLY, allowed_drivers: Some(&["ESRI Shapefile"]), open_options: Some(&[&decode]), sibling_files: None };
    Dataset::open_ex(path, options).map_err(|e| named(path, e))
}

/// The columns of a shapefile that these tables can hold, each with its place in the file. A column of
/// another kind (a date) is left out: it must not spoil the whole file.
fn known_columns(layer: &Layer<'_>) -> Vec<(usize, String, ColumnKind)> {
    layer.defn().fields().enumerate().filter_map(|(place, field)| ColumnKind::of(field.field_type()).map(|kind| (place, field.name(), kind))).collect()
}

fn shapefile_facts_of(layer: &Layer<'_>) -> Result<TableFacts, EngineError> {
    let epsg = layer.spatial_ref().filter(|s| s.auth_name().as_deref() == Some("EPSG")).and_then(|s| s.auth_code().ok()).and_then(|code| u32::try_from(code).ok());
    Ok(TableFacts { epsg, kind: kind_of(layer)?, columns: known_columns(layer).into_iter().map(|(_, name, kind)| (name, kind)).collect(), rows: layer.feature_count() })
}

/// What a shapefile holds, without reading its rows.
///
/// # Errors
/// The file is missing or no shapefile, or holds neither lines nor polygons.
pub fn shapefile_facts(path: &Path, encoding: &str) -> Result<TableFacts, EngineError> {
    let dataset = open_shapefile(path, encoding)?;
    let layer = dataset.layer(0).map_err(|e| named(path, e))?;
    shapefile_facts_of(&layer)
}

/// Reads a shapefile row by row, with the cells of the named columns in that order, until `each` returns
/// false.
///
/// # Errors
/// As `shapefile_facts`; a named column is missing or of a kind these tables do not hold; a row cannot be read.
pub fn read_shapefile(path: &Path, encoding: &str, columns: &[&str], each: &mut dyn FnMut(Row) -> bool) -> Result<TableFacts, EngineError> {
    rows_of_shapefile(path, encoding, columns, None, each)
}

/// As [`read_shapefile`], but only the rows whose shape's box meets `rect` (`[min x, min y, max x, max y]` in
/// the CRS of the file) are handed over, as `QgsFeatureRequest::setFilterRect` does for the C++. The facts
/// are those of the whole file.
///
/// # Errors
/// As [`read_shapefile`].
pub fn read_shapefile_within(path: &Path, encoding: &str, columns: &[&str], rect: [f64; 4], each: &mut dyn FnMut(Row) -> bool) -> Result<TableFacts, EngineError> {
    rows_of_shapefile(path, encoding, columns, Some(rect), each)
}

fn rows_of_shapefile(path: &Path, encoding: &str, columns: &[&str], rect: Option<[f64; 4]>, each: &mut dyn FnMut(Row) -> bool) -> Result<TableFacts, EngineError> {
    let dataset = open_shapefile(path, encoding)?;
    let mut layer = dataset.layer(0).map_err(|e| named(path, e))?;
    let (facts, known) = (shapefile_facts_of(&layer)?, known_columns(&layer));
    let mut wanted = Vec::new();
    for name in columns {
        wanted.push(known.iter().find(|(_, known, _)| known == name).map(|(place, _, _)| *place).ok_or_else(|| named(path, format!("the shapefile has no column {name}")))?);
    }
    if let Some([min_x, min_y, max_x, max_y]) = rect {
        layer.set_spatial_filter_rect(min_x, min_y, max_x, max_y);
    }
    for feature in layer.features() {
        let cells = wanted.iter().map(|place| cell(feature.field(*place)?)).collect::<Result<Vec<_>, _>>()?;
        if !each(Row { shapes: shapes(feature.geometry(), facts.kind)?, cells }) {
            break;
        }
    }
    Ok(facts)
}

#[cfg(test)]
#[path = "shapefile_tests.rs"]
mod shapefile_tests;
