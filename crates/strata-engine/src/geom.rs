//! 면 도형 검사·고치기 — GEOS. QGIS `QgsGeometry::isGeosValid`(GEOSisValid)·`makeValid()`
//! (QgsGeos::makeValid: GEOSMakeValidWithParams, linework, keepCollapsed = false)와 같은 함수.
//! 넓이·둘레도 GEOS 로 잰다(QGIS 와 같은 평면 셈).

use geos::{Geom, Geometry, GeometryTypes, MakeValidMethod, MakeValidParams};

use crate::EngineError;

/// 면 하나: 바깥 고리 다음 구멍들. 고리는 닫힌 점 목록(첫 점 = 끝 점).
pub type PolygonRings = Vec<Vec<[f64; 2]>>;

fn ring_wkt(ring: &[[f64; 2]]) -> String {
    let pts: Vec<String> = ring.iter().map(|p| format!("{} {}", p[0], p[1])).collect();
    format!("({})", pts.join(", "))
}

fn to_geos(rings: &PolygonRings) -> Result<Geometry, EngineError> {
    let body: Vec<String> = rings.iter().map(|r| ring_wkt(r)).collect();
    Ok(Geometry::new_from_wkt(&format!("POLYGON ({})", body.join(", ")))?)
}

fn ring_points<G: Geom>(ring: &G) -> Result<Vec<[f64; 2]>, EngineError> {
    let seq = ring.get_coord_seq()?;
    (0..seq.size()?).map(|i| Ok([seq.get_x(i)?, seq.get_y(i)?])).collect()
}

fn polygon_rings<G: Geom>(poly: &G) -> Result<PolygonRings, EngineError> {
    let mut out = vec![ring_points(&poly.get_exterior_ring()?)?];
    for i in 0..poly.get_num_interior_rings()? {
        out.push(ring_points(&poly.get_interior_ring_n(i)?)?);
    }
    Ok(out)
}

/// QGIS EditGeometryRepair `collectPolygons` — 면이면 그대로, 다중 면은 조각마다, 묶음은 안을 차례로.
/// 선·점은 버린다.
fn collect_polygons<G: Geom>(g: &G, out: &mut Vec<PolygonRings>) -> Result<(), EngineError> {
    match g.geometry_type()? {
        GeometryTypes::Polygon => {
            if !g.is_empty()? {
                out.push(polygon_rings(g)?);
            }
        }
        GeometryTypes::MultiPolygon | GeometryTypes::GeometryCollection => {
            for i in 0..g.get_num_geometries()? {
                collect_polygons(&g.get_geometry_n(i)?, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// `isGeosValid`.
///
/// # Errors
/// GEOS 가 도형을 만들지 못함.
pub fn polygon_is_valid(rings: &PolygonRings) -> Result<bool, EngineError> {
    Ok(to_geos(rings)?.is_valid()?)
}

/// `makeValid()` 뒤 면 조각들(GEOS 가 내놓은 차례).
///
/// # Errors
/// GEOS 가 고치지 못함.
pub fn make_valid_polygon_parts(rings: &PolygonRings) -> Result<Vec<PolygonRings>, EngineError> {
    let mut params = MakeValidParams::new()?;
    params.set_method(MakeValidMethod::Linework)?;
    params.set_keep_collapsed(false)?;
    let fixed = to_geos(rings)?.make_valid_with_params(&params)?;
    let mut out = Vec::new();
    collect_polygons(&fixed, &mut out)?;
    Ok(out)
}

/// 평면 넓이.
///
/// # Errors
/// GEOS 가 도형을 만들지 못함.
pub fn polygon_area(rings: &PolygonRings) -> Result<f64, EngineError> {
    Ok(to_geos(rings)?.area()?)
}

/// 둘레(모든 고리 길이의 합 — QgsCurvePolygon::perimeter).
///
/// # Errors
/// GEOS 가 도형을 만들지 못함.
pub fn polygon_perimeter(rings: &PolygonRings) -> Result<f64, EngineError> {
    Ok(to_geos(rings)?.length()?)
}

/// WKT 를 GEOS 로 정규화(normalize)한 글 — 판마다 다를 수 있는 고리 시작점·방향을 맞춰 비교한다.
///
/// # Errors
/// WKT 를 읽지 못함.
pub fn normalized_wkt(wkt: &str) -> Result<String, EngineError> {
    let mut g = Geometry::new_from_wkt(wkt)?;
    g.normalize()?;
    Ok(g.to_wkt()?)
}

#[cfg(test)]
#[path = "geom_tests.rs"]
mod tests;
