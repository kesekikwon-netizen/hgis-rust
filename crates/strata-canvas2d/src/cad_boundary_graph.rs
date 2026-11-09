//! 경계 그래프(`BoundaryGraph`) — 노드·간선·링 연결.
//!
//! `impl BoundaryGraph` 가 279줄이라 두 갈래로 나눴다.
//! 스냅·추적은 `cad_snap_trace` 에 있다.

use crate::cad_point_grid::{dist, project_on_segment, sqr_dist, PointGrid};

/// `SnapTraceGraph.h::BoundaryGraph` — 최대 900점 제한 경계 그래프 및 최단 경로 추적
#[derive(Debug, Clone)]
pub struct BoundaryGraph {
    pub merge_tol: f64,
    pub merge_sq: f64,
    pub nodes: Vec<[f64; 2]>,
    pub adj: Vec<Vec<(usize, f64)>>,
    pub segs: Vec<(usize, usize)>,
    pub grid: PointGrid,
}

impl BoundaryGraph {

    pub const MAX_NODES: usize = 900;

    #[must_use]
    pub fn new(merge_tol: f64, box_span: f64) -> Self {
        let tol = merge_tol.max(1e-4);
        let cell = (tol * 2.0).max(if box_span.is_finite() && box_span > 0.0 {
            box_span / 128.0
        } else {
            tol * 2.0
        });
        Self {
            merge_tol: tol,
            merge_sq: tol * tol,
            nodes: Vec::new(),
            adj: Vec::new(),
            segs: Vec::new(),
            grid: PointGrid::new(cell),
        }
    }

    #[must_use]
    pub fn full(&self) -> bool {
        self.nodes.len() >= Self::MAX_NODES
    }

    #[must_use]
    pub fn find_node(&self, p: [f64; 2]) -> Option<usize> {
        let cands = self.grid.candidates(
            p[0] - self.merge_tol,
            p[1] - self.merge_tol,
            p[0] + self.merge_tol,
            p[1] + self.merge_tol,
        );
        cands
            .into_iter()
            .find(|&i| sqr_dist(self.nodes[i], p) <= self.merge_sq)
    }

    pub fn add_node(&mut self, p: [f64; 2]) -> usize {
        if let Some(i) = self.find_node(p) {
            return i;
        }
        let idx = self.nodes.len();
        self.nodes.push(p);
        self.adj.push(Vec::new());
        self.grid.insert(p, idx);
        idx
    }

    pub fn add_edge(&mut self, u: usize, v: usize) {
        if u == v {
            return;
        }
        let w = dist(self.nodes[u], self.nodes[v]);
        self.adj[u].push((v, w));
        self.adj[v].push((u, w));
        self.segs.push((u, v));
    }

    pub fn add_ring(
        &mut self,
        ring: &[[f64; 2]],
        min_xy: [f64; 2],
        max_xy: [f64; 2],
        closed: bool,
    ) {
        if ring.len() < 2 || self.full() {
            return;
        }
        let in_box = |pt: [f64; 2]| -> bool {
            pt[0] >= min_xy[0] && pt[0] <= max_xy[0] && pt[1] >= min_xy[1] && pt[1] <= max_xy[1]
        };
        for w in ring.windows(2) {
            if self.full() {
                break;
            }
            if in_box(w[0]) || in_box(w[1]) {
                let u = self.add_node(w[0]);
                let v = self.add_node(w[1]);
                self.add_edge(u, v);
            }
        }
        if closed && ring.len() >= 3 && !self.full() {
            let a = ring[ring.len() - 1];
            let b = ring[0];
            if sqr_dist(a, b) > self.merge_sq && (in_box(a) || in_box(b)) {
                let u = self.add_node(a);
                let v = self.add_node(b);
                self.add_edge(u, v);
            }
        }
    }

    pub fn link_t_junctions(&mut self) {
        let reach = self.merge_tol * 2.0;
        let reach_sq = self.merge_sq * 4.0;
        let orig_segs = self.segs.len();
        for s in 0..orig_segs {
            let (u, v) = self.segs[s];
            let nu = self.nodes[u];
            let nv = self.nodes[v];
            let min_x = nu[0].min(nv[0]) - reach;
            let max_x = nu[0].max(nv[0]) + reach;
            let min_y = nu[1].min(nv[1]) - reach;
            let max_y = nu[1].max(nv[1]) + reach;
            for w in self.grid.candidates(min_x, min_y, max_x, max_y) {
                if w == u || w == v {
                    continue;
                }
                let nw = self.nodes[w];
                if sqr_dist(project_on_segment(nw, nu, nv), nw) <= reach_sq {
                    self.add_edge(u, w);
                    self.add_edge(w, v);
                }
            }
        }
    }
}
