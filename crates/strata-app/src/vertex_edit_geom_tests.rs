//! The vertices of a shape — values ported from the C++ tests named at each test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_storage::wkb_codec::WkbGeometry;

use super::*;

fn ring(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]
}

fn square_with_hole() -> Vec<WkbGeometry> {
    vec![WkbGeometry::Polygon(vec![ring(0.0, 0.0, 100.0, 100.0), ring(40.0, 40.0, 60.0, 60.0)])]
}

/// ported from tests/test_polygon_erase.cpp:361-388 (movingTheFirstVertexKeepsAHoleClosed): moving the first
/// point of a ring takes its closing point along, for the outer ring and for a hole.
#[test]
fn moving_the_first_vertex_keeps_its_ring_closed() {
    let shape = square_with_hole();
    assert_eq!(vertices(&shape).len(), 10);
    assert_eq!((ring_closing_vertex(&shape, 0), ring_closing_vertex(&shape, 4), ring_closing_vertex(&shape, 5), ring_closing_vertex(&shape, 9)), (Some(4), Some(0), Some(9), Some(5)));
    assert_eq!(ring_closing_vertex(&shape, 2), None);

    let moved = with_vertex_moved(&shape, 0, [-5.0, -5.0]).unwrap();
    let points = vertices(&moved);
    assert_eq!((points[0], points[4]), ([-5.0, -5.0], [-5.0, -5.0]));
    let moved = with_vertex_moved(&moved, 5, [35.0, 40.0]).unwrap();
    let points = vertices(&moved);
    assert_eq!((points[5], points[9]), ([35.0, 40.0], [35.0, 40.0]));
    assert_eq!(points[1], [100.0, 0.0], "the other points stay");
    assert!(with_vertex_moved(&shape, 10, [0.0, 0.0]).is_none());
}

/// ported from tests/test_buffer.cpp:199-223 (moveFeatureVertex_updatesPolygonCorner): the corner (10, 10) of
/// a square goes to (20, 10); a line has no closing point.
#[test]
fn a_corner_of_a_square_and_a_point_of_a_line_are_moved() {
    let square = vec![WkbGeometry::Polygon(vec![ring(0.0, 0.0, 10.0, 10.0)])];
    let corner = vertex_near(&square, [10.0, 10.0], 0.5).unwrap();
    let moved = with_vertex_moved(&square, corner, [20.0, 10.0]).unwrap();
    assert!(vertices(&moved).iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max) == 20.0);
    let line = vec![WkbGeometry::LineString(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]])];
    assert_eq!(ring_closing_vertex(&line, 0), None);
    assert_eq!(vertices(&with_vertex_moved(&line, 0, [1.0, 1.0]).unwrap()), [[1.0, 1.0], [10.0, 0.0], [10.0, 10.0]]);
}

/// `vertexNear`: the nearest vertex within the tolerance, and of equally near ones the last (the closing
/// point of a ring); `segmentNear`: the end of the nearest segment and the point on it.
#[test]
fn the_vertex_and_the_segment_under_the_pointer() {
    let shape = square_with_hole();
    assert_eq!(vertex_near(&shape, [99.0, 1.0], 2.0), Some(1));
    assert_eq!(vertex_near(&shape, [0.5, 0.5], 2.0), Some(4), "the closing point of the outer ring");
    assert_eq!(vertex_near(&shape, [50.0, 50.0], 2.0), None);
    assert_eq!(segment_near(&shape, [50.0, 1.0], 2.0), Some((1, [50.0, 0.0])));
    assert_eq!(segment_near(&shape, [61.0, 50.0], 2.0), Some((7, [60.0, 50.0])), "a side of the hole");
    assert_eq!(segment_near(&shape, [20.0, 20.0], 2.0), None);
}

/// A new vertex goes in before the end of its segment (`insertVertexAt`), in its own ring.
#[test]
fn a_vertex_is_put_into_a_segment() {
    let shape = square_with_hole();
    let (before, at) = segment_near(&shape, [50.0, 1.0], 2.0).unwrap();
    let wider = with_vertex_inserted(&shape, before, at).unwrap();
    assert_eq!(vertices(&wider).len(), 11);
    assert_eq!(vertices(&wider)[..3], [[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]]);
    let (before, at) = segment_near(&shape, [61.0, 50.0], 2.0).unwrap();
    let WkbGeometry::Polygon(rings) = &with_vertex_inserted(&shape, before, at).unwrap()[0] else { panic!("polygon") };
    assert_eq!((rings[0].len(), rings[1].len()), (5, 6));
    assert_eq!(rings[1][2], [60.0, 50.0]);
}

/// ported from tests/test_editing_tools.cpp:228-267 (clickedHandleIsPickedAndDeleteRemovesOnlyThatVertex): a
/// five-cornered shape loses the corner (6 points become 5). A ring keeps three corners, a line two points,
/// and only the ring of the vertex is counted: a triangle with a hole does not lose a corner because the
/// hole has points to spare (KaVertexEditTool.cpp:361-373). Taking out the first point closes the ring on
/// the next one.
#[test]
fn a_vertex_is_taken_out_down_to_the_fewest() {
    let five = vec![WkbGeometry::Polygon(vec![vec![[200_020.0, 450_020.0], [200_060.0, 450_020.0], [200_070.0, 450_045.0], [200_040.0, 450_065.0], [200_015.0, 450_045.0], [200_020.0, 450_020.0]]])];
    let four = without_vertex(&five, 1).unwrap();
    assert_eq!(vertices(&four).len(), 5);
    assert!(!vertices(&four).contains(&[200_060.0, 450_020.0]));

    let first_gone = without_vertex(&five, 0).unwrap();
    let points = vertices(&first_gone);
    assert_eq!((points.len(), points[0], points[4]), (5, [200_060.0, 450_020.0], [200_060.0, 450_020.0]));
    assert_eq!(vertices(&without_vertex(&five, 5).unwrap()), points, "the closing point is the same corner");

    let triangle = vec![WkbGeometry::Polygon(vec![vec![[0.0, 0.0], [100.0, 0.0], [50.0, 100.0], [0.0, 0.0]], ring(40.0, 20.0, 60.0, 40.0)])];
    assert_eq!(without_vertex(&triangle, 1), Err(FEWEST_OF_A_POLYGON));
    assert!(without_vertex(&triangle, 5).is_ok(), "the hole has five points: it may lose one");
    let line = vec![WkbGeometry::LineString(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]])];
    let shorter = without_vertex(&line, 1).unwrap();
    assert_eq!(vertices(&shorter), [[0.0, 0.0], [10.0, 10.0]]);
    assert_eq!(without_vertex(&shorter, 0), Err(FEWEST_OF_A_LINE));
}
