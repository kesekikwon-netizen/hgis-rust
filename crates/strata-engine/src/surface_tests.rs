//! The inside point must be usable for reading a colour: inside the shape, outside its holes.

use super::*;
use crate::wkt_point_relation;

fn square(x: f64, y: f64, side: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side], [x, y]]
}

fn wkt(polygons: &[PolygonRings]) -> String {
    let ring = |r: &Vec<[f64; 2]>| format!("({})", r.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", "));
    let polygon = |p: &PolygonRings| format!("({})", p.iter().map(ring).collect::<Vec<_>>().join(", "));
    format!("MULTIPOLYGON ({})", polygons.iter().map(polygon).collect::<Vec<_>>().join(", "))
}

#[test]
fn the_area_counts_every_part_and_no_hole() {
    let ring_shape = vec![square(0.0, 0.0, 100.0), square(40.0, 40.0, 20.0)];
    let island = vec![square(500.0, 0.0, 10.0)];
    assert!((polygons_area(&[ring_shape, island]).expect("area") - (10_000.0 - 400.0 + 100.0)).abs() < 1e-9);
    assert_eq!(polygons_area(&[]).expect("area"), 0.0);
}

/// A square with a large hole in its middle: the centre is in the hole, the point must not be.
#[test]
fn the_point_lies_in_the_shape_and_not_in_its_hole() {
    let ring_shape = vec![vec![square(0.0, 0.0, 100.0), square(10.0, 10.0, 80.0)]];
    let point = point_on_surface(&ring_shape).expect("point").expect("some");
    let (inside, distance) = wkt_point_relation(&wkt(&ring_shape), point).expect("relation");
    assert!(inside && distance == 0.0, "{point:?}");
    let in_hole = (10.0..=90.0).contains(&point[0]) && (10.0..=90.0).contains(&point[1]);
    assert!(!in_hole, "{point:?}");
}

/// Two parts far apart: the point is in one of them, not between them (where the centre of gravity lies).
#[test]
fn the_point_lies_in_one_of_the_parts() {
    let parts = vec![vec![square(0.0, 0.0, 10.0)], vec![square(1000.0, 0.0, 10.0)]];
    let point = point_on_surface(&parts).expect("point").expect("some");
    assert!(wkt_point_relation(&wkt(&parts), point).expect("relation").0, "{point:?}");
    // A C shape: its centre of gravity lies in the opening.
    let c = vec![vec![vec![[0.0, 0.0], [100.0, 0.0], [100.0, 10.0], [10.0, 10.0], [10.0, 90.0], [100.0, 90.0], [100.0, 100.0], [0.0, 100.0], [0.0, 0.0]]]];
    let point = point_on_surface(&c).expect("point").expect("some");
    assert!(wkt_point_relation(&wkt(&c), point).expect("relation").0, "{point:?}");
}

#[test]
fn nothing_gives_no_point() {
    assert_eq!(point_on_surface(&[]).expect("point"), None);
}
