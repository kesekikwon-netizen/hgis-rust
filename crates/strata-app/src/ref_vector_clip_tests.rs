//! The two cuts against hand-worked cases.

use super::*;

const BOX: ClipBox = [0.0, 0.0, 100.0, 50.0];

fn inside(b: &ClipBox, p: [f64; 2]) -> bool {
    p[0] >= b[0] && p[0] <= b[2] && p[1] >= b[1] && p[1] <= b[3]
}

fn area(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum::<f64>().abs() / 2.0
}

#[test]
fn a_segment_inside_stays_as_it_is() {
    assert_eq!(clip_segment(&BOX, [10.0, 10.0], [90.0, 40.0]), Some(([10.0, 10.0], [90.0, 40.0])));
    // On the edge counts as inside.
    assert_eq!(clip_segment(&BOX, [0.0, 0.0], [100.0, 50.0]), Some(([0.0, 0.0], [100.0, 50.0])));
    assert!(inside(&BOX, [100.0, 50.0]) && !inside(&BOX, [100.0, 50.1]));
}

#[test]
fn a_segment_is_cut_where_it_crosses_the_sides() {
    // Through the left and the right side, along y = 25.
    assert_eq!(clip_segment(&BOX, [-1.0e9, 25.0], [1.0e9, 25.0]), Some(([0.0, 25.0], [100.0, 25.0])));
    // One end inside: only the other is moved, and the kept end is the very same number.
    assert_eq!(clip_segment(&BOX, [50.0, 25.0], [50.0, 1000.0]), Some(([50.0, 25.0], [50.0, 50.0])));
    // A slanted one: from (-50, 0) to (150, 100) enters at x = 0 (y = 25) and leaves at y = 50 (x = 50).
    assert_eq!(clip_segment(&BOX, [-50.0, 0.0], [150.0, 100.0]), Some(([0.0, 25.0], [50.0, 50.0])));
    // The direction of the segment is kept.
    assert_eq!(clip_segment(&BOX, [150.0, 100.0], [-50.0, 0.0]), Some(([50.0, 50.0], [0.0, 25.0])));
}

#[test]
fn a_segment_outside_gives_nothing() {
    assert_eq!(clip_segment(&BOX, [-10.0, -10.0], [-5.0, 200.0]), None);
    // Parallel to a side and beyond it.
    assert_eq!(clip_segment(&BOX, [0.0, 60.0], [100.0, 60.0]), None);
    // Passing a corner outside: from (90, 70) to (130, 30) crosses x = 100 at y = 60.
    assert_eq!(clip_segment(&BOX, [90.0, 70.0], [130.0, 30.0]), None);
    // A single point outside.
    assert_eq!(clip_segment(&BOX, [200.0, 200.0], [200.0, 200.0]), None);
}

#[test]
fn a_ring_larger_than_the_box_becomes_the_box() {
    let huge = [[-1.0e9, -1.0e9], [1.0e9, -1.0e9], [1.0e9, 1.0e9], [-1.0e9, 1.0e9], [-1.0e9, -1.0e9]];
    let cut = clip_ring(&BOX, &huge);
    assert!((area(&cut) - 5000.0).abs() < 1e-9, "{cut:?}");
    assert!(cut.iter().all(|p| inside(&BOX, *p)));
}

#[test]
fn a_ring_is_cut_to_the_part_inside() {
    // A triangle with one corner outside to the right: (50, 0), (150, 25), (50, 50). The side x = 100 cuts it
    // where the slanted sides reach y = 12.5 and y = 37.5: area 2500 - 625 = 1875.
    let cut = clip_ring(&BOX, &[[50.0, 0.0], [150.0, 25.0], [50.0, 50.0]]);
    assert!((area(&cut) - 1875.0).abs() < 1e-9, "{cut:?}");
    assert!(cut.contains(&[100.0, 12.5]) && cut.contains(&[100.0, 37.5]));
    // A ring inside is returned without its repeated first point.
    let small = [[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 10.0]];
    assert_eq!(clip_ring(&BOX, &small), [[10.0, 10.0], [20.0, 10.0], [20.0, 20.0]]);
    // A ring outside leaves nothing.
    assert!(clip_ring(&BOX, &[[200.0, 0.0], [300.0, 0.0], [300.0, 50.0]]).is_empty());
    assert!(clip_ring(&BOX, &[]).is_empty());
}

/// A notch that leaves and comes back: the cut lays a side along the box, the area is still that of the part
/// inside. U shape: outer (0,0)-(100,0)-(100,80)-(70,80)-(70,20)-(30,20)-(30,80)-(0,80); inside y <= 50 its
/// area is 100 x 50 minus the notch 40 x 30.
#[test]
fn a_ring_that_leaves_twice_keeps_the_right_area() {
    let u = [[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [70.0, 80.0], [70.0, 20.0], [30.0, 20.0], [30.0, 80.0], [0.0, 80.0]];
    let cut = clip_ring(&BOX, &u);
    assert!((area(&cut) - (5000.0 - 1200.0)).abs() < 1e-9, "{cut:?}");
}
