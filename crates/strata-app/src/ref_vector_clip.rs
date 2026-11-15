//! Cutting lines and rings to a rectangle around the picture before they are drawn.
//!
//! Zoomed far in, one side of a geology unit or a river line can run on for millions of pixels beside the
//! picture. The line drawer walks a line in steps of a few pixels and the rasteriser sets up every edge it
//! is given, so shapes are cut, in double precision, to a box a little larger than the picture first: the
//! work then follows what is in view, and every coordinate handed on is small. (The cut does not sharpen
//! anything: the stored points are single precision already, see `ref_vector_data.rs`.) Both algorithms are
//! the textbook ones, O(points):
//! lines — Liang and Barsky, "A new concept and method for line clipping", ACM TOG 3(1), 1984;
//! rings — Sutherland and Hodgman, "Reentrant polygon clipping", CACM 17(1), 1974.

#[cfg(test)]
#[path = "ref_vector_clip_tests.rs"]
mod tests;

/// `[min x, min y, max x, max y]`.
pub(crate) type ClipBox = [f64; 4];

/// The part of the segment `a`-`b` inside the box, if any (Liang-Barsky).
pub(crate) fn clip_segment(b: &ClipBox, a: [f64; 2], z: [f64; 2]) -> Option<([f64; 2], [f64; 2])> {
    let (dx, dy) = (z[0] - a[0], z[1] - a[1]);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    // For each side: p is the direction against it, q the distance to it.
    for (p, q) in [(-dx, a[0] - b[0]), (dx, b[2] - a[0]), (-dy, a[1] - b[1]), (dy, b[3] - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return None;
            }
        }
    }
    let at = |t: f64| [a[0] + t * dx, a[1] + t * dy];
    Some((if t0 > 0.0 { at(t0) } else { a }, if t1 < 1.0 { at(t1) } else { z }))
}

/// The ring cut to the box (Sutherland-Hodgman). The ring may be given closed or open; the result is open
/// (no repeated first point) and may be empty. Sides that the cut lays along the box add no area.
pub(crate) fn clip_ring(b: &ClipBox, ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut points: Vec<[f64; 2]> = ring.to_vec();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    // The four half planes: axis, limit, whether inside means at or above the limit.
    for (axis, limit, above) in [(0_usize, b[0], true), (0, b[2], false), (1, b[1], true), (1, b[3], false)] {
        if points.is_empty() {
            break;
        }
        let keep = |p: &[f64; 2]| if above { p[axis] >= limit } else { p[axis] <= limit };
        let cross = |p: &[f64; 2], q: &[f64; 2]| {
            let t = (limit - p[axis]) / (q[axis] - p[axis]);
            let mut c = [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])];
            c[axis] = limit;
            c
        };
        let mut out = Vec::with_capacity(points.len() + 4);
        let mut previous = points[points.len() - 1];
        for current in &points {
            match (keep(&previous), keep(current)) {
                (true, true) => out.push(*current),
                (true, false) => out.push(cross(&previous, current)),
                (false, true) => {
                    out.push(cross(&previous, current));
                    out.push(*current);
                }
                (false, false) => {}
            }
            previous = *current;
        }
        points = out;
    }
    points
}
