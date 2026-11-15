//! The area around the survey area that downloaded reference data is kept for.
//!
//! C++ `SurveyScopeClip` (`target/cpp-ref/src/core/SurveyScopeClip.cpp`): the survey-area shapes are joined
//! (`QgsGeometry::unaryUnion`, :69) and grown by the radius (`survey.buffer(meters, 24)`, :72); a shape of a
//! reference layer is kept when it touches that area (`QgsGeometryEngine::intersects`, :31-34). The same GEOS
//! functions are called here. Cutting a shape to the area is not in the C++ (it takes whole shapes or the map
//! view); it is GEOS `intersection`, for the layers the owner gave a radius (docs/RUST_PLAN.md 2.9).

use geos::{CoordSeq, CoordType, Geom, Geometry, GeometryTypes, PreparedGeometry};

use crate::geom::{collect_polygons, ring_points, PolygonRings};
use crate::{EngineError, Transformer};

#[cfg(test)]
#[path = "scope_tests.rs"]
mod tests;

/// Straight pieces per quarter circle of a round corner (`SurveyScopeClip.cpp:72`).
const QUARTER_SEGMENTS: i32 = 24;

/// What the C++ says when the area cannot be built (`SurveyScopeClip.cpp:25`), without its fixed "5km".
const NOT_BUILT: &str = "조사 주변 범위를 만들지 못했습니다.";

/// The joined survey areas, grown by a radius. All coordinates are metres of one CRS, the caller's.
pub struct Scope {
    area: Geometry,
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope").field("bbox", &self.bbox().ok()).finish()
    }
}

/// A scope made ready for many touch tests (GEOS prepared geometry, as the C++ `prepareGeometry`, :99).
pub struct ScopeTest<'a> {
    prepared: PreparedGeometry<'a>,
}

impl std::fmt::Debug for ScopeTest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopeTest")
    }
}

fn sequence(points: &[[f64; 2]], close: bool) -> Result<CoordSeq, EngineError> {
    let mut flat: Vec<f64> = points.iter().flat_map(|p| [p[0], p[1]]).collect();
    if close && points.first() != points.last() {
        flat.extend_from_slice(&points[0]);
    }
    Ok(CoordSeq::new_from_buffer(&flat, flat.len() / 2, CoordType::XY)?)
}

/// A polygon of GEOS from rings (outer ring first). `None` for an outer ring that cannot enclose anything;
/// a hole that cannot is left out.
fn polygon(rings: &[Vec<[f64; 2]>]) -> Result<Option<Geometry>, EngineError> {
    // A closed ring has at least three different corners and the first one again.
    let encloses = |ring: &Vec<[f64; 2]>| ring.len() >= 4 || (ring.len() == 3 && ring.first() != ring.last());
    let Some(outer) = rings.first().filter(|r| encloses(r)) else {
        return Ok(None);
    };
    let exterior = Geometry::create_linear_ring(sequence(outer, true)?)?;
    let mut holes = Vec::new();
    for ring in rings[1..].iter().filter(|r| encloses(r)) {
        holes.push(Geometry::create_linear_ring(sequence(ring, true)?)?);
    }
    Ok(Some(Geometry::create_polygon(exterior, holes)?))
}

/// A line of GEOS; a single point when the line has only one.
fn line(points: &[[f64; 2]]) -> Result<Option<Geometry>, EngineError> {
    match points.len() {
        0 => Ok(None),
        1 => Ok(Some(sequence(points, false)?.create_point()?)),
        _ => Ok(Some(Geometry::create_line_string(sequence(points, false)?)?)),
    }
}

