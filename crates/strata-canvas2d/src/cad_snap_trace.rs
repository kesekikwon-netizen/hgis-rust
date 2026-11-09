//! 경계 그래프 스냅·부착·경로 추적과 근접점 탐색.
//!
//! `impl BoundaryGraph` 가 279줄이라 여기로 나눴다.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::cad_boundary_graph::BoundaryGraph;
use crate::cad_point_grid::{dist, project_on_segment, sqr_dist, PointGrid};

#[derive(Copy, Clone, PartialEq)]
struct DijkstraItem {
    cost: f64,
    node: usize,
}

impl Eq for DijkstraItem {}

impl Ord for DijkstraItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for DijkstraItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl BoundaryGraph {

    pub fn snap_point(&self, target: [f64; 2], v_tol: f64, s_tol: f64) -> ([f64; 2], bool) {
        let mut best_n: Option<usize> = None;
        let mut best_nd = v_tol;
        let mut best_s: Option<usize> = None;
        let mut best_sd = s_tol;
        let mut best_proj = target;

        for (i, &node) in self.nodes.iter().enumerate() {
            let d = dist(node, target);
            if d <= best_nd {
                best_nd = d;
                best_n = Some(i);
            }
        }
        for (i, &(u, v)) in self.segs.iter().enumerate() {
            let proj = project_on_segment(target, self.nodes[u], self.nodes[v]);
            let d = dist(proj, target);
            if d < best_sd {
                best_sd = d;
                best_s = Some(i);
                best_proj = proj;
            }
        }
        let res = if let Some(n_idx) = best_n {
            if best_nd <= (best_sd * 2.2).max(v_tol * 0.45) {
                self.nodes[n_idx]
            } else if best_s.is_some() {
                best_proj
            } else {
                self.nodes[n_idx]
            }
        } else if best_s.is_some() {
            best_proj
        } else {
            target
        };
        let moved = sqr_dist(res, target) > self.merge_sq;
        (res, moved)
    }

    pub fn attach(&mut self, target: [f64; 2], snap_tol: f64, v_tol: f64) -> Option<usize> {
        let mut best_n: Option<usize> = None;
        let mut best_nd = snap_tol;
        let mut best_s: Option<usize> = None;
        let mut best_sd = snap_tol;
        let mut best_proj = target;

        for (i, &node) in self.nodes.iter().enumerate() {
            let d = dist(node, target);
            if d <= best_nd {
                best_nd = d;
                best_n = Some(i);
            }
        }
        if let Some(n_idx) = best_n {
            if best_nd <= v_tol {
                return Some(n_idx);
            }
        }
        for (i, &(u, v)) in self.segs.iter().enumerate() {
            let proj = project_on_segment(target, self.nodes[u], self.nodes[v]);
            let d = dist(proj, target);
            if d < best_sd {
                best_sd = d;
                best_s = Some(i);
                best_proj = proj;
            }
        }
        let Some(s_idx) = best_s else {
            return best_n;
        };
        let (u, v) = self.segs[s_idx];
        if dist(best_proj, self.nodes[u]) <= v_tol {
            return Some(u);
        }
        if dist(best_proj, self.nodes[v]) <= v_tol {
            return Some(v);
        }
        let w = self.add_node(best_proj);
        self.add_edge(w, u);
        self.add_edge(w, v);
        Some(w)
    }

    pub fn trace(
        &mut self,
        from_pt: [f64; 2],
        to_pt: [f64; 2],
        snap_tol: f64,
        v_tol: f64,
        skip_sq: f64,
        max_mids: usize,
    ) -> Vec<[f64; 2]> {
        let chord = dist(from_pt, to_pt);
        if self.nodes.len() < 3 || !chord.is_finite() || chord <= self.merge_tol * 2.0 {
            return Vec::new();
        }
        let Some(s) = self.attach(from_pt, snap_tol, v_tol) else {
            return Vec::new();
        };
        let Some(t) = self.attach(to_pt, snap_tol, v_tol) else {
            return Vec::new();
        };
        if s == t {
            return Vec::new();
        }
        let corridor_sq = (snap_tol * 1.5).max(chord * 0.45).powi(2);
        let mut d_vec = vec![f64::INFINITY; self.nodes.len()];
        let mut parent = vec![usize::MAX; self.nodes.len()];
        let mut pq = BinaryHeap::new();
        d_vec[s] = 0.0;
        pq.push(DijkstraItem { cost: 0.0, node: s });

        while let Some(DijkstraItem { cost, node: u }) = pq.pop() {
            if u == t {
                break;
            }
            if cost > d_vec[u] {
                continue;
            }
            for &(v, w) in &self.adj[u] {
                let proj = project_on_segment(self.nodes[v], from_pt, to_pt);
                if sqr_dist(proj, self.nodes[v]) > corridor_sq {
                    continue;
                }
                let next_cost = cost + w;
                if next_cost < d_vec[v] {
                    d_vec[v] = next_cost;
                    parent[v] = u;
                    pq.push(DijkstraItem {
                        cost: next_cost,
                        node: v,
                    });
                }
            }
        }
        if !d_vec[t].is_finite() || d_vec[t] > chord * 2.1 {
            return Vec::new();
        }
        let mut chain = Vec::new();
        let mut cur = parent[t];
        while cur != usize::MAX && cur != s {
            chain.push(cur);
            cur = parent[cur];
        }
        chain.reverse();
        let mut mids = Vec::new();
        for idx in chain {
            let pt = self.nodes[idx];
            if sqr_dist(pt, from_pt) > skip_sq && sqr_dist(pt, to_pt) > skip_sq {
                mids.push(pt);
            }
        }
        if mids.len() <= max_mids {
            mids
        } else {
            Vec::new()
        }
    }
}

