//! 「겹친 곳 지우기」: what remains of a polygon when another is taken out of it — C++ `PolygonErase::erased`,
//! `overlapWithArea` and the pieces of `partsFor` (core/PolygonErase.cpp:52-55, 92-110, 145-160).

use geo::{Area, BooleanOps, LineString, MultiPolygon, Polygon, Relate};

/// The rings of one polygon, the outer one first.
pub type Rings = Vec<Vec<[f64; 2]>>;

fn shape(parts: &[Rings]) -> MultiPolygon<f64> {
    let line = |ring: &Vec<[f64; 2]>| LineString::from(ring.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>());
    MultiPolygon(parts.iter().filter(|rings| !rings.is_empty()).map(|rings| Polygon::new(line(&rings[0]), rings[1..].iter().map(line).collect())).collect())
}

fn rings_of(polygon: &Polygon<f64>) -> Rings {
    let points = |line: &LineString<f64>| line.coords().map(|c| [c.x, c.y]).collect();
    std::iter::once(polygon.exterior()).chain(polygon.interiors()).map(points).collect()
}

/// Whether two shapes share more than their edges (`overlapWithArea`: they intersect and do not merely
/// touch).
#[must_use]
pub fn overlap_with_area(a: &[Rings], b: &[Rings]) -> bool {
    let relation = shape(a).relate(&shape(b));
    relation.is_intersects() && !relation.is_touches()
}

/// What remains of `target` when `cutter` is taken out of it, as polygons, the largest first. Empty when
/// nothing remains but a sliver of rounding (a billionth of the area).
#[must_use]
pub fn erased(target: &[Rings], cutter: &[Rings]) -> Vec<Rings> {
    let from = shape(target);
    let whole = from.unsigned_area();
    let rest = from.difference(&shape(cutter));
    if whole.is_nan() || rest.unsigned_area() <= whole * 1e-9 {
        return Vec::new();
    }
    let mut pieces: Vec<&Polygon<f64>> = rest.0.iter().filter(|piece| piece.unsigned_area() > 0.0).collect();
    pieces.sort_by(|a, b| b.unsigned_area().total_cmp(&a.unsigned_area()));
    pieces.into_iter().map(rings_of).collect()
}

/// The shapes taken together as one (`QgsGeometry::unaryUnion`, GeometryEditMerge.cpp:126-136): the pieces
/// of the union, the largest first. Shapes that touch or overlap become one piece; shapes apart stay apart.
#[must_use]
pub fn merged(shapes: &[Vec<Rings>]) -> Vec<Rings> {
    let mut whole = MultiPolygon::<f64>(Vec::new());
    for one in shapes {
        whole = whole.union(&shape(one));
    }
    let mut pieces: Vec<&Polygon<f64>> = whole.0.iter().filter(|piece| piece.unsigned_area() > 0.0).collect();
    pieces.sort_by(|a, b| b.unsigned_area().total_cmp(&a.unsigned_area()));
    pieces.into_iter().map(rings_of).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Rings> {
        vec![vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]]
    }

    fn area(piece: &Rings) -> f64 {
        let ring = |r: &Vec<[f64; 2]>| (r.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>() * 0.5).abs();
        ring(&piece[0]) - piece[1..].iter().map(ring).sum::<f64>()
    }

    /// ported from tests/test_polygon_erase.cpp:70-85 (erased_cutsOnlyTheOverlappingPlace): a shape inside
    /// leaves a hole, one across the edge a notch, one that covers all leaves nothing, one apart changes
    /// nothing.
    #[test]
    fn erasing_cuts_only_the_overlapping_place() {
        let outer = rect(0.0, 0.0, 100.0, 100.0);
        let inside = erased(&outer, &rect(40.0, 40.0, 60.0, 60.0));
        assert_eq!(inside.len(), 1);
        assert!((area(&inside[0]) - 9600.0).abs() < 1e-6 && inside[0].len() == 2, "one hole: {inside:?}");
        let notch = erased(&outer, &rect(80.0, 40.0, 120.0, 60.0));
        assert!(notch.len() == 1 && (area(&notch[0]) - 9600.0).abs() < 1e-6 && notch[0].len() == 1, "{notch:?}");
        assert!(erased(&outer, &rect(-10.0, -10.0, 110.0, 110.0)).is_empty());
        let apart = erased(&outer, &rect(200.0, 0.0, 300.0, 100.0));
        assert!(apart.len() == 1 && (area(&apart[0]) - 10_000.0).abs() < 1e-6);
    }

    /// A band across a shape leaves two pieces (ported from :282-296: 8,000 of the 10,000 remain), the
    /// larger first.
    #[test]
    fn a_band_across_leaves_two_pieces_the_larger_first() {
        let pieces = erased(&rect(0.0, 0.0, 100.0, 100.0), &rect(30.0, -10.0, 50.0, 110.0));
        assert_eq!(pieces.len(), 2);
        assert!((area(&pieces[0]) - 5000.0).abs() < 1e-6 && (area(&pieces[1]) - 3000.0).abs() < 1e-6, "{pieces:?}");
    }

    /// Shapes that only share an edge or a corner do not overlap; shapes apart do not either.
    #[test]
    fn touching_is_not_overlapping() {
        let outer = rect(0.0, 0.0, 100.0, 100.0);
        assert!(overlap_with_area(&outer, &rect(40.0, 40.0, 60.0, 60.0)));
        assert!(overlap_with_area(&outer, &rect(80.0, 40.0, 120.0, 60.0)));
        assert!(!overlap_with_area(&outer, &rect(100.0, 0.0, 200.0, 100.0)), "a shared edge");
        assert!(!overlap_with_area(&outer, &rect(100.0, 100.0, 200.0, 200.0)), "a shared corner");
        assert!(!overlap_with_area(&outer, &rect(300.0, 0.0, 400.0, 100.0)));
    }

    /// ported from tests/test_layer_edits.cpp:54-106: two touching squares become one piece of their summed
    /// area; squares apart stay two pieces, the larger first.
    #[test]
    fn touching_shapes_merge_into_one_and_shapes_apart_stay_apart() {
        let one = merged(&[rect(0.0, 0.0, 10.0, 10.0), rect(10.0, 0.0, 20.0, 10.0)]);
        assert_eq!(one.len(), 1);
        assert!((area(&one[0]) - 200.0).abs() < 1e-6, "{one:?}");
        let apart = merged(&[rect(0.0, 0.0, 10.0, 10.0), rect(100.0, 100.0, 120.0, 120.0)]);
        assert_eq!(apart.len(), 2);
        assert!((area(&apart[0]) - 400.0).abs() < 1e-6 && (area(&apart[1]) - 100.0).abs() < 1e-6);
    }
}
