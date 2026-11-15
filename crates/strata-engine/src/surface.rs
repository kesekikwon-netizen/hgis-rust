//! A point inside a shape of several polygons, and the shape's area — GEOS, the calls QGIS makes for
//! `QgsGeometry::pointOnSurface` and `QgsGeometry::area` (the C++ geology layer reads the official colour
//! of a rock unit at such a point of the unit's largest shape, `target/cpp-ref/src/core/
//! GeologyMapService.cpp:877-887`).

use geos::{Geom, Geometry};

use crate::geom::{to_geos, PolygonRings};
use crate::EngineError;

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;

fn multi(polygons: &[PolygonRings]) -> Result<Geometry, EngineError> {
    let parts = polygons.iter().map(to_geos).collect::<Result<Vec<_>, _>>()?;
    Ok(Geometry::create_multipolygon(parts)?)
}

/// Plane area of the polygons together (holes taken out).
///
/// # Errors
/// GEOS cannot build or measure the shape.
pub fn polygons_area(polygons: &[PolygonRings]) -> Result<f64, EngineError> {
    Ok(multi(polygons)?.area()?)
}

/// A point that lies inside the shape (never in a hole, never between two parts). `None` for a shape
/// without area to stand on.
///
/// # Errors
/// GEOS cannot build the shape or find the point.
pub fn point_on_surface(polygons: &[PolygonRings]) -> Result<Option<[f64; 2]>, EngineError> {
    let point = multi(polygons)?.point_on_surface()?;
    if point.is_empty()? {
        return Ok(None);
    }
    Ok(Some([point.get_x()?, point.get_y()?]))
}
