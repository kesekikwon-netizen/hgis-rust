//! 주변 거리 경계(「주변 500m」) — C++ `BufferAnalysis::addDistanceRing` 의 도형 셈(core/BufferAnalysis.cpp):
//! 면들을 합치고(unaryUnion) 면만 남겨, 0.05 안의 겹친 꼭짓점을 빼고(removeDuplicateNodes), makeValid, 그다음
//! buffer(거리, 36 조각, 둥근 끝·둥근 이음, 마이터 2.0) 한 면마다 바깥 고리(길이 20 이상)를 경계선으로 한다.
//! 거리 글자 자리는 경계선 상자의 북·동·남·서 가운데 점에 가장 가까운 선 위 자리(lineLocatePoint)다.

use geos::{CapStyle, Geom, Geometry, JoinStyle};

use crate::geom::{collect_polygons, to_geos};
use crate::{EngineError, PolygonRings};

/// QgsLineString::removeDuplicateNodes(eps) — 앞 점과 x·y 가 eps 안이면 뺀다(닫는 끝 점은 남김). 고리가 4점 아래로
/// 줄면 그대로 둔다.
fn remove_duplicate_nodes(ring: &[[f64; 2]], eps: f64) -> Vec<[f64; 2]> {
    if ring.len() <= 2 {
        return ring.to_vec();
    }
    let mut out = vec![ring[0]];
    let last = ring.len() - 1;
    for (i, p) in ring.iter().enumerate().skip(1) {
        let prev = out[out.len() - 1];
        if (p[0] - prev[0]).abs() <= eps && (p[1] - prev[1]).abs() <= eps && i < last {
            continue;
        }
        out.push(*p);
    }
    if out.len() < 4 {
        return ring.to_vec();
    }
    out
}

fn multipolygon(parts: &[PolygonRings]) -> Result<Geometry, EngineError> {
    let geoms = parts.iter().map(to_geos).collect::<Result<Vec<_>, _>>()?;
    Ok(Geometry::create_multipolygon(geoms)?)
}

fn line_length(pts: &[[f64; 2]]) -> f64 {
    pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

/// 면 WKT 들 둘레의 거리 경계선들(닫힌 점 목록). 면이 없으면 빈 목록.
///
/// # Errors
/// WKT 를 읽지 못하거나 GEOS 가 셈을 하지 못함.
pub fn distance_rings(polygon_wkts: &[String], meters: f64) -> Result<Vec<Vec<[f64; 2]>>, EngineError> {
    if polygon_wkts.is_empty() {
        return Ok(Vec::new());
    }
    let parts = polygon_wkts.iter().map(|w| Geometry::new_from_wkt(w)).collect::<Result<Vec<_>, _>>()?;
    let union = Geometry::create_geometry_collection(parts)?.unary_union()?;
    let mut polys = Vec::new();
    collect_polygons(&union, &mut polys)?;
    if polys.is_empty() {
        return Ok(Vec::new());
    }
    let cleaned: Vec<PolygonRings> =
        polys.iter().map(|p| p.iter().map(|r| remove_duplicate_nodes(r, 0.05)).collect()).collect();
    let mut poly = multipolygon(&cleaned)?;
    let mut valid = Vec::new();
    collect_polygons(&poly.make_valid()?, &mut valid)?;
    if !valid.is_empty() {
        poly = multipolygon(&valid)?;
    }
    let buffered = poly.buffer_with_style(meters, 36, CapStyle::Round, JoinStyle::Round, 2.0)?;
    let mut out_polys = Vec::new();
    collect_polygons(&buffered, &mut out_polys)?;
    Ok(out_polys.into_iter().filter_map(|p| p.into_iter().next()).filter(|r| line_length(r) >= 20.0).collect())
}

/// 경계선 위 거리 글자 자리 — 상자 북·동·남·서 가운데 점마다 선 위 가장 가까운 자리(못 구하면 길이의 1/8·3/8·5/8·7/8).
///
/// # Errors
/// GEOS 가 선을 만들지 못함.
pub fn ring_label_points(ring: &[[f64; 2]]) -> Result<Vec<[f64; 2]>, EngineError> {
    if ring.len() < 2 {
        return Ok(Vec::new());
    }
    let coords: Vec<String> = ring.iter().map(|p| format!("{} {}", p[0], p[1])).collect();
    let line = Geometry::new_from_wkt(&format!("LINESTRING ({})", coords.join(", ")))?;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in ring {
        (x0, y0, x1, y1) = (x0.min(p[0]), y0.min(p[1]), x1.max(p[0]), y1.max(p[1]));
    }
    let (mx, my) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let mut cuts = Vec::new();
    for (x, y) in [(mx, y1), (x1, my), (mx, y0), (x0, my)] {
        let d = line.project(&Geometry::new_from_wkt(&format!("POINT ({x} {y})"))?)?;
        if d >= 0.0 && d.is_finite() {
            cuts.push(d);
        }
    }
    if cuts.is_empty() {
        let len = line_length(ring);
        cuts = vec![len * 0.125, len * 0.375, len * 0.625, len * 0.875];
    }
    cuts.into_iter()
        .map(|d| {
            let p = line.interpolate(d)?;
            Ok([p.get_x()?, p.get_y()?])
        })
        .collect()
}

#[cfg(test)]
#[path = "rings_tests.rs"]
mod tests;
