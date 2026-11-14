//! Direct `f64` overlap bounds of a trench rectangle with a survey area (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:328-340`).

use geo::{Contains, Coord, MultiPolygon};

struct Crossing {
    xy: [f64; 2],
    proper: bool,
}

fn segment_crossing(p: [f64; 2], q: [f64; 2], a: Coord<f64>, b: Coord<f64>) -> Option<Crossing> {
    let (rx, ry) = (q[0] - p[0], q[1] - p[1]);
    let (sx, sy) = (b.x - a.x, b.y - a.y);
    let scale = rx.hypot(ry) * sx.hypot(sy);
    let denom = rx * sy - ry * sx;
    if scale == 0.0 || denom.abs() <= 1e-13 * scale {
        return None;
    }
    let (dx, dy) = (a.x - p[0], a.y - p[1]);
    let t = (dx * sy - dy * sx) / denom;
    let u = (dx * ry - dy * rx) / denom;
    const EPS: f64 = 1e-11;
    if (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u) {
        let tc = t.clamp(0.0, 1.0);
        let proper = t > EPS && t < 1.0 - EPS && u > EPS && u < 1.0 - EPS;
        Some(Crossing {
            xy: [p[0] + tc * rx, p[1] + tc * ry],
            proper,
        })
    } else {
        None
    }
}

/// Computes `(lo_v, hi_v)` of `area_mp ∩ rect` directly in `f64` from area vertices inside the
/// rectangle, rectangle corners inside the area, and edge-edge crossings (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:328-340`).
pub(super) fn trench_overlap_v_bounds(
    area_mp: &MultiPolygon<f64>,
    corners: [[f64; 2]; 4],
    rect_bounds: [f64; 4],
    world: &impl Fn(f64, f64) -> [f64; 2],
    to_local: &impl Fn(f64, f64) -> (f64, f64),
) -> Option<(f64, f64)> {
    let [u0, v0, w, len] = rect_bounds;
    let has_interior_overlap_near = |lu: f64, lv: f64| -> bool {
        let cu = lu.clamp(u0, u0 + w);
        let cv = lv.clamp(v0, v0 + len);
        let du = if cu <= u0 + w * 0.5 { 1.0 } else { -1.0 };
        let dv = if cv <= v0 + len * 0.5 { 1.0 } else { -1.0 };
        let probes = [
            (cu + du * 1e-9, cv + dv * 1e-9),
            (cu + du * 1e-6, cv + dv * 1e-10),
            (cu + du * 1e-10, cv + dv * 1e-6),
            ((cu - 1e-6).max(u0 + 1e-10), cv + dv * 1e-10),
            ((cu + 1e-6).min(u0 + w - 1e-10), cv + dv * 1e-10),
            (cu + du * 1e-10, (cv - 1e-6).max(v0 + 1e-10)),
            (cu + du * 1e-10, (cv + 1e-6).min(v0 + len - 1e-10)),
        ];
        for (su, sv) in probes {
            if su > u0 && su < u0 + w && sv > v0 && sv < v0 + len {
                let [sx, sy] = world(su, sv);
                if area_mp.contains(&Coord { x: sx, y: sy }) {
                    return true;
                }
            }
        }
        false
    };

    let mut lo_v = 1e300_f64;
    let mut hi_v = -1e300_f64;
    let mut any = false;
    let mut record_lv = |lv: f64| {
        lo_v = lo_v.min(lv);
        hi_v = hi_v.max(lv);
        any = true;
    };

    for part in &area_mp.0 {
        for ring in std::iter::once(part.exterior()).chain(part.interiors().iter()) {
            for &coord in &ring.0 {
                let (lu, lv) = to_local(coord.x, coord.y);
                if lu < u0 - 1e-9 || lu > u0 + w + 1e-9 || lv < v0 - 1e-9 || lv > v0 + len + 1e-9 {
                    continue;
                }
                let strictly_in_rect =
                    lu > u0 + 1e-9 && lu < u0 + w - 1e-9 && lv > v0 + 1e-9 && lv < v0 + len - 1e-9;
                if strictly_in_rect || has_interior_overlap_near(lu, lv) {
                    record_lv(lv);
                }
            }
        }
    }
    for [wx, wy] in corners {
        let (lu, lv) = to_local(wx, wy);
        if area_mp.contains(&Coord { x: wx, y: wy }) || has_interior_overlap_near(lu, lv) {
            record_lv(lv);
        }
    }
    let rect_edges = [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
    ];
    for (rp, rq) in rect_edges {
        for part in &area_mp.0 {
            for ring in std::iter::once(part.exterior()).chain(part.interiors().iter()) {
                for seg in ring.0.windows(2) {
                    if seg[0] == seg[1] {
                        continue;
                    }
                    if let Some(hit) = segment_crossing(rp, rq, seg[0], seg[1]) {
                        let (lu, lv) = to_local(hit.xy[0], hit.xy[1]);
                        if hit.proper || has_interior_overlap_near(lu, lv) {
                            record_lv(lv);
                        }
                    }
                }
            }
        }
    }

    any.then(|| (lo_v.max(v0), hi_v.min(v0 + len)))
}
