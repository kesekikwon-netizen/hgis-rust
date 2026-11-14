//! Reads the source GPKG layers of one key into memory and merges several into one table
//! (cpp `SubmitShp.cpp:23-48` emptyMemoryCopy + copyValue, `:82-133` mergeLayers).

use std::collections::HashMap;

use gdal::spatial_ref::{AxisMappingStrategy, CoordTransform, SpatialRef};
use gdal::vector::sql::Dialect;
use gdal::vector::{Geometry, LayerAccess, OGRFieldType, OGRwkbGeometryType};
use gdal::Dataset;

use super::SubmitSource;

/// The field kinds whose DBF columns differ in the cpp truth sets (tests/fixtures/cpp_submit_types_*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Integer,
    Integer64,
    Real,
    Date,
    DateTime,
    Boolean,
    String,
}

#[derive(Debug, Clone)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) width: i32,
    pub(crate) precision: i32,
}

/// One value. Dates are OGR's `YYYY/MM/DD`; date-times are already the text the cpp packages hold.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Bool(bool),
}

/// One key's features in memory: `fields[0]` is the GPKG FID column (first DBF column of every cpp package).
#[derive(Debug)]
pub(crate) struct Table {
    pub(crate) fields: Vec<Field>,
    pub(crate) rows: Vec<(Vec<Value>, Option<Geometry>)>,
    pub(crate) srs: SpatialRef,
    pub(crate) geom_type: OGRwkbGeometryType::Type,
}

/// DBF name of the GPKG FID column in every cpp package (`fid` N(20,0)).
const FID_FIELD: &str = "fid";

/// Reads one GPKG layer, features by fid.
pub(crate) fn read_table(key: &str, source: &SubmitSource) -> Result<Table, String> {
    let ds = Dataset::open(&source.gpkg).map_err(|e| e.to_string())?;
    let declared = declared_types(&ds, &source.table)?;
    let mut layer = ds.layer_by_name(&source.table).map_err(|e| e.to_string())?;
    let mut srs = layer.spatial_ref().ok_or_else(String::new)?;
    srs.set_axis_mapping_strategy(AxisMappingStrategy::TraditionalGisOrder);
    let geom_type = layer
        .defn()
        .geom_fields()
        .next()
        .map(|g| g.field_type())
        .ok_or_else(|| format!("{key}: no geometry column"))?;
    let mut fields = vec![Field { name: FID_FIELD.to_owned(), kind: Kind::Integer64, width: 0, precision: 0 }];
    for f in layer.defn().fields() {
        let name = f.name();
        let boolean = declared.get(&name).is_some_and(|t| t.eq_ignore_ascii_case("BOOLEAN"));
        let kind = match f.field_type() {
            OGRFieldType::OFTInteger if boolean => Kind::Boolean,
            OGRFieldType::OFTInteger => Kind::Integer,
            OGRFieldType::OFTInteger64 => Kind::Integer64,
            OGRFieldType::OFTReal => Kind::Real,
            OGRFieldType::OFTDate => Kind::Date,
            OGRFieldType::OFTDateTime => Kind::DateTime,
            OGRFieldType::OFTString => Kind::String,
            other => return Err(format!("{key}.{name}: OGR field type {other} has no shapefile column")),
        };
        fields.push(Field { name, kind, width: f.width(), precision: f.precision() });
    }
    let mut feats: Vec<_> = layer.features().collect();
    feats.sort_by_key(|f| f.fid());
    let mut rows = Vec::with_capacity(feats.len());
    for f in &feats {
        let id = f.fid().and_then(|v| i64::try_from(v).ok()).ok_or_else(|| format!("{key}: feature without fid"))?;
        let mut values = vec![Value::Int(id)];
        for (i, field) in fields[1..].iter().enumerate() {
            values.push(read_value(f, i, field.kind).map_err(|e| e.to_string())?);
        }
        rows.push((values, f.geometry().cloned()));
    }
    Ok(Table { fields, rows, srs, geom_type })
}

/// Declared SQLite column types (`BOOLEAN` is an OGR subtype gdal 0.19 cannot read).
fn declared_types(ds: &Dataset, table: &str) -> Result<HashMap<String, String>, String> {
    let sql = format!("SELECT name, type FROM pragma_table_info('{}')", table.replace('\'', "''"));
    let mut out = HashMap::new();
    if let Some(mut rs) = ds.execute_sql(sql, None, Dialect::DEFAULT).map_err(|e| e.to_string())? {
        for f in rs.features() {
            let name = f.field_as_string(0).map_err(|e| e.to_string())?.unwrap_or_default();
            let ty = f.field_as_string(1).map_err(|e| e.to_string())?.unwrap_or_default();
            out.insert(name, ty);
        }
    }
    Ok(out)
}

