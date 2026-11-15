//! Expected values of `target/cpp-ref/tests/test_survey_scope_clip.cpp` (the three C++ cases, with their
//! coordinates), and the edges of the rules the C++ leaves to GEOS.

use super::*;
use crate::Transformer;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))")
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Vec<[f64; 2]>> {
    vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]
}

fn scope_5km(wkts: &[String]) -> Scope {
    Scope::around(wkts, 5000.0).expect("scope").expect("some")
}

/// noSurveyLeavesFeatures (:16-31): without a survey area there is no scope, and that is no error.
#[test]
fn no_survey_area_gives_no_scope() {
    assert!(Scope::around(&[], 5000.0).expect("no error").is_none());
    assert!(Scope::around(&["POLYGON EMPTY".to_owned()], 5000.0).expect("no error").is_none());
}

/// surveyBufferDropsFarFeature (:33-66): 근처 (201000, 450000) stays, 멀리 (220000, 470000) goes.
#[test]
fn a_near_point_touches_and_a_far_point_does_not() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let test = scope.test().expect("test");
    assert!(test.touches_point([201_000.0, 450_000.0]).expect("near"));
    assert!(!test.touches_point([220_000.0, 470_000.0]).expect("far"));
}

/// intersectingCopyKeepsNearLine (:68-101): the line that leaves the survey area is kept whole; the far one is not.
#[test]
fn a_near_line_touches_and_a_far_line_does_not() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_020.0, 450_020.0)]);
    let test = scope.test().expect("test");
    assert!(test.touches_line(&[[200_010.0, 450_010.0], [200_040.0, 450_040.0]]).expect("near"));
    assert!(!test.touches_line(&[[230_000.0, 480_000.0], [230_100.0, 480_100.0]]).expect("far"));
    // A line that only passes through the buffer, with both ends outside, also touches.
    assert!(test.touches_line(&[[190_000.0, 450_010.0], [210_000.0, 450_010.0]]).expect("through"));
}

/// The edge is the radius from the nearest side (straight parts of the buffer are exact).
#[test]
fn the_edge_lies_at_the_radius_from_a_side() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let test = scope.test().expect("test");
    assert!(test.touches_point([200_050.0 + 4999.0, 450_025.0]).expect("inside"));
    assert!(!test.touches_point([200_050.0 + 5001.0, 450_025.0]).expect("outside"));
    assert_eq!(scope.bbox().expect("bbox"), [195_000.0, 445_000.0, 205_050.0, 455_050.0]);
}

/// Round corners are made of 24 straight pieces per quarter circle (`SurveyScopeClip.cpp:72`): half-way
/// between two corner points the edge lies at 5000 * cos(90deg / 24 / 2) = 4997.32 m from the corner.
/// 8 pieces would put it at 4975.9 m, 48 pieces at 4999.33 m.
#[test]
fn a_round_corner_has_24_pieces_per_quarter() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let test = scope.test().expect("test");
    let half_piece = (90.0_f64 / 24.0 / 2.0).to_radians();
    let at = |d: f64| [200_050.0 + d * half_piece.cos(), 450_050.0 + d * half_piece.sin()];
    assert!(test.touches_point(at(4996.0)).expect("inside the chord"));
    assert!(!test.touches_point(at(4998.5)).expect("outside the chord, inside the true circle"));
}

/// Several survey areas are one scope (`QgsGeometry::unaryUnion`, :69).
#[test]
fn two_survey_areas_are_joined() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0), rect(230_000.0, 450_000.0, 230_050.0, 450_050.0)]);
    let test = scope.test().expect("test");
    assert!(test.touches_point([203_000.0, 450_000.0]).expect("first"));
    assert!(test.touches_point([233_000.0, 450_000.0]).expect("second"));
    assert!(!test.touches_point([215_000.0, 450_000.0]).expect("between"));
    assert_eq!(scope.polygons().expect("polygons").len(), 2);
}

/// A shape larger than the scope is cut to it; one inside stays as it is; one outside leaves nothing.
#[test]
fn a_polygon_is_cut_to_the_scope() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let whole = scope.area().expect("area");
    let area = |parts: &[Vec<Vec<[f64; 2]>>]| parts.iter().map(|p| crate::polygon_area(p).expect("area")).sum::<f64>();

    let huge = scope.cut_polygon(&square(100_000.0, 350_000.0, 300_000.0, 550_000.0)).expect("cut");
    assert!((area(&huge) - whole).abs() < 1e-3, "{} against {whole}", area(&huge));

    let inside = scope.cut_polygon(&square(200_100.0, 450_100.0, 200_200.0, 450_200.0)).expect("cut");
    assert!((area(&inside) - 10_000.0).abs() < 1e-6);

    // Half of this square lies beyond the straight east edge at x = 205050.
    let half = scope.cut_polygon(&square(205_000.0, 450_000.0, 205_100.0, 450_050.0)).expect("cut");
    assert!((area(&half) - 50.0 * 50.0).abs() < 1e-6, "{}", area(&half));

    assert!(scope.cut_polygon(&square(230_000.0, 480_000.0, 230_100.0, 480_100.0)).expect("cut").is_empty());
}