/// The lines inside a result of GEOS (a cut may also leave single points where a line only touches).
fn collect_lines<G: Geom>(g: &G, out: &mut Vec<Vec<[f64; 2]>>) -> Result<(), EngineError> {
    match g.geometry_type()? {
        GeometryTypes::LineString | GeometryTypes::LinearRing => {
            if !g.is_empty()? {
                out.push(ring_points(g)?);
            }
        }
        GeometryTypes::MultiLineString | GeometryTypes::GeometryCollection => {
            for i in 0..g.get_num_geometries()? {
                collect_lines(&g.get_geometry_n(i)?, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl Scope {
    /// The scope of these survey-area shapes (WKT polygons). `None` when there is no shape: then nothing is
    /// cut away (`SurveyScopeClip.cpp:22`, :89).
    ///
    /// # Errors
    /// A WKT that cannot be read, a radius that is not a positive number, or GEOS cannot build the area.
    pub fn around(polygon_wkts: &[String], radius: f64) -> Result<Option<Self>, EngineError> {
        let Some(joined) = Self::joined(polygon_wkts)? else {
            return Ok(None);
        };
        // `bufferMeters` gives nothing for such a radius, and `scopeOf` reports that (:71, :24-27).
        if !radius.is_finite() || radius <= 0.0 {
            return Err(EngineError(NOT_BUILT.to_owned()));
        }
        Self::checked(joined.buffer(radius, QUARTER_SEGMENTS)?).map(Some)
    }

    /// The shapes joined into one; `None` when there is none.
    fn joined(polygon_wkts: &[String]) -> Result<Option<Geometry>, EngineError> {
        let mut parts = Vec::new();
        for wkt in polygon_wkts {
            let shape = Geometry::new_from_wkt(wkt)?;
            if !shape.is_empty()? {
                parts.push(shape);
            }
        }
        if parts.is_empty() {
            return Ok(None);
        }
        Ok(Some(Geometry::create_geometry_collection(parts)?.unary_union()?))
    }

    /// The scope of these shapes (WKT polygons) as they are: joined, not grown. `None` when there is no
    /// shape.
    ///
    /// # Errors
    /// A WKT that cannot be read, or GEOS cannot build the area.
    pub fn of(polygon_wkts: &[String]) -> Result<Option<Self>, EngineError> {
        Self::joined(polygon_wkts)?.map(Self::checked).transpose()
    }

    fn checked(area: Geometry) -> Result<Self, EngineError> {
        if area.is_empty()? || !area.is_valid()? {
            return Err(EngineError(NOT_BUILT.to_owned()));
        }
        Ok(Self { area })
    }

    /// The polygons of the scope: one for each group of survey areas that lie apart.
    ///
    /// # Errors
    /// GEOS cannot read its own result.
    pub fn polygons(&self) -> Result<Vec<PolygonRings>, EngineError> {
        let mut out = Vec::new();
        collect_polygons(&self.area, &mut out)?;
        Ok(out)
    }

    /// `[min x, min y, max x, max y]`.
    ///
    /// # Errors
    /// GEOS cannot read its own result.
    pub fn bbox(&self) -> Result<[f64; 4], EngineError> {
        let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for p in self.polygons()?.iter().filter_map(|rings| rings.first()).flatten() {
            b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
        }
        Ok(b)
    }

    /// Plane area in square metres.
    ///
    /// # Errors
    /// GEOS cannot measure the area.
    pub fn area(&self) -> Result<f64, EngineError> {
        Ok(self.area.area()?)
    }

    /// The same scope in another CRS: every corner point goes through PROJ. The straight pieces between them
    /// are about 650 m long for a scope of 10 km and then bend by about a quarter of a millimetre between two
    /// Korean belts; pieces ten times as long (a scope of 100 km) bend a hundred times as much, a few
    /// centimetres.
    ///
    /// # Errors
    /// PROJ does not know one of the CRSs or cannot carry a point over.
    pub fn reprojected(&self, from_epsg: u32, to_epsg: u32) -> Result<Self, EngineError> {
        let transformer = Transformer::new(from_epsg, to_epsg)?;
        let mut parts = Vec::new();
        for rings in self.polygons()? {
            let carried = rings.iter().map(|ring| transformer.transform_all(ring)).collect::<Result<Vec<_>, _>>()?;
            parts.extend(polygon(&carried)?);
        }
        Self::checked(Geometry::create_multipolygon(parts)?)
    }

    /// Makes the scope ready for many touch tests.
    ///
    /// # Errors
    /// GEOS cannot prepare the area.
    pub fn test(&self) -> Result<ScopeTest<'_>, EngineError> {
        Ok(ScopeTest { prepared: self.area.to_prepared_geom()? })
    }

    /// The parts of a polygon (rings, outer first) that lie inside the scope. A shape that crosses itself is
    /// mended first (GEOS `MakeValid`); without that the cut may fail or give a wrong area.
    ///
    /// # Errors
    /// GEOS cannot cut the shape.
    pub fn cut_polygon(&self, rings: &[Vec<[f64; 2]>]) -> Result<Vec<PolygonRings>, EngineError> {
        let Some(mut shape) = polygon(rings)? else {
            return Ok(Vec::new());
        };
        if !shape.is_valid()? {
            shape = shape.make_valid()?;
        }
        let mut out = Vec::new();
        collect_polygons(&self.area.intersection(&shape)?, &mut out)?;
        Ok(out)
    }

    /// The pieces of a line that lie inside the scope.
    ///
    /// # Errors
    /// GEOS cannot cut the line.
    pub fn cut_line(&self, points: &[[f64; 2]]) -> Result<Vec<Vec<[f64; 2]>>, EngineError> {
        let Some(shape) = line(points)? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        collect_lines(&self.area.intersection(&shape)?, &mut out)?;
        Ok(out)
    }
}

impl ScopeTest<'_> {
    /// Whether the point lies in the scope or on its edge.
    ///
    /// # Errors
    /// GEOS cannot build or test the point.
    pub fn touches_point(&self, xy: [f64; 2]) -> Result<bool, EngineError> {
        Ok(self.prepared.intersects(&sequence(&[xy], false)?.create_point()?)?)
    }

    /// Whether any part of the line lies in the scope (`intersectsScope`, `SurveyScopeClip.cpp:31-34`).
    ///
    /// # Errors
    /// GEOS cannot build or test the line.
    pub fn touches_line(&self, points: &[[f64; 2]]) -> Result<bool, EngineError> {
        match line(points)? {
            Some(shape) => Ok(self.prepared.intersects(&shape)?),
            None => Ok(false),
        }
    }

    /// Whether any part of the polygon (rings, outer first) lies in the scope.
    ///
    /// # Errors
    /// GEOS cannot build or test the polygon.
    pub fn touches_polygon(&self, rings: &[Vec<[f64; 2]>]) -> Result<bool, EngineError> {
        match polygon(rings)? {
            Some(shape) => Ok(self.prepared.intersects(&shape)?),
            None => Ok(false),
        }
    }
}
