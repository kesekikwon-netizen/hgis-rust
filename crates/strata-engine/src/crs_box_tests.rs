//! The carried box against points carried one by one.

use crate::Transformer;

/// A 20 km square across the central meridian of the central belt, carried to longitude and latitude: the
/// box holds every point of the square's sides (1000 to a side) to within a centimetre (1e-7 degree) and
/// is no larger than they need. Its top side is highest in latitude in the middle, about 5 m above its two
/// ends, so the corners alone would give a box that is too small: the test measures that gap.
#[test]
fn the_box_holds_the_bent_sides() {
    let carry = Transformer::new(5186, 4326).expect("transformer");
    let rect = [190_000.0, 250_000.0, 210_000.0, 270_000.0];
    let boxed = carry.transform_box(rect).expect("box");
    let mut seen = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for i in 0..=1000 {
        let t = f64::from(i) / 1000.0;
        let (x, y) = (rect[0] + (rect[2] - rect[0]) * t, rect[1] + (rect[3] - rect[1]) * t);
        for p in [[x, rect[1]], [x, rect[3]], [rect[0], y], [rect[2], y]] {
            let q = carry.transform(p).expect("point");
            seen = [seen[0].min(q[0]), seen[1].min(q[1]), seen[2].max(q[0]), seen[3].max(q[1])];
        }
    }
    for (got, want) in boxed.iter().zip(seen) {
        assert!((got - want).abs() < 1e-7, "{boxed:?} against {seen:?}");
    }
    let corner = carry.transform([rect[0], rect[3]]).expect("point");
    let gap = boxed[3] - corner[1];
    assert!(gap > 3.0e-5 && gap < 7.0e-5, "the middle of the top side lies {gap} degrees north of its ends");
}

/// The same CRS on both sides gives the rectangle back.
#[test]
fn nothing_changes_within_one_crs() {
    let same = Transformer::new(5187, 5187).expect("transformer");
    let rect = [210_000.0, 360_000.0, 230_000.0, 380_000.0];
    let boxed = same.transform_box(rect).expect("box");
    for (got, want) in boxed.iter().zip(rect) {
        assert!((got - want).abs() < 1e-6);
    }
}

/// A rectangle that is a single point, or a line, is carried as what it is.
#[test]
fn a_rectangle_without_size_is_carried_as_its_points() {
    let carry = Transformer::new(5186, 5187).expect("transformer");
    let point = carry.transform([300_000.0, 250_000.0]).expect("point");
    let boxed = carry.transform_box([300_000.0, 250_000.0, 300_000.0, 250_000.0]).expect("box");
    assert!((boxed[0] - point[0]).abs() < 1e-9 && (boxed[2] - point[0]).abs() < 1e-9);
    assert!((boxed[1] - point[1]).abs() < 1e-9 && (boxed[3] - point[1]).abs() < 1e-9);
    let line = carry.transform_box([300_000.0, 250_000.0, 320_000.0, 250_000.0]).expect("box");
    assert!(line[2] - line[0] > 19_000.0 && line[3] - line[1] < 400.0);
}

#[test]
fn what_is_no_rectangle_is_refused() {
    let carry = Transformer::new(5186, 4326).expect("transformer");
    assert!(carry.transform_box([f64::NAN, 0.0, 1.0, 1.0]).is_err());
    assert!(carry.transform_box([0.0, 0.0, f64::INFINITY, 1.0]).is_err());
    assert!(carry.transform_box([2.0, 0.0, 1.0, 1.0]).is_err(), "max before min");
}