/// A hole of the shape stays a hole after the cut.
#[test]
fn a_hole_survives_the_cut() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let mut with_hole = square(200_100.0, 450_100.0, 200_300.0, 450_300.0);
    with_hole.push(vec![[200_150.0, 450_150.0], [200_150.0, 450_250.0], [200_250.0, 450_250.0], [200_250.0, 450_150.0], [200_150.0, 450_150.0]]);
    let cut = scope.cut_polygon(&with_hole).expect("cut");
    assert_eq!(cut.len(), 1);
    assert_eq!(cut[0].len(), 2, "outer ring and the hole");
    assert!((crate::polygon_area(&cut[0]).expect("area") - (200.0 * 200.0 - 100.0 * 100.0)).abs() < 1e-6);
}

/// A line is cut to the pieces inside; here the chord along y = 450025 from x = 195000 to x = 205050.
#[test]
fn a_line_is_cut_to_the_scope() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let pieces = scope.cut_line(&[[180_000.0, 450_025.0], [220_000.0, 450_025.0]]).expect("cut");
    assert_eq!(pieces.len(), 1);
    let (first, last) = (pieces[0][0], pieces[0][pieces[0].len() - 1]);
    let (west, east) = (first[0].min(last[0]), first[0].max(last[0]));
    assert!((west - 195_000.0).abs() < 1e-6 && (east - 205_050.0).abs() < 1e-6, "{west} {east}");
    assert!(scope.cut_line(&[[230_000.0, 480_000.0], [230_100.0, 480_100.0]]).expect("cut").is_empty());
}

/// A shape that crosses itself (seen in served map data) is mended before the cut instead of failing.
#[test]
fn a_self_crossing_polygon_is_cut_all_the_same() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let bow_tie = vec![vec![[200_100.0, 450_100.0], [200_300.0, 450_300.0], [200_300.0, 450_100.0], [200_100.0, 450_300.0], [200_100.0, 450_100.0]]];
    let cut = scope.cut_polygon(&bow_tie).expect("cut");
    let area: f64 = cut.iter().map(|p| crate::polygon_area(p).expect("area")).sum();
    assert!((area - 2.0 * (200.0 * 100.0 / 2.0)).abs() < 1e-6, "{area}");
}

/// The scope carried into another CRS (work CRS EPSG:5187, data in EPSG:5186) keeps its edge within a
/// centimetre: every corner point is carried by PROJ, and a 330 m piece between two of them bends by far
/// less than a millimetre between two neighbouring belts.
#[test]
fn the_scope_in_another_crs_keeps_its_edge() {
    // A survey area near Gyeongju in EPSG:5187 (east belt).
    let scope = scope_5km(&[rect(220_000.0, 370_000.0, 220_050.0, 370_050.0)]);
    let central = scope.reprojected(5187, 5186).expect("reprojected");
    let to_central = Transformer::new(5187, 5186).expect("transformer");
    let test = central.test().expect("test");
    let inside = to_central.transform([220_050.0 + 4999.9, 370_025.0]).expect("point");
    let outside = to_central.transform([220_050.0 + 5000.1, 370_025.0]).expect("point");
    assert!(test.touches_point(inside).expect("inside"));
    assert!(!test.touches_point(outside).expect("outside"));
    // The area changes only by the scale difference of the two projections (below 2 parts in 1000 here).
    let ratio = central.area().expect("area") / scope.area().expect("area");
    assert!((ratio - 1.0).abs() < 2e-3, "{ratio}");
}

/// A radius that is not a positive number is refused with the C++ words (`SurveyScopeClip.cpp:24-27`).
#[test]
fn a_bad_radius_is_refused() {
    let shape = [rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)];
    for radius in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let error = Scope::around(&shape, radius).expect_err("refused");
        assert_eq!(error.0, "조사 주변 범위를 만들지 못했습니다.");
    }
}

/// A polygon touches when any part of it lies in the scope: one that reaches in, one that holds the whole
/// scope inside it, but not one that lies apart (parcels are kept whole by this test, `SurveyScopeClip.h:11`).
#[test]
fn a_polygon_touches_when_any_part_lies_inside() {
    let scope = scope_5km(&[rect(200_000.0, 450_000.0, 200_050.0, 450_050.0)]);
    let test = scope.test().expect("test");
    assert!(test.touches_polygon(&square(205_000.0, 450_000.0, 205_100.0, 450_050.0)).expect("reaches in"));
    assert!(test.touches_polygon(&square(100_000.0, 350_000.0, 300_000.0, 550_000.0)).expect("holds the scope"));
    assert!(!test.touches_polygon(&square(205_060.0, 450_000.0, 205_100.0, 450_050.0)).expect("apart"));
    assert!(!test.touches_polygon(&[]).expect("no ring"));
}

/// An area that is taken as it is (one that was cut out before): its shapes are joined and not grown.
#[test]
fn shapes_taken_as_they_are_are_joined_and_not_grown() {
    let scope = Scope::of(&[rect(0.0, 0.0, 100.0, 100.0), rect(100.0, 0.0, 200.0, 100.0)]).expect("scope").expect("some");
    assert!((scope.area().expect("area") - 20_000.0).abs() < 1e-6);
    assert_eq!(scope.polygons().expect("polygons").len(), 1, "joined");
    let test = scope.test().expect("test");
    assert!(test.touches_point([150.0, 50.0]).expect("inside") && !test.touches_point([201.0, 50.0]).expect("outside"));
    assert!(Scope::of(&[]).expect("no error").is_none());
}
