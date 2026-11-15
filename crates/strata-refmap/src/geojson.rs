//! Reading the GeoJSON answers of the map services: the CRS they declare, their polygons and their
//! properties as table columns. The C++ lets OGR read the file and then checks the declared CRS itself
//! (`target/cpp-ref/src/core/GeologyMapService.cpp:795-820`, `:842-857`).

use serde_json::{Map, Value};
use strata_engine::{Cell, ColumnKind, PolygonRings};

#[cfg(test)]
#[path = "geojson_tests.rs"]
mod tests;

/// The EPSG code of the CRS the answer's coordinates are in. An answer without a `crs` member is in
/// longitude and latitude (EPSG:4326), as GeoJSON readers take it. `None`: the answer names a CRS in a
/// form that is not understood (the C++ refuses the answer then, `:806-810`).
#[must_use]
pub fn declared_epsg(collection: &Map<String, Value>) -> Option<u32> {
    let Some(definition) = collection.get("crs") else {
        return Some(4326);
    };
    let properties = definition.get("properties");
    match definition.get("type").and_then(Value::as_str) {
        Some("name") => named_epsg(properties?.get("name")?.as_str()?),
        Some("EPSG") => properties?.get("code")?.as_u64().and_then(|code| u32::try_from(code).ok()),
        _ => None,
    }
}

/// `EPSG:5186`, `urn:ogc:def:crs:EPSG::5186`, `http://www.opengis.net/def/crs/EPSG/0/5186`, and the names of
/// longitude and latitude on WGS 84 (`CRS84`).
fn named_epsg(name: &str) -> Option<u32> {
    let parts: Vec<&str> = name.split([':', '/']).filter(|p| !p.is_empty()).collect();
    if parts.iter().any(|p| p.eq_ignore_ascii_case("CRS84")) || (parts.len() >= 2 && parts[parts.len() - 2].eq_ignore_ascii_case("CRS") && parts[parts.len() - 1] == "84") {
        return Some(4326);
    }
    if !parts.iter().any(|p| p.eq_ignore_ascii_case("EPSG")) {
        return None;
    }
    parts.last()?.parse().ok()
}

fn ring(value: &Value) -> Option<Vec<[f64; 2]>> {
    value.as_array()?.iter().map(|point| Some([point.get(0)?.as_f64()?, point.get(1)?.as_f64()?])).collect()
}

fn rings(value: &Value) -> Option<PolygonRings> {
    value.as_array()?.iter().map(ring).collect()
}

/// The polygons of a feature's geometry (`Polygon` or `MultiPolygon`), each as its rings, outer first, in
/// the answer's CRS. `None` for no geometry, another kind of geometry or malformed coordinates; a polygon
/// without a ring is left out.
#[must_use]
pub fn polygons_of(geometry: &Value) -> Option<Vec<PolygonRings>> {
    let coordinates = geometry.get("coordinates")?;
    let polygons = match geometry.get("type")?.as_str()? {
        "Polygon" => vec![rings(coordinates)?],
        "MultiPolygon" => coordinates.as_array()?.iter().map(rings).collect::<Option<Vec<_>>>()?,
        _ => return None,
    };
    Some(polygons.into_iter().filter(|p| !p.is_empty()).collect())
}

/// The properties of the features as table columns: every name once, feature after feature, the names of
/// one feature by name (the JSON reader of this workspace keeps the members of an object sorted; the C++
/// reads through OGR, which keeps the order of the file; nothing reads these columns by position).
/// Whole numbers only give an integer column, numbers only a real column, anything else a text column.
#[must_use]
pub fn columns_of(features: &[Value]) -> Vec<(String, ColumnKind)> {
    let mut columns: Vec<(String, Option<ColumnKind>)> = Vec::new();
    for properties in features.iter().filter_map(|f| f.get("properties")?.as_object()) {
        for (name, value) in properties {
            let kind = match value {
                Value::Null => None,
                Value::Number(n) if n.is_i64() => Some(ColumnKind::Integer),
                Value::Number(_) => Some(ColumnKind::Real),
                _ => Some(ColumnKind::Text),
            };
            let at = columns.iter().position(|(n, _)| n == name).unwrap_or_else(|| {
                columns.push((name.clone(), None));
                columns.len() - 1
            });
            columns[at].1 = match (columns[at].1, kind) {
                (known, None) => known,
                (None, seen) => seen,
                (Some(ColumnKind::Integer), Some(ColumnKind::Integer)) => Some(ColumnKind::Integer),
                (Some(ColumnKind::Integer | ColumnKind::Real), Some(ColumnKind::Integer | ColumnKind::Real)) => Some(ColumnKind::Real),
                _ => Some(ColumnKind::Text),
            };
        }
    }
    columns.into_iter().map(|(name, kind)| (name, kind.unwrap_or(ColumnKind::Text))).collect()
}

/// The text without what stands between `<` and the next `>` (`kTagRe` `<[^>]*>`, `:839`): the service
/// puts links into some texts.
#[must_use]
pub fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        out.push_str(&rest[..open]);
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// One property as the cell of its column. Texts lose their markup (`:853-857`).
#[must_use]
pub fn cell(value: Option<&Value>, kind: ColumnKind) -> Cell {
    match (value, kind) {
        (None | Some(Value::Null), _) => Cell::Null,
        (Some(Value::Number(n)), ColumnKind::Integer) => n.as_i64().map_or(Cell::Null, Cell::Integer),
        (Some(Value::Number(n)), ColumnKind::Real) => n.as_f64().map_or(Cell::Null, Cell::Real),
        (Some(Value::String(text)), _) => Cell::Text(if text.contains('<') { strip_tags(text) } else { text.clone() }),
        (Some(other), _) => Cell::Text(other.to_string()),
    }
}
