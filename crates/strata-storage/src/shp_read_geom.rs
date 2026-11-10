//! SHP 도형 → L0 [`Geometry`] · [`Extent`] 변환.
//!
//! `shp_reader` 에서 분리했다(300줄 규칙).
//!
//! C++ 쪽에서는 QGIS OGR provider 가 `QgsGeometry` 를 만드는 자리다
//! (`D:\hgis\src\core\HeritageImport.cpp` 의 `QgsVectorLayer` → `QgsFeature`).
//! 여기서는 그 `QgsGeometry` 에 해당하는 `strata_types::geom::Geometry` 로 바로 옮긴다.
//!
//! ## 폴리곤 링 해석
//!
//! SHP 폴리곤은 링 목록만 주고 어느 것이 외곽인지 표시하지 않는다. 규격은 "시계방향 =
//! 외곽, 반시계방향 = 구멍" 이고 `shapefile` 크레이트가 이미 그 판정을 해서
//! `PolygonRing::Outer`/`Inner` 로 나눠 준다. 그래서 여기서 부호를 다시 계산하지 않는다.
//! 외곽 링이 여럿이면 각각을 하나의 폴리곤으로 보고 [`Geometry::MultiPolygon`] 으로 담는다
//! (한 외곽 안의 구멍은 그 외곽에 붙인다).

use strata_types::geom::{Extent, Geometry, PolygonRings};

/// 도형 하나를 L0 기하로. 담을 수 없는 종류는 `None`.
///
/// L0 에 MultiLineString·MultiPoint 가 없어서:
/// - 여러 조각 폴리라인 → **첫 조각만** `LineString`
/// - 점이 1개인 MultiPoint → `Point`, 2점 이상 → `None`
/// - NullShape·Multipatch → `None`
#[must_use]
pub fn shape_geometry(shape: &shapefile::Shape) -> Option<Geometry> {
    use shapefile::Shape;
    match shape {
        Shape::NullShape => None,
        Shape::Point(p) => Some(Geometry::Point([p.x, p.y])),
        Shape::PointM(p) => Some(Geometry::Point([p.x, p.y])),
        Shape::PointZ(p) => Some(Geometry::Point([p.x, p.y])),
        Shape::Polyline(pl) => first_part(pl.parts()),
        Shape::PolylineM(pl) => first_part(pl.parts()),
        Shape::PolylineZ(pl) => first_part(pl.parts()),
        Shape::Polygon(pg) => Some(polygon_geometry(pg.rings())),
        Shape::PolygonM(pg) => Some(polygon_geometry(pg.rings())),
        Shape::PolygonZ(pg) => Some(polygon_geometry(pg.rings())),
        Shape::Multipoint(mp) => multipoint_geometry(mp.points()),
        Shape::MultipointM(mp) => multipoint_geometry(mp.points()),
        Shape::MultipointZ(mp) => multipoint_geometry(mp.points()),
        Shape::Multipatch(_) => None,
    }
}

/// 도형 하나가 실제로 차지하는 범위. 모든 꼭짓점을 본다.
///
/// [`shape_geometry`] 가 `None` 을 주는 종류(여러 조각·다점 MultiPoint·Multipatch)도
/// 여기서는 빠짐없이 포함하므로 레이어 범위는 언제나 정확하다.
#[must_use]
pub fn shape_extent(shape: &shapefile::Shape) -> Option<Extent> {
    use shapefile::record::traits::HasXY;
    use shapefile::Shape;
    match shape {
        Shape::NullShape => None,
        Shape::Point(p) => single(p),
        Shape::PointM(p) => single(p),
        Shape::PointZ(p) => single(p),
        Shape::Polyline(pl) => parts_extent(pl.parts()),
        Shape::PolylineM(pl) => parts_extent(pl.parts()),
        Shape::PolylineZ(pl) => parts_extent(pl.parts()),
        Shape::Polygon(pg) => rings_extent(pg.rings()),
        Shape::PolygonM(pg) => rings_extent(pg.rings()),
        Shape::PolygonZ(pg) => rings_extent(pg.rings()),
        Shape::Multipoint(mp) => points_extent(mp.points()),
        Shape::MultipointM(mp) => points_extent(mp.points()),
        Shape::MultipointZ(mp) => points_extent(mp.points()),
        Shape::Multipatch(mp) => {
            let mut acc: Option<Extent> = None;
            for patch in mp.patches() {
                for p in patch.points() {
                    acc = grow(acc, p.x(), p.y());
                }
            }
            acc
        }
    }
}