/// `SnapTraceOps::nearbyVertices` — 커서 반경(radius_px) 내 근접 자석점 추출 (PointGrid 중복 제거)
#[must_use]
pub fn nearby_vertices(
    cursor_xy: [f64; 2],
    active_snap_pt: Option<[f64; 2]>,
    rings: &[Vec<[f64; 2]>],
    mupp: f64,
    radius_px: f64,
    max_count: usize,
) -> Vec<[f64; 2]> {
    if radius_px <= 0.0 || max_count == 0 || !cursor_xy[0].is_finite() || !cursor_xy[1].is_finite()
    {
        return Vec::new();
    }
    let eff_mupp = if mupp > 0.0 { mupp } else { 0.05 };
    let radius_map = radius_px * eff_mupp;
    let radius_sq = radius_map * radius_map;
    let dedup = (eff_mupp * 2.5).max(0.01);
    let dedup_sq = dedup * dedup;
    let mut seen = PointGrid::new(dedup);
    let mut cands: Vec<(f64, [f64; 2])> = Vec::new();

    for ring in rings {
        for &pt in ring {
            let d_sq = sqr_dist(cursor_xy, pt);
            if d_sq > radius_sq {
                continue;
            }
            if active_snap_pt.is_some_and(|sp| sqr_dist(sp, pt) <= dedup_sq) {
                continue;
            }
            let near = seen.candidates(pt[0] - dedup, pt[1] - dedup, pt[0] + dedup, pt[1] + dedup);
            if near.iter().any(|&i| sqr_dist(cands[i].1, pt) <= dedup_sq) {
                continue;
            }
            seen.insert(pt, cands.len());
            cands.push((d_sq, pt));
        }
    }
    cands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    cands
        .into_iter()
        .take(max_count)
        .map(|(_, pt)| pt)
        .collect()
}

/// `SnapTraceOps::traceBetween` — 두 클릭 점 사이 경계선을 따라 빠진 꼭짓점 자동 채움
#[must_use]
pub fn trace_between(
    from_xy: [f64; 2],
    to_xy: [f64; 2],
    rings: &[Vec<[f64; 2]>],
    mupp: f64,
    max_intermediate: usize,
) -> Vec<[f64; 2]> {
    if max_intermediate == 0 {
        return Vec::new();
    }
    let chord = dist(from_xy, to_xy);
    let eff_mupp = if mupp > 0.0 { mupp } else { 0.05 };
    let snap_tol = (eff_mupp * 16.0).max(0.25);
    let merge_tol = (eff_mupp * 0.4).max(0.005);
    let skip_sq = (eff_mupp * 2.0).max(0.02).powi(2);
    let v_tol = (merge_tol * 4.0).max(eff_mupp * 4.0);
    if !chord.is_finite() || chord <= snap_tol * 0.5 {
        return Vec::new();
    }
    let grow = (chord * 0.6).max(snap_tol * 2.0);
    let min_xy = [
        from_xy[0].min(to_xy[0]) - grow,
        from_xy[1].min(to_xy[1]) - grow,
    ];
    let max_xy = [
        from_xy[0].max(to_xy[0]) + grow,
        from_xy[1].max(to_xy[1]) + grow,
    ];
    let span = (max_xy[0] - min_xy[0]).max(max_xy[1] - min_xy[1]);

    let mut graph = BoundaryGraph::new(merge_tol, span);
    for ring in rings {
        if graph.full() {
            break;
        }
        graph.add_ring(ring, min_xy, max_xy, true);
    }
    if !graph.nodes.is_empty() {
        graph.link_t_junctions();
    }
    graph.trace(from_xy, to_xy, snap_tol, v_tol, skip_sq, max_intermediate)
}
