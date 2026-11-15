//! Reference-data tables in GeoPackage files, written and read through GDAL's GPKG driver.
//!
//! The C++ keeps every downloaded reference map as a GeoPackage table in the survey folder, written by
//! `QgsVectorFileWriter` over the same driver (`target/cpp-ref/src/core/PreparedReferenceMap.cpp:131-155`,
//! `src/app/KaTopographicImportDialog.cpp:148-200`): UTF-8, one table, multi-part shapes.

use std::path::Path;

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Feature, FieldDefn, Geometry, Layer, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType};
use gdal::{Dataset, DatasetOptions, DriverManager, GdalOpenFlags};

use crate::geom::PolygonRings;
use crate::EngineError;

#[path = "vector_table_read.rs"]
mod read;
#[cfg(test)]
#[path = "vector_table_tests.rs"]
mod tests;

pub use read::{read_shapefile, read_table, shapefile_facts, table_facts};

/// What a column holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Text,
    Real,
    Integer,
}

/// One value of a row.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    Text(String),
    Real(f64),
    Integer(i64),
}

/// The shapes of one row: the parts of a multi-part line or polygon, in the table's CRS.
#[derive(Debug, Clone, PartialEq)]
pub enum Shapes {
    Lines(Vec<Vec<[f64; 2]>>),
    Polygons(Vec<PolygonRings>),
}

/// What kind of shapes a table holds (stored as MULTILINESTRING or MULTIPOLYGON).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Lines,
    Polygons,
}

/// A table to create.
#[derive(Debug, Clone)]
pub struct TableLayout<'a> {
    pub table: &'a str,
    pub epsg: u32,
    pub kind: ShapeKind,
    pub columns: &'a [(&'a str, ColumnKind)],
}

/// One row: its shapes and one cell for each column, in the table's column order.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub shapes: Shapes,
    pub cells: Vec<Cell>,
}

/// What a table is, read from the file.
#[derive(Debug, Clone, PartialEq)]
pub struct TableFacts {
    /// EPSG code of the table's CRS, when the file names one.
    pub epsg: Option<u32>,
    pub kind: ShapeKind,
    pub columns: Vec<(String, ColumnKind)>,
    pub rows: u64,
}

impl ColumnKind {
    fn ogr(self) -> OGRFieldType::Type {
        match self {
            Self::Text => OGRFieldType::OFTString,
            Self::Real => OGRFieldType::OFTReal,
            Self::Integer => OGRFieldType::OFTInteger64,
        }
    }

    /// The kind of a column of a file; `None` for a column type these tables never hold.
    pub(super) fn of(ogr: OGRFieldType::Type) -> Option<Self> {
        match ogr {
            OGRFieldType::OFTString => Some(Self::Text),
            OGRFieldType::OFTReal => Some(Self::Real),
            OGRFieldType::OFTInteger | OGRFieldType::OFTInteger64 => Some(Self::Integer),
            _ => None,
        }
    }
}

impl ShapeKind {
    fn ogr(self) -> OGRwkbGeometryType::Type {
        match self {
            Self::Lines => OGRwkbGeometryType::wkbMultiLineString,
            Self::Polygons => OGRwkbGeometryType::wkbMultiPolygon,
        }
    }
}

pub(super) fn named(path: &Path, e: impl std::fmt::Display) -> EngineError {
    EngineError(format!("{}: {e}", path.display()))
}

/// Opens a GeoPackage, and nothing else, for reading or for adding rows.
pub(super) fn open(path: &Path, update: bool) -> Result<Dataset, EngineError> {
    let access = if update { GdalOpenFlags::GDAL_OF_UPDATE } else { GdalOpenFlags::GDAL_OF_READONLY };
    let options = DatasetOptions { open_flags: GdalOpenFlags::GDAL_OF_VECTOR | access, allowed_drivers: Some(&["GPKG"]), open_options: None, sibling_files: None };
    Dataset::open_ex(path, options).map_err(|e| named(path, e))
}

/// The columns of a table of a file, in its order.
pub(super) fn columns_of(layer: &Layer<'_>) -> Result<Vec<(String, ColumnKind)>, EngineError> {
    let unknown = |name: &str| EngineError(format!("column {name} holds a kind of value these tables do not have"));
    layer.defn().fields().map(|f| ColumnKind::of(f.field_type()).map(|kind| (f.name(), kind)).ok_or_else(|| unknown(&f.name()))).collect()
}

fn points(of: &[[f64; 2]], kind: OGRwkbGeometryType::Type) -> Result<Geometry, EngineError> {
    let mut g = Geometry::empty(kind)?;
    for p in of {
        g.add_point_2d((p[0], p[1]));
    }
    Ok(g)
}

