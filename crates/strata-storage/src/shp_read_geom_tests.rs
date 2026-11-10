//! `shp_read_geom` 시험 — 도형 종류별 변환과 범위 계산.

use strata_types::geom::{Extent, Geometry};

use super::*;

#[test]
fn point_variants_become_xy_points() {
    let p = shapefile::Shape::Point(shapefile::Point::new(1.5, -2.5));
    assert_eq!(shape_geometry(&p), Some(Geometry::Point([1.5, -2.5])));
    assert_eq!(shape_extent(&p), Some(Extent::point(1.5, -2.5)));

    // Z·M 점도 XY 만 쓴다(L0 Geometry 는 2D).
    let pz = shapefile::Shape::PointZ(shapefile::PointZ::new(3.0, 4.0, 99.0, 0.0));
    assert_eq!(shape_geometry(&pz), Some(Geometry::Point([3.0, 4.0])));
    assert_eq!(shape_extent(&pz), Some(Extent::point(3.0, 4.0)));
}

#[test]
fn multipart_polyline_keeps_first_part_but_full_extent() {
    let line = shapefile::Polyline::with_parts(vec![
        vec![
            shapefile::Point::new(0.0, 0.0),
            shapefile::Point::new(1.0, 1.0),
        ],
        vec![
            shapefile::Point::new(10.0, 20.0),
            shapefile::Point::new(30.0, 40.0),
        ],
    ]);
    let shape = shapefile::Shape::Polyline(line);
    match shape_geometry(&shape) {
        Some(Geometry::LineString(pts)) => {
            assert_eq!(pts, vec![[0.0, 0.0], [1.0, 1.0]], "L0 에 MultiLineString 이 없어 첫 조각만");
        }
        other => panic!("LineString 이어야 한다: {other:?}"),
    }
    assert_eq!(
        shape_extent(&shape),
        Some(Extent::new(0.0, 0.0, 30.0, 40.0)),
        "범위는 두 조각을 모두 담아야 한다"
    );
}

#[test]
fn polygon_with_two_outers_becomes_multipolygon() {
    let outer_a = vec![
        shapefile::Point::new(0.0, 0.0),
        shapefile::Point::new(0.0, 10.0),
        shapefile::Point::new(10.0, 10.0),
        shapefile::Point::new(10.0, 0.0),
        shapefile::Point::new(0.0, 0.0),
    ];
    let outer_b = vec![
        shapefile::Point::new(100.0, 100.0),
        shapefile::Point::new(100.0, 110.0),
        shapefile::Point::new(110.0, 110.0),
        shapefile::Point::new(110.0, 100.0),
        shapefile::Point::new(100.0, 100.0),
    ];
    let poly = shapefile::Polygon::with_rings(vec![
        shapefile::PolygonRing::Outer(outer_a),
        shapefile::PolygonRing::Outer(outer_b),
    ]);
    let shape = shapefile::Shape::Polygon(poly);
    match shape_geometry(&shape) {
        Some(Geometry::MultiPolygon(parts)) => assert_eq!(parts.len(), 2),
        other => panic!("MultiPolygon 이어야 한다: {other:?}"),
    }
    assert_eq!(
        shape_extent(&shape),
        Some(Extent::new(0.0, 0.0, 110.0, 110.0))
    );
}

#[test]
fn hole_only_polygon_does_not_lose_its_ring() {
    // 규격 위반(외곽 없는 구멍)이라도 링을 버리지 않는다. 범위 계산이 0 이 되면 안 된다.
    let hole = vec![
        shapefile::Point::new(5.0, 5.0),
        shapefile::Point::new(6.0, 5.0),
        shapefile::Point::new(6.0, 6.0),
        shapefile::Point::new(5.0, 5.0),
    ];
    let poly = shapefile::Polygon::with_rings(vec![shapefile::PolygonRing::Inner(hole)]);
    let shape = shapefile::Shape::Polygon(poly);
    assert_eq!(
        shape_extent(&shape),
        Some(Extent::new(5.0, 5.0, 6.0, 6.0)),
        "구멍 링만 있어도 범위는 그 링을 담는다"
    );
    assert!(matches!(
        shape_geometry(&shape),
        Some(Geometry::Polygon { .. })
    ));
}

#[test]
fn single_point_multipoint_becomes_point_and_multi_point_stays_none() {
    let one = shapefile::Multipoint::new(vec![shapefile::Point::new(7.0, 8.0)]);
    let shape = shapefile::Shape::Multipoint(one);
    assert_eq!(shape_geometry(&shape), Some(Geometry::Point([7.0, 8.0])));

    let two = shapefile::Multipoint::new(vec![
        shapefile::Point::new(1.0, 2.0),
        shapefile::Point::new(3.0, 4.0),
    ]);
    let shape = shapefile::Shape::Multipoint(two);
    assert_eq!(shape_geometry(&shape), None, "L0 에 MultiPoint 가 없다");
    assert_eq!(
        shape_extent(&shape),
        Some(Extent::new(1.0, 2.0, 3.0, 4.0)),
        "범위는 두 점을 모두 담는다"
    );
}

#[test]
fn null_shape_has_no_geometry_and_no_extent() {
    let shape = shapefile::Shape::NullShape;
    assert_eq!(shape_geometry(&shape), None);
    assert_eq!(shape_extent(&shape), None);
}

#[test]
fn non_finite_vertices_do_not_poison_the_extent() {
    let line = shapefile::Polyline::new(vec![
        shapefile::Point::new(f64::NAN, 0.0),
        shapefile::Point::new(1.0, 2.0),
    ]);
    let shape = shapefile::Shape::Polyline(line);
    assert_eq!(shape_extent(&shape), Some(Extent::point(1.0, 2.0)));
}
