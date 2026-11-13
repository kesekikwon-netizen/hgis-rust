//! WKT 도형 검사 — C++ ChecklistStateGeometry.cpp 의 isGeosValid·area·「조사구역 안」(unaryUnion·buffer(0.5, 4)·contains).

use super::*;

#[test]
fn validity_and_area_of_any_geometry() {
    assert!(!wkt_is_valid("POLYGON ((0 0, 10 10, 10 0, 0 10, 0 0))").expect("bow-tie"));
    assert!(wkt_is_valid("POINT (1 2)").expect("point"));
    assert!(wkt_is_valid("LINESTRING (0 0, 5 5)").expect("line"));
    assert!((wkt_area("POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))").expect("area") - 100.0).abs() < 1e-12);
    assert_eq!(wkt_area("POLYGON ((0 0, 10 0, 20 0, 0 0))").expect("flat"), 0.0);
    assert!(wkt_is_valid("not wkt").is_err());
}

/// 조사구역 두 개를 합쳐 0.5 m 넓힌 안: 가장자리 0.3 m 밖은 안, 0.6 m 밖은 밖. 두 구역에 걸친 선도 안.
#[test]
fn survey_area_contains_with_half_metre_tolerance() {
    let area = SurveyArea::new(
        &["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))".to_owned(), "POLYGON ((10 0, 20 0, 20 10, 10 10, 10 0))".to_owned()],
        0.5,
    )
    .expect("union")
    .expect("non-empty");
    assert!(area.contains("POINT (20.3 5)").expect("contains"));
    assert!(!area.contains("POINT (20.6 5)").expect("contains"));
    assert!(area.contains("LINESTRING (2 5, 18 5)").expect("contains"));
    assert!(!area.contains("POLYGON ((15 5, 25 5, 25 8, 15 8, 15 5))").expect("contains"));
    assert!(SurveyArea::new(&[], 0.5).expect("empty").is_none());
}