fn to_ogr(shapes: &Shapes, kind: ShapeKind) -> Result<Geometry, EngineError> {
    let mut multi = Geometry::empty(kind.ogr())?;
    match (shapes, kind) {
        (Shapes::Lines(lines), ShapeKind::Lines) => {
            for line in lines {
                multi.add_geometry(points(line, OGRwkbGeometryType::wkbLineString)?)?;
            }
        }
        (Shapes::Polygons(polygons), ShapeKind::Polygons) => {
            for rings in polygons {
                let mut polygon = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
                for ring in rings {
                    polygon.add_geometry(points(ring, OGRwkbGeometryType::wkbLinearRing)?)?;
                }
                multi.add_geometry(polygon)?;
            }
        }
        _ => return Err(EngineError("a row holds another kind of shape than its table".to_owned())),
    }
    Ok(multi)
}

/// Writes the rows into the table; a row that does not fit stops the writing.
fn add_rows(layer: &Layer<'_>, kind: ShapeKind, columns: &[ColumnKind], rows: &mut dyn Iterator<Item = Row>) -> Result<(), EngineError> {
    for row in rows {
        if row.cells.len() != columns.len() {
            return Err(EngineError(format!("a row has {} cells for {} columns", row.cells.len(), columns.len())));
        }
        let mut feature = Feature::new(layer.defn())?;
        for (i, (cell, column)) in row.cells.iter().zip(columns).enumerate() {
            match (cell, column) {
                (Cell::Null, _) => {}
                (Cell::Text(v), ColumnKind::Text) => feature.set_field_string(i, v)?,
                (Cell::Real(v), ColumnKind::Real) => feature.set_field_double(i, *v)?,
                (Cell::Integer(v), ColumnKind::Integer) => feature.set_field_integer64(i, *v)?,
                _ => return Err(EngineError(format!("cell {i} of a row is not of the kind of its column"))),
            }
        }
        feature.set_geometry(to_ogr(&row.shapes, kind)?)?;
        feature.create(layer)?;
    }
    Ok(())
}

fn rows_in(path: &Path, table: &str) -> Result<u64, EngineError> {
    Ok(open(path, false)?.layer_by_name(table).map_err(|e| named(path, e))?.feature_count())
}

/// Creates the file with one table and writes the rows. Returns the number of rows found when the finished
/// file is opened again (`PreparedReferenceMap.cpp:147-153` checks the same). The file must not exist.
///
/// # Errors
/// The file exists, cannot be written, or a row does not fit the table; nothing is left on disk then.
pub fn write_new_table(path: &Path, layout: &TableLayout<'_>, rows: &mut dyn Iterator<Item = Row>) -> Result<u64, EngineError> {
    if path.exists() {
        return Err(named(path, "the file is there already"));
    }
    let written = create(path, layout, rows).and_then(|()| rows_in(path, layout.table));
    if written.is_err() {
        // The file did not exist before this call: what is left of it is this call's own.
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("gpkg-journal"));
    }
    written
}

fn create(path: &Path, layout: &TableLayout<'_>, rows: &mut dyn Iterator<Item = Row>) -> Result<(), EngineError> {
    let srs = SpatialRef::from_epsg(layout.epsg)?;
    let mut dataset = DriverManager::get_driver_by_name("GPKG")?.create_vector_only(path).map_err(|e| named(path, e))?;
    let mut work = dataset.start_transaction()?;
    {
        let layer = work.create_layer(LayerOptions { name: layout.table, srs: Some(&srs), ty: layout.kind.ogr(), options: None })?;
        for (name, kind) in layout.columns {
            FieldDefn::new(name, kind.ogr())?.add_to_layer(&layer)?;
        }
        let kinds: Vec<ColumnKind> = layout.columns.iter().map(|(_, kind)| *kind).collect();
        add_rows(&layer, layout.kind, &kinds, rows)?;
    }
    work.commit()?;
    Ok(dataset.close()?)
}

/// Adds rows to a table written before. Returns the number of rows the table holds afterwards.
///
/// # Errors
/// The file or the table is missing, or a row does not fit; the rows that were there stay as they were.
pub fn append_to_table(path: &Path, table: &str, rows: &mut dyn Iterator<Item = Row>) -> Result<u64, EngineError> {
    let mut dataset = open(path, true)?;
    let work = dataset.start_transaction()?;
    {
        let layer = work.layer_by_name(table).map_err(|e| named(path, e))?;
        let kind = read::kind_of(&layer)?;
        let kinds: Vec<ColumnKind> = columns_of(&layer)?.into_iter().map(|(_, kind)| kind).collect();
        // A row that does not fit ends the function here: the transaction is taken back when it is dropped.
        add_rows(&layer, kind, &kinds, rows)?;
    }
    work.commit()?;
    dataset.close()?;
    rows_in(path, table)
}
