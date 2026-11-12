//! GEOS 검사·고치기가 C++ 앱과 같은 엔진(OSGeo4W geosop)과 같은지. 기준값:
//! docs/senior/ref-files/geos-makevalid-reference.txt(hgis-2e 가 geosop 으로 만든 값). GEOS 판이 달라
//! 고리 시작점·방향이 다를 수 있으므로 GEOS normalize 뒤 equalsExact 로 견준다.

use geos::{Geom, Geometry};

use super::*;

/// (입력, isValid, makeValid, 입력 넓이).
const CASES: [(&str, bool, &str, f64); 6] = [
    (
        "POLYGON((0 0,10 10,10 0,0 10,0 0))",
        false,
        "MULTIPOLYGON (((5 5, 0 0, 0 10, 5 5)), ((5 5, 10 10, 10 0, 5 5)))",
        0.0,
    ),
    (
        "POLYGON((0 0,10 0,10 10,5 10,5 20,5 10,0 10,0 0))",
        false,
        "GEOMETRYCOLLECTION (POLYGON ((5 10, 10 10, 10 0, 0 0, 0 10, 5 10)), LINESTRING (5 10, 5 20))",
        100.0,
    ),
    (
        "POLYGON((0 0,10 0,10 0,10 10,0 10,0 0))",
        true,
        "POLYGON ((0 0, 10 0, 10 0, 10 10, 0 10, 0 0))",
        100.0,
    ),
    (
        "POLYGON((0 0,20 0,20 10,0 10,0 0,5 2,15 2,15 8,5 8,5 2,0 0))",
        false,
        "GEOMETRYCOLLECTION (POLYGON ((0 0, 0 10, 20 10, 20 0, 0 0), (5 2, 15 2, 15 8, 5 8, 5 2)), LINESTRING (0 0, 5 2))",
        260.0,
    ),
    (
        "POLYGON((0 0,10 0,10 10,0 10,0 0),(5 5,15 5,15 15,5 15,5 5))",
        false,
        "MULTIPOLYGON (((10 5, 10 0, 0 0, 0 10, 5 10, 5 5, 10 5)), ((5 10, 5 15, 15 15, 15 5, 10 5, 10 10, 5 10)))",
        0.0,
    ),
    (
        "POLYGON((0 0,30 0,30 30,0 30,0 0,10 10,20 10,20 20,10 20,10 10,0 0))",
        false,
        "GEOMETRYCOLLECTION (POLYGON ((0 0, 0 30, 30 30, 30 0, 0 0), (10 10, 20 10, 20 20, 10 20, 10 10)), LINESTRING (0 0, 10 10))",
        1000.0,
    ),
];

/// geosop 은 GEOSMakeValid(기본값 = linework). C++ 도 linework 다. keepCollapsed 는 structure 방법에만 쓰여
/// linework 는 무너진 조각(선)을 늘 남긴다 — 면 조각만 고르는 것은 collectPolygons(C++ 과 같음)가 한다.
fn make_valid_full(wkt: &str) -> Geometry {
    Geometry::new_from_wkt(wkt).expect("wkt").make_valid().expect("makeValid")
}

fn same_normalized(a: &Geometry, b_wkt: &str) -> bool {
    let mut x = Clone::clone(a);
    x.normalize().expect("normalize");
    let mut y = Geometry::new_from_wkt(b_wkt).expect("reference wkt");
    y.normalize().expect("normalize");
    x.equals_exact(&y, 0.0).expect("equalsExact")
}

fn rings_of(wkt: &str) -> PolygonRings {
    let g = Geometry::new_from_wkt(wkt).expect("wkt");
    let mut out = Vec::new();
    super::collect_polygons(&g, &mut out).expect("collect");
    out.into_iter().next().expect("one polygon")
}

/// isValid·makeValid·넓이가 geosop 과 같다(정규화 비교, 원문도 찍는다).
#[test]
fn make_valid_matches_geosop_references() {
    for (input, valid, want, area) in CASES {
        let rings = rings_of(input);
        assert_eq!(polygon_is_valid(&rings).expect("isValid"), valid, "{input}");
        assert!((polygon_area(&rings).expect("area") - area).abs() < 1e-9, "{input}");
        let got = make_valid_full(input);
        eprintln!("{input}\n  geosop: {want}\n  ours:   {}", got.to_wkt().expect("wkt"));
        assert!(same_normalized(&got, want), "{input}: {} vs {want}", got.to_wkt().expect("wkt"));
    }
}

/// 면 조각들을 정규화 WKT 로(차례는 GEOS 판마다 다를 수 있어 정렬).
fn normalized_parts(parts: &[PolygonRings]) -> Vec<String> {
    let mut out: Vec<String> = parts
        .iter()
        .map(|p| {
            let mut g = super::to_geos(p).expect("polygon");
            g.normalize().expect("normalize");
            g.to_wkt().expect("wkt")
        })
        .collect();
    out.sort();
    out
}

/// C++ 이 쓰는 꼴(linework, keepCollapsed = false)의 면 조각 하나하나가 geosop 결과의 면 조각과 같다.
#[test]
fn polygon_parts_are_the_polygons_of_make_valid() {
    for (input, _, want, _) in CASES {
        let parts = make_valid_polygon_parts(&rings_of(input)).expect("parts");
        let mut want_parts = Vec::new();
        super::collect_polygons(&Geometry::new_from_wkt(want).expect("wkt"), &mut want_parts).expect("collect");
        assert_eq!(normalized_parts(&parts), normalized_parts(&want_parts), "{input}");
    }
}

/// 둘레(고리 길이 합)와 정규화 글.
#[test]
fn perimeter_and_normalized_wkt() {
    let square = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]];
    assert!((polygon_perimeter(&square).expect("perimeter") - 40.0).abs() < 1e-12);
    let a = normalized_wkt("POLYGON ((10 0, 10 10, 0 10, 0 0, 10 0))").expect("a");
    let b = normalized_wkt("POLYGON ((0 0, 0 10, 10 10, 10 0, 0 0))").expect("b");
    assert_eq!(a, b);
}