fn read_value(f: &gdal::vector::Feature<'_>, i: usize, kind: Kind) -> gdal::errors::Result<Value> {
    Ok(match kind {
        Kind::Integer | Kind::Integer64 => f.field_as_integer64(i)?.map_or(Value::Null, Value::Int),
        Kind::Boolean => f.field_as_integer64(i)?.map_or(Value::Null, |v| Value::Bool(v != 0)),
        Kind::Real => f.field_as_double(i)?.map_or(Value::Null, Value::Real),
        Kind::DateTime => f.field_as_string(i)?.map_or(Value::Null, |s| Value::Text(shapefile_datetime(&s))),
        Kind::Date | Kind::String => f.field_as_string(i)?.map_or(Value::Null, Value::Text),
    })
}

/// The cpp packages hold a date-time as text without a time zone (cpp_submit_types_*: `2026/11/14 08:30:15.000`
/// for `2026-11-14T08:30:15Z`):
/// `YYYY/MM/DD HH:MM:SS.sss` (from OGR's `YYYY/MM/DD HH:MM:SS[.sss][+TZ]`).
fn shapefile_datetime(ogr: &str) -> String {
    let (Some(stamp), Some(rest)) = (ogr.get(..17), ogr.get(17..)) else {
        return ogr.to_owned();
    };
    let digits = rest.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(rest.len());
    let seconds: f64 = rest[..digits].parse().unwrap_or(0.0);
    format!("{stamp}{seconds:06.3}")
}

/// `mergeLayers` (SubmitShp.cpp:82-133): fields of every layer by name (first wins), features of each
/// layer in turn, geometries moved to the first layer's CRS, values copied by field name.
pub(crate) fn merge_tables(key: &str, sources: &[&SubmitSource], notes: &mut Vec<String>) -> Result<Table, String> {
    let mut tables = Vec::with_capacity(sources.len());
    for s in sources {
        tables.push((read_table(key, s)?, &s.title));
    }
    let mut it = tables.into_iter();
    let (mut merged, _) = it.next().ok_or_else(String::new)?;
    let rest: Vec<_> = it.collect();
    for (t, _) in &rest {
        for f in &t.fields {
            if !merged.fields.iter().any(|m| m.name == f.name) {
                merged.fields.push(f.clone());
            }
        }
    }
    for (values, _) in &mut merged.rows {
        values.resize(merged.fields.len(), Value::Null);
    }
    for (t, title) in rest {
        let to_primary = (t.srs != merged.srs)
            .then(|| CoordTransform::new(&t.srs, &merged.srs))
            .transpose()
            .map_err(|e| e.to_string())?;
        for (values, geom) in t.rows {
            let mut row = vec![Value::Null; merged.fields.len()];
            for (slot, target) in row.iter_mut().zip(&merged.fields) {
                let Some(i) = t.fields.iter().position(|f| f.name == target.name) else { continue };
                *slot = convert(&values[i], target.kind).unwrap_or_else(|| {
                    notes.push(format!("{key} {}: 형식이 달라 값 하나를 비웠습니다", target.name));
                    Value::Null
                });
            }
            let geom = match (geom, &to_primary) {
                (Some(mut g), Some(ct)) => {
                    g.transform_inplace(ct)
                        .map_err(|_| format!("{key} 레이어 「{title}」의 좌표를 합치다 변환에 실패했습니다."))?;
                    Some(g)
                }
                (g, _) => g,
            };
            merged.rows.push((row, geom));
        }
    }
    Ok(merged)
}

/// Value conversion of SubmitShp.cpp:36-47 (copyValue) for the kinds above; no cpp truth set covers mixed types.
/// `None` = the value cannot be held (becomes NULL + note).
fn convert(v: &Value, to: Kind) -> Option<Value> {
    let rounded = |d: f64, max: i64| (d.is_finite() && d.round().abs() <= max as f64).then(|| Value::Int(d.round() as i64));
    match (v, to) {
        (Value::Null, _) => Some(Value::Null),
        (Value::Int(x), Kind::Integer) => i32::try_from(*x).ok().map(|_| Value::Int(*x)),
        (Value::Int(x), Kind::Integer64) => Some(Value::Int(*x)),
        (Value::Real(d), Kind::Integer) => rounded(*d, i64::from(i32::MAX)),
        (Value::Real(d), Kind::Integer64) => rounded(*d, i64::MAX),
        (Value::Text(s), Kind::Integer | Kind::Integer64) => s.trim().parse::<i64>().ok().map(Value::Int),
        (Value::Int(x), Kind::Real) => Some(Value::Real(*x as f64)),
        (Value::Real(d), Kind::Real) => Some(Value::Real(*d)),
        (Value::Text(s), Kind::Real) => s.trim().parse::<f64>().ok().map(Value::Real),
        (Value::Bool(b), Kind::Boolean) => Some(Value::Bool(*b)),
        (Value::Int(x), Kind::Boolean) => Some(Value::Bool(*x != 0)),
        (Value::Text(s), Kind::Date | Kind::DateTime | Kind::String) => Some(Value::Text(s.clone())),
        (Value::Int(x), Kind::String) => Some(Value::Text(x.to_string())),
        (Value::Real(d), Kind::String) => Some(Value::Text(d.to_string())),
        (Value::Bool(b), Kind::String) => Some(Value::Text(b.to_string())),
        _ => None,
    }
}