fn single<P: shapefile::record::traits::HasXY>(p: &P) -> Option<Extent> {
    Some(Extent::point(p.x(), p.y()))
}

fn grow(acc: Option<Extent>, x: f64, y: f64) -> Option<Extent> {
    if !x.is_finite() || !y.is_finite() {
        return acc;
    }
    Some(match acc {
        Some(mut e) => {
            e.include_xy(x, y);
            e
        }
        None => Extent::point(x, y),
    })
}

fn first_part<P: shapefile::record::traits::HasXY>(parts: &[Vec<P>]) -> Option<Geometry> {
    let pts: Vec<[f64; 2]> = parts.first()?.iter().map(|p| [p.x(), p.y()]).collect();
    if pts.is_empty() {
        return None;
    }
    Some(Geometry::LineString(pts))
}

fn multipoint_geometry<P: shapefile::record::traits::HasXY>(pts: &[P]) -> Option<Geometry> {
    match pts {
        [only] => Some(Geometry::Point([only.x(), only.y()])),
        _ => None,
    }
}

fn points_extent<P: shapefile::record::traits::HasXY>(pts: &[P]) -> Option<Extent> {
    let mut acc: Option<Extent> = None;
    for p in pts {
        acc = grow(acc, p.x(), p.y());
    }
    acc
}

fn parts_extent<P: shapefile::record::traits::HasXY>(parts: &[Vec<P>]) -> Option<Extent> {
    let mut acc: Option<Extent> = None;
    for part in parts {
        for p in part {
            acc = grow(acc, p.x(), p.y());
        }
    }
    acc
}

fn rings_extent<P: shapefile::record::traits::HasXY>(
    rings: &[shapefile::PolygonRing<P>],
) -> Option<Extent> {
    let mut acc: Option<Extent> = None;
    for ring in rings {
        for p in ring.points() {
            acc = grow(acc, p.x(), p.y());
        }
    }
    acc
}

/// 링 목록을 외곽별 폴리곤으로 묶는다. 외곽이 하나면 `Polygon`, 여럿이면 `MultiPolygon`.
fn polygon_geometry<P: shapefile::record::traits::HasXY>(
    rings: &[shapefile::PolygonRing<P>],
) -> Geometry {
    let mut outers: Vec<PolygonRings> = Vec::new();
    for ring in rings {
        let pts = ring_points(ring.points());
        match ring {
            shapefile::PolygonRing::Outer(_) => outers.push((pts, Vec::new())),
            shapefile::PolygonRing::Inner(_) => match outers.last_mut() {
                Some(last) => last.1.push(pts),
                // 외곽 없는 구멍은 규격 위반이다. 버리지 않고 자체 외곽으로 세운다.
                None => outers.push((pts, Vec::new())),
            },
        }
    }
    match outers.len() {
        0 => Geometry::Polygon {
            exterior: Vec::new(),
            interiors: Vec::new(),
        },
        1 => {
            let (exterior, interiors) = outers.remove(0);
            Geometry::Polygon {
                exterior,
                interiors,
            }
        }
        _ => Geometry::MultiPolygon(outers),
    }
}

fn ring_points<P: shapefile::record::traits::HasXY>(pts: &[P]) -> Vec<[f64; 2]> {
    pts.iter().map(|p| [p.x(), p.y()]).collect()
}

#[cfg(test)]
#[path = "shp_read_geom_tests.rs"]
mod tests;
