//! The area of a site for the badge placer: the pieces of the site that the sheet shows, with their box
//! and their size (`LayoutBadgePlacer.cpp:176-188`: a prepared polygon, its bounding box and its area).

use super::Rings;

/// The pieces of one site.
#[derive(Debug)]
pub(super) struct Site {
    pieces: Vec<Rings>,
    /// `[min x, min y, max x, max y]`.
    bounds: [f64; 4],
    pub(super) area: f64,
}

/// The signed area of a ring (shoelace).
fn ring_area(ring: &[[f64; 2]]) -> f64 {
    let mut twice = 0.0;
    for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        twice += a[0] * b[1] - b[0] * a[1];
    }
    twice * 0.5
}

/// Whether a point lies inside a ring (a ray to the east crosses its edge an odd number of times).
fn in_ring(ring: &[[f64; 2]], point: [f64; 2]) -> bool {
    let mut inside = false;
    for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        if (a[1] > point[1]) != (b[1] > point[1]) && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

impl Site {
    /// The area of these pieces; `None` when they have no ground (no piece, or pieces without size).
    pub(super) fn of(pieces: &[Rings]) -> Option<Self> {
        let points = || pieces.iter().flatten().flatten();
        let bounds = points().fold(None, |bounds: Option<[f64; 4]>, p| Some(bounds.map_or([p[0], p[1], p[0], p[1]], |b| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])))?;
        // The outer ring less its holes.
        let area: f64 = pieces.iter().map(|rings| rings.iter().enumerate().map(|(place, ring)| if place == 0 { ring_area(ring).abs() } else { -ring_area(ring).abs() }).sum::<f64>()).sum();
        (area > 0.0).then(|| Self { pieces: pieces.to_vec(), bounds, area })
    }

    /// Whether a point lies on the site.
    pub(super) fn holds(&self, point: [f64; 2]) -> bool {
        let [min_x, min_y, max_x, max_y] = self.bounds;
        if point[0] < min_x || point[0] > max_x || point[1] < min_y || point[1] > max_y {
            return false;
        }
        self.pieces.iter().any(|rings| rings.first().is_some_and(|outer| in_ring(outer, point)) && !rings.iter().skip(1).any(|hole| in_ring(hole, point)))
    }

    /// The distance from a point to the far corner of the site's box.
    pub(super) fn reach_from(&self, origin: [f64; 2]) -> f64 {
        let [min_x, min_y, max_x, max_y] = self.bounds;
        (min_x - origin[0]).abs().max((max_x - origin[0]).abs()).hypot((min_y - origin[1]).abs().max((max_y - origin[1]).abs()))
    }
}
