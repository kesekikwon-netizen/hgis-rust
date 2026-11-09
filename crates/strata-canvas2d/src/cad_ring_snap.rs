//! 대략 그린 링을 가장 가까운 경계선에 맞추기.
//!
//! cad_snap_trace 가 388줄이라 여기로 나눴다.

use crate::cad_boundary_graph::BoundaryGraph;
use crate::cad_point_grid::sqr_dist;

/// `SnapTraceOps::snapGeometryToNearestLines` — 도형을 가장 가까운 지적·참조 선에 붙이고 사이 꼭짓점 자동 채움
#[must_use]
pub fn snap_ring_to_nearest_lines(
    in_ring: &[[f64; 2]],
    closed: bool,
    reference_rings: &[Vec<[f64; 2]>],
    mupp: f64,
) -> (Vec<[f64; 2]>, usize, usize) {
    let mut n = in_ring.len();
    if closed && n > 1 && in_ring[0] == in_ring[n - 1] {
        n -= 1;
    }
    let min_pts = if closed { 3 } else { 2 };
    if n < min_pts || reference_rings.is_empty() {
        return (in_ring.to_vec(), 0, 0);
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &pt in &in_ring[..n] {
        min_x = min_x.min(pt[0]);
        min_y = min_y.min(pt[1]);
        max_x = max_x.max(pt[0]);
        max_y = max_y.max(pt[1]);
    }
    let diag = (max_x - min_x).hypot(max_y - min_y);
    let eff_mupp = if mupp > 0.0 { mupp } else { 0.08 };
    let snap_tol = (eff_mupp * 48.0).max(diag * 0.24).clamp(2.5, 25.0);
    let v_tol = snap_tol * 0.78;
    let merge_tol = (eff_mupp * 0.3).max(0.01);
    let skip_sq = (merge_tol * 2.5).powi(2);

    let grow = snap_tol * 1.5;
    let box_min = [min_x - grow, min_y - grow];
    let box_max = [max_x + grow, max_y + grow];
    let span = (box_max[0] - box_min[0]).max(box_max[1] - box_min[1]);

    let mut graph = BoundaryGraph::new(merge_tol, span);
    for ref_ring in reference_rings {
        if graph.full() {
            break;
        }
        graph.add_ring(ref_ring, box_min, box_max, true);
    }
    if graph.nodes.is_empty() {
        return (in_ring.to_vec(), 0, 0);
    }
    graph.link_t_junctions();

    let mut moved_count = 0usize;
    let mut inserted_count = 0usize;
    let mut snapped = Vec::with_capacity(n);
    for &pt in &in_ring[..n] {
        let (sp, moved) = graph.snap_point(pt, v_tol, snap_tol);
        if moved {
            moved_count += 1;
        }
        snapped.push(sp);
    }

    let mut built = Vec::new();
    for i in 0..n {
        built.push(snapped[i]);
        if !closed && i + 1 >= n {
            break;
        }
        let next_pt = snapped[(i + 1) % n];
        let mids = graph.trace(snapped[i], next_pt, snap_tol, v_tol, skip_sq, 24);
        if !mids.is_empty() {
            inserted_count += mids.len();
            built.extend(mids);
        }
    }

    let mut clean: Vec<[f64; 2]> = Vec::new();
    for pt in built {
        if clean
            .last()
            .is_some_and(|&last| sqr_dist(last, pt) <= graph.merge_sq)
        {
            continue;
        }
        if clean.len() >= 2 && sqr_dist(clean[clean.len() - 2], pt) <= graph.merge_sq {
            clean.pop();
            continue;
        }
        clean.push(pt);
    }
    if closed && clean.len() >= 3 && sqr_dist(clean[0], clean[clean.len() - 1]) <= graph.merge_sq {
        clean.pop();
    }
    if clean.len() >= min_pts {
        (clean, moved_count, inserted_count)
    } else {
        (in_ring.to_vec(), 0, 0)
    }
}
