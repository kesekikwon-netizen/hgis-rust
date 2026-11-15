//! 「폴리곤 나누기」: a shape cut along a line, and the place two shapes share — C++
//! `LayerOps::splitPolygonWithLine` (QgsGeometry::splitGeometry, core/LayerOps.cpp:800-874) and
//! `LayerOps::splitTwoOverlappingFeatures` (the intersection, :876-1019).

use geo::{Area, BooleanOps, LineString, MultiPolygon, Polygon};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::slice::FloatSlice;

use crate::polygon_erase::Rings;

fn open(ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut points = ring.to_vec();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

fn closed(contour: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut points = contour.to_vec();
    if let Some(first) = points.first().copied() {
        if points.last() != Some(&first) {
            points.push(first);
        }
    }
    points
}

fn area(rings: &Rings) -> f64 {
    let ring = |r: &Vec<[f64; 2]>| (r.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>() * 0.5).abs();
    rings.first().map_or(0.0, ring) - rings.iter().skip(1).map(ring).sum::<f64>()
}

fn largest_first(mut pieces: Vec<Rings>) -> Vec<Rings> {
    pieces.retain(|piece| area(piece) > 0.0);
    pieces.sort_by(|a, b| area(b).total_cmp(&area(a)));
    pieces
}

/// The shape (its polygons) cut along the `line`: the pieces, the largest first. A line that does not cross
/// the shape from one side to the other leaves it in one piece per polygon.
#[must_use]
pub fn sliced(shape: &[Rings], line: &[[f64; 2]]) -> Vec<Rings> {
    let shapes: Vec<Vec<Vec<[f64; 2]>>> = shape.iter().map(|rings| rings.iter().map(|ring| open(ring)).collect()).collect();
    let pieces = shapes.slice_by(&line.to_vec(), FillRule::EvenOdd);
    largest_first(pieces.into_iter().map(|contours| contours.iter().map(|contour| closed(contour)).collect()).collect())
}

fn multi(parts: &[Rings]) -> MultiPolygon<f64> {
    let line = |ring: &Vec<[f64; 2]>| LineString::from(ring.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>());
    MultiPolygon(parts.iter().filter(|rings| !rings.is_empty()).map(|rings| Polygon::new(line(&rings[0]), rings[1..].iter().map(line).collect())).collect())
}

/// The place two shapes share, as polygons, the largest first; empty when they only touch or are apart.
#[must_use]
pub fn overlap(a: &[Rings], b: &[Rings]) -> Vec<Rings> {
    let shared = multi(a).intersection(&multi(b));
    let points = |line: &LineString<f64>| line.coords().map(|c| [c.x, c.y]).collect::<Vec<_>>();
    let pieces = shared.0.iter().filter(|piece| piece.unsigned_area() > 0.0).map(|piece| std::iter::once(piece.exterior()).chain(piece.interiors()).map(points).collect());
    largest_first(pieces.collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Rings> {
        vec![vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]]
    }

    /// ported from tests/test_buffer.cpp:280-314: a line that does not cross the 10 x 10 square leaves it whole;
    /// a line across it from below to above makes two pieces of 100 together.
    #[test]
    fn a_line_across_cuts_a_square_in_two_and_a_line_beside_it_does_not() {
        let square = rect(0.0, 0.0, 10.0, 10.0);
        assert_eq!(sliced(&square, &[[-5.0, -5.0], [-1.0, -1.0]]).len(), 1);
        let pieces = sliced(&square, &[[5.0, -2.0], [5.0, 12.0]]);
        assert_eq!(pieces.len(), 2, "{pieces:?}");
        assert!((area(&pieces[0]) + area(&pieces[1]) - 100.0).abs() < 0.1);
        let bent = sliced(&square, &[[3.0, -2.0], [3.0, 5.0], [7.0, 5.0], [7.0, 12.0]]);
        assert_eq!(bent.len(), 2, "a bent line cuts as well");
        assert!((area(&bent[0]) - 50.0).abs() < 0.1 && (area(&bent[1]) - 50.0).abs() < 0.1, "{bent:?}");
    }

    /// ported from tests/test_buffer.cpp:316-376: the 20 x 20 squares side by side share 10 x 20 = 200; squares
    /// apart share nothing.
    #[test]
    fn two_overlapping_squares_share_their_middle() {
        let shared = overlap(&rect(0.0, 0.0, 20.0, 20.0), &rect(10.0, 0.0, 30.0, 20.0));
        assert_eq!(shared.len(), 1);
        assert!((area(&shared[0]) - 200.0).abs() < 0.5);
        assert!(overlap(&rect(0.0, 0.0, 20.0, 20.0), &rect(100.0, 100.0, 110.0, 110.0)).is_empty());
    }
}
