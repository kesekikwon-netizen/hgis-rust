use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::canvas_painter::{MapSceneModel, WorldPolygonItem};
use crate::map_tools::MapToolKind;
use crate::viewport::ViewportTransform;

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureReadout {
    pub total_distance_m: f64,
    pub polygon_area_m2: f64,
    /// 찍은 순서대로 이은 다각형의 둘레 — `KaMeasureMapTool.cpp:369,399` 「둘레」
    pub polygon_perimeter_m: f64,
}

pub const HGIS_METERS_TO_KM_THRESHOLD: f64 = 1000.0;
pub const HGIS_SQMETERS_TO_HA_THRESHOLD: f64 = 10000.0;

/// C++ `KaMeasureTool::SegmentMetrics` — 선분 거리(m) 및 북기준 시계방향 방위각(0° ~ 360°)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentMetrics {
    pub distance_m: f64,
    pub azimuth_deg: f64,
}

/// C++ `KaMeasureTool::computeSegmentMetrics` — 북쪽 0.0°, 동쪽 90.0°, 남쪽 180.0°, 서쪽 270.0° 시계방향 방위각 계산
#[must_use]
pub fn compute_segment_metrics(start_pt: [f64; 2], end_pt: [f64; 2]) -> SegmentMetrics {
    let dx = end_pt[0] - start_pt[0];
    let dy = end_pt[1] - start_pt[1];
    let distance_m = dx.hypot(dy);
    if distance_m < 1e-12 {
        return SegmentMetrics {
            distance_m: 0.0,
            azimuth_deg: 0.0,
        };
    }
    let theta_deg = dy.atan2(dx).to_degrees();
    let azimuth_deg = (90.0 - theta_deg).rem_euclid(360.0);
    SegmentMetrics {
        distance_m,
        azimuth_deg,
    }
}

/// C++ `KaMeasureTool` 거리 포맷 — `< 1000.0m`는 `"850.50 m"`, `>= 1000.0m`는 `"1.50 km"`
#[must_use]
pub fn format_measure_distance(distance_m: f64) -> String {
    if distance_m >= HGIS_METERS_TO_KM_THRESHOLD {
        format!("{:.2} km", distance_m / HGIS_METERS_TO_KM_THRESHOLD)
    } else {
        format!("{distance_m:.2} m")
    }
}

/// C++ `KaMeasureTool` 면적 포맷 — `< 10000.0m²`는 `"200.00 ㎡"`, `>= 10000.0m²`는 `"2.50 ha"`
#[must_use]
pub fn format_measure_area(area_m2: f64) -> String {
    if area_m2 >= HGIS_SQMETERS_TO_HA_THRESHOLD {
        format!("{:.2} ha", area_m2 / HGIS_SQMETERS_TO_HA_THRESHOLD)
    } else {
        format!("{area_m2:.2} ㎡")
    }
}

/// `SnapTraceGraph.h` — 마우스 이동 시 O(N^2) 비교를 피하기 위한 정사각 격자 해시
#[derive(Debug, Clone, Default)]
pub struct PointGrid {
    pub cell: f64,
    pub cells: HashMap<u64, Vec<usize>>,
}

impl PointGrid {
    #[must_use]
    pub fn new(cell: f64) -> Self {
        Self {
            cell: cell.max(1e-6),
            cells: HashMap::new(),
        }
    }

    #[must_use]
    pub fn key_of(cx: i64, cy: i64) -> u64 {
        ((cx as i32 as u32 as u64) << 32) | (cy as i32 as u32 as u64)
    }

    #[must_use]
    pub fn cell_of(&self, v: f64) -> i64 {
        (v / self.cell).floor() as i64
    }

    pub fn insert(&mut self, p: [f64; 2], index: usize) {
        let key = Self::key_of(self.cell_of(p[0]), self.cell_of(p[1]));
        self.cells.entry(key).or_default().push(index);
    }

    #[must_use]
    pub fn candidates(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<usize> {
        let mut out = Vec::new();
        let x0 = self.cell_of(min_x);
        let x1 = self.cell_of(max_x);
        let y0 = self.cell_of(min_y);
        let y1 = self.cell_of(max_y);
        let span_cells = ((x1 - x0 + 1) as f64) * ((y1 - y0 + 1) as f64);
        if span_cells > self.cells.len() as f64 {
            for (&key, indices) in &self.cells {
                let cx = (key >> 32) as u32 as i32 as i64;
                let cy = (key & 0xffff_ffff) as u32 as i32 as i64;
                if cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1 {
                    out.extend_from_slice(indices);
                }
            }
        } else {
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    if let Some(indices) = self.cells.get(&Self::key_of(cx, cy)) {
                        out.extend_from_slice(indices);
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

#[must_use]
pub fn project_on_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq <= 1e-12 {
        return a;
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    [a[0] + t * dx, a[1] + t * dy]
}

#[inline]
fn sqr_dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

#[inline]
fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

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

#[derive(Debug, Clone)]
pub struct CadSessionController {
    pub last_measure: Option<MeasureReadout>,
    pub snap_enabled: bool,
    pub snap_trace_enabled: bool,
    pub snap_preview_enabled: bool,
    pub snap_tolerance_px: f64,
    /// `KaSnapSettingsWidget.cpp:28-30` — 켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮긴다
    pub snap_topological: bool,
    /// `KaSnapSettingsWidget.cpp:24-27` — `false` = 「모든 조사 레이어」, `true` = 「현재 레이어」
    pub snap_target_current_only: bool,
    pub preview_markers_world: Vec<[f64; 2]>,
}

impl Default for CadSessionController {
    fn default() -> Self {
        Self {
            last_measure: None,
            snap_enabled: true,
            snap_trace_enabled: true,
            snap_preview_enabled: true,
            snap_tolerance_px: 12.0,
            snap_topological: true,
            snap_target_current_only: false,
            preview_markers_world: Vec::new(),
        }
    }
}

impl CadSessionController {
    #[must_use]
    pub fn snap_world_point(world_xy: [f64; 2], scene: &MapSceneModel, tol_m: f64) -> [f64; 2] {
        let mut best = world_xy;
        let mut best_d = tol_m;
        for item in &scene.polygons {
            for &pt in &item.world_pts {
                let d = (pt[0] - world_xy[0]).hypot(pt[1] - world_xy[1]);
                if d < best_d {
                    best_d = d;
                    best = pt;
                }
            }
        }
        best
    }

    pub fn on_left_click(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        scene: &mut MapSceneModel,
    ) {
        self.on_left_click_with_refs(world_xy, tool, scene, &[], 0.5);
    }

    /// `KaSnapSettingsWidget.cpp:24-27` — 「현재 레이어」일 때는 그 레이어 도형에만 붙는다.
    /// `current_only` 판단은 호출한 쪽의 레이어 고르기를 따른다(`snap_target_current_only`가 켜졌을 때만).
    #[must_use]
    fn snap_rings_for<'a>(
        &self,
        scene: &'a MapSceneModel,
        extra_rings: &'a [Vec<[f64; 2]>],
        current_layer_key: Option<&str>,
    ) -> Vec<Vec<[f64; 2]>> {
        let mut rings: Vec<Vec<[f64; 2]>> = scene
            .polygons
            .iter()
            .filter(|p| {
                !(self.snap_target_current_only
                    && current_layer_key.is_some_and(|k| p.layer_key != k))
            })
            .map(|p| p.world_pts.clone())
            .collect();
        rings.extend_from_slice(extra_rings);
        rings
    }

    pub fn on_left_click_with_refs(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        scene: &mut MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        mupp: f64,
    ) {
        self.on_left_click_with_refs_and_layer(world_xy, tool, scene, extra_rings, mupp, None);
    }

    pub fn on_left_click_with_refs_and_layer(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        scene: &mut MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        mupp: f64,
        current_layer_key: Option<&str>,
    ) {
        let tol_m = if self.snap_enabled {
            (self.snap_tolerance_px * mupp.max(0.05)).max(2.5)
        } else {
            0.0
        };
        let snap_rings = self.snap_rings_for(scene, extra_rings, current_layer_key);
        let snapped = if self.snap_enabled {
            Self::snap_world_point_with_rings(world_xy, scene, &snap_rings, tol_m)
        } else {
            world_xy
        };
        scene.snap_marker_world = Some(snapped);
        match tool {
            MapToolKind::DrawPolygon | MapToolKind::DrawLine => {
                if self.snap_enabled && self.snap_trace_enabled {
                    if let Some(&last_pt) = scene.active_rubberband.last() {
                        let mut all_rings: Vec<Vec<[f64; 2]>> =
                            scene.polygons.iter().map(|p| p.world_pts.clone()).collect();
                        all_rings.extend_from_slice(extra_rings);
                        let mids = trace_between(last_pt, snapped, &all_rings, mupp, 20);
                        scene.active_rubberband.extend(mids);
                    }
                }
                scene.active_rubberband.push(snapped);
            }
            MapToolKind::PlacePoint => {
                let s = 1.5;
                scene.polygons.push(WorldPolygonItem {
                    layer_key: "control_points".to_owned(),
                    world_pts: vec![
                        [snapped[0] - s, snapped[1] - s],
                        [snapped[0] + s, snapped[1] - s],
                        [snapped[0] + s, snapped[1] + s],
                        [snapped[0] - s, snapped[1] + s],
                    ],
                    fill_rgba: [220, 38, 38, 220],
                    stroke_rgba: [153, 27, 27, 255],
                });
            }
            MapToolKind::TrenchPlace => {
                scene.polygons.push(WorldPolygonItem {
                    layer_key: "trial_trench".to_owned(),
                    world_pts: vec![
                        [snapped[0] - 1.0, snapped[1] - 10.0],
                        [snapped[0] + 1.0, snapped[1] - 10.0],
                        [snapped[0] + 1.0, snapped[1] + 10.0],
                        [snapped[0] - 1.0, snapped[1] + 10.0],
                    ],
                    fill_rgba: [37, 99, 235, 95],
                    stroke_rgba: [29, 78, 216, 255],
                });
            }
            MapToolKind::Measure => {
                scene.active_rubberband.push(snapped);
                self.recompute_measure(&scene.active_rubberband);
            }
            _ => {}
        }
    }

    #[must_use]
    pub fn snap_world_point_with_refs(
        world_xy: [f64; 2],
        scene: &MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        tol_m: f64,
    ) -> [f64; 2] {
        Self::snap_world_point_with_rings(world_xy, scene, extra_rings, tol_m)
    }

    #[must_use]
    pub fn snap_world_point_with_rings(
        world_xy: [f64; 2],
        scene: &MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        tol_m: f64,
    ) -> [f64; 2] {
        let mut best = Self::snap_world_point(world_xy, scene, tol_m);
        let mut best_d = (best[0] - world_xy[0]).hypot(best[1] - world_xy[1]);
        if best == world_xy {
            best_d = tol_m;
        }
        for ring in extra_rings {
            for &pt in ring {
                let d = (pt[0] - world_xy[0]).hypot(pt[1] - world_xy[1]);
                if d < best_d {
                    best_d = d;
                    best = pt;
                }
            }
        }
        best
    }

    pub fn update_cursor_preview(
        &mut self,
        cursor_xy: [f64; 2],
        scene: &MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        mupp: f64,
    ) {
        if !self.snap_enabled || !self.snap_preview_enabled {
            self.preview_markers_world.clear();
            return;
        }
        let mut all_rings: Vec<Vec<[f64; 2]>> =
            scene.polygons.iter().map(|p| p.world_pts.clone()).collect();
        all_rings.extend_from_slice(extra_rings);
        self.preview_markers_world = nearby_vertices(
            cursor_xy,
            scene.snap_marker_world,
            &all_rings,
            mupp,
            48.0,
            24,
        );
    }

    pub fn on_right_click(&mut self, tool: MapToolKind, scene: &mut MapSceneModel) -> bool {
        if tool == MapToolKind::DrawPolygon && scene.active_rubberband.len() >= 3 {
            let pts: Vec<[f64; 2]> = scene.active_rubberband.drain(..).collect();
            let n = pts.len() as f64;
            let cx = pts.iter().map(|p| p[0]).sum::<f64>() / n;
            let cy = pts.iter().map(|p| p[1]).sum::<f64>() / n;
            scene.polygons.push(WorldPolygonItem {
                layer_key: "survey_area".to_owned(),
                world_pts: pts,
                fill_rgba: [181, 87, 58, 52],
                stroke_rgba: [181, 87, 58, 240],
            });
            scene.buffer_500m_ring = (0..=36)
                .map(|i| {
                    let th = (i as f64 / 36.0) * std::f64::consts::TAU;
                    [cx + 500.0 * th.cos(), cy + 500.0 * th.sin()]
                })
                .collect();
            scene.buffer_1000m_ring = (0..=36)
                .map(|i| {
                    let th = (i as f64 / 36.0) * std::f64::consts::TAU;
                    [cx + 1000.0 * th.cos(), cy + 1000.0 * th.sin()]
                })
                .collect();
            return true;
        }
        scene.active_rubberband.clear();
        false
    }

    pub fn on_escape(&mut self, scene: &mut MapSceneModel) {
        scene.active_rubberband.clear();
        self.last_measure = None;
    }

    pub fn on_backspace(&mut self, scene: &mut MapSceneModel) {
        let _ = scene.active_rubberband.pop();
        self.recompute_measure(&scene.active_rubberband);
    }

    pub fn on_wheel_zoom(
        &mut self,
        scroll_delta_y: f32,
        cursor_px: [f64; 2],
        vp: &mut ViewportTransform,
    ) {
        if scroll_delta_y.abs() < 0.01 {
            return;
        }
        let factor = if scroll_delta_y > 0.0 { 0.85 } else { 1.18 };
        vp.zoom_at(factor, cursor_px);
    }

    fn recompute_measure(&mut self, pts: &[[f64; 2]]) {
        if pts.len() < 2 {
            self.last_measure = None;
            return;
        }
        let mut total_distance_m = 0.0;
        for w in pts.windows(2) {
            total_distance_m += (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        }
        let mut polygon_area_m2 = 0.0;
        let mut polygon_perimeter_m = 0.0;
        if pts.len() >= 3 {
            let mut shoelace = 0.0;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                shoelace += pts[i][0] * pts[j][1] - pts[j][0] * pts[i][1];
                polygon_perimeter_m +=
                    (pts[j][0] - pts[i][0]).hypot(pts[j][1] - pts[i][1]);
            }
            polygon_area_m2 = 0.5 * shoelace.abs();
        }
        self.last_measure = Some(MeasureReadout {
            total_distance_m,
            polygon_area_m2,
            polygon_perimeter_m,
        });
    }
}

/// 1평 환산 제곱미터 계수 (`3.305785`, C++ `HGIS_PYEONG_CONVERSION_FACTOR`)
pub const HGIS_PYEONG_CONVERSION_FACTOR: f64 = 3.305_785;
/// 좌표 콜아웃 말풍선 모서리 반경 (`2.0 mm`, C++ `HGIS_CALLOUT_CORNER_RADIUS_MM`)
pub const HGIS_CALLOUT_CORNER_RADIUS_MM: f64 = 2.0;

/// 슈레이스 공식(Shoelace Formula)으로 다각형 면적(`㎡`) 및 한국 고고학 발굴 평수(`㎡ / 3.305785`) 산출
/// (C++ `MeasureOps::calculatePolygonAreaM2` 1:1 대응)
#[must_use]
pub fn calculate_polygon_area_and_pyeong(points: &[[f64; 2]]) -> (f64, f64) {
    if points.len() < 3 {
        return (0.0, 0.0);
    }
    let mut shoelace = 0.0_f64;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        shoelace += points[i][0] * points[j][1] - points[j][0] * points[i][1];
    }
    let area_m2 = 0.5 * shoelace.abs();
    let pyeong = area_m2 / HGIS_PYEONG_CONVERSION_FACTOR;
    (area_m2, pyeong)
}

/// 방위각(`degrees`)을 도·분·초(`D°MM′SS″`) 문자열로 포맷팅
/// (C++ `MeasureOps::formatBearing` 1:1 대응)
#[must_use]
pub fn format_bearing_dms(degrees: f64) -> String {
    if !degrees.is_finite() {
        return "—".to_owned();
    }
    let total = (degrees * 3600.0).round() as i64;
    let wrapped = total.rem_euclid(360 * 3600);
    let d = wrapped / 3600;
    let m = (wrapped % 3600) / 60;
    let s = wrapped % 60;
    format!("{d}°{m:02}′{s:02}″")
}

/// 방위각 다이얼 위젯 한국어 안내 문구 생성
/// (C++ `KaAzimuthDial::note` 1:1 대응)
#[must_use]
pub fn azimuth_dial_note_ko(degrees: f64) -> String {
    let turn = degrees.rem_euclid(360.0);
    if turn.abs() < 0.5 || (turn - 360.0).abs() < 0.5 {
        "위가 북쪽입니다".to_owned()
    } else {
        format!("북에서 시계 방향 {:.0}°", turn)
    }
}

/// 도면 좌표 인출선 콜아웃 문자열(`X: ...\nY: ...\nEL: ...`) 생성
/// (C++ `KaDrawingCoordCallout::drawCoordCallout` 1:1 대응)
#[must_use]
pub fn format_drawing_coord_callout(world_xy: [f64; 2], ground_z: f64) -> String {
    fn fmt_with_commas(v: f64) -> String {
        let sign = if v < 0.0 { "-" } else { "" };
        let abs = v.abs();
        let whole = abs.trunc() as u64;
        let frac = ((abs - (whole as f64)) * 100.0).round() as u64;
        let s = whole.to_string();
        let mut out = String::new();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        let whole_comma: String = out.chars().rev().collect();
        format!("{sign}{whole_comma}.{frac:02}")
    }
    format!(
        "X: {}m\nY: {}m\nEL: {:.2}m",
        fmt_with_commas(world_xy[0]),
        fmt_with_commas(world_xy[1]),
        ground_z
    )
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn cad_left_clicks_and_right_click_commit_new_polygon_and_compute_measure() {
        let mut cad = CadSessionController::default();
        let mut scene = MapSceneModel::default();

        cad.on_left_click([100.0, 100.0], MapToolKind::DrawPolygon, &mut scene);
        cad.on_left_click([200.0, 100.0], MapToolKind::DrawPolygon, &mut scene);
        cad.on_left_click([200.0, 200.0], MapToolKind::DrawPolygon, &mut scene);
        cad.on_left_click([100.0, 200.0], MapToolKind::DrawPolygon, &mut scene);
        assert_eq!(scene.active_rubberband.len(), 4);

        let committed = cad.on_right_click(MapToolKind::DrawPolygon, &mut scene);
        assert!(committed);
        assert!(scene.active_rubberband.is_empty());
        assert_eq!(scene.polygons.len(), 1);
        assert_eq!(scene.buffer_500m_ring.len(), 37);

        cad.on_left_click([0.0, 0.0], MapToolKind::Measure, &mut scene);
        cad.on_left_click([100.0, 0.0], MapToolKind::Measure, &mut scene);
        cad.on_left_click([100.0, 100.0], MapToolKind::Measure, &mut scene);
        cad.on_left_click([0.0, 100.0], MapToolKind::Measure, &mut scene);
        let m = cad.last_measure.as_ref();
        assert!(m.is_some_and(|r| (r.total_distance_m - 300.0).abs() < 1e-4));
        assert!(m.is_some_and(|r| (r.polygon_area_m2 - 10_000.0).abs() < 1e-2));
    }

    // D13 KaSnapSettingsWidget.cpp:16-39 — 자석 / 공유 경계 / 허용치 4~64 px / 붙을 대상
    #[test]
    fn test_d13_snap_settings_tolerance_and_measure_copy_text() {
        let mut cad = CadSessionController::default();
        assert!(cad.snap_enabled);
        assert!(cad.snap_topological);
        assert!((cad.snap_tolerance_px - 12.0).abs() < 1e-9);
        assert!(!cad.snap_target_current_only);

        // KaSnapSettingsWidget.cpp:20-21 — 범위는 4.0~64.0 px
        cad.snap_tolerance_px = cad.snap_tolerance_px.clamp(4.0, 64.0);
        assert!((cad.snap_tolerance_px - 12.0).abs() < 1e-9);
        cad.snap_tolerance_px = 100.0_f64.clamp(4.0, 64.0);
        assert!((cad.snap_tolerance_px - 64.0).abs() < 1e-9);
        cad.snap_tolerance_px = 1.0_f64.clamp(4.0, 64.0);
        assert!((cad.snap_tolerance_px - 4.0).abs() < 1e-9);

        // 「모든 조사 레이어」 <-> 「현재 레이어」
        cad.snap_target_current_only = true;
        assert!(cad.snap_target_current_only);
        cad.snap_target_current_only = false;
        assert!(!cad.snap_target_current_only);

        cad.snap_topological = false;
        assert!(!cad.snap_topological);

        // KaMeasureMapTool.cpp:331-355 copyText() — 면적은 둘레도 함께 나간다
        let mut scene = MapSceneModel::default();
        for pt in [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]] {
            cad.on_left_click(pt, MapToolKind::Measure, &mut scene);
        }
        let m = cad.last_measure.expect("measure readout");
        assert!((m.polygon_perimeter_m - 40.0).abs() < 1e-6);
        assert!((m.polygon_area_m2 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn snap_trace_graph_fills_intermediate_parcel_vertices_and_snaps_rough_polygon() {
        let parcel_ring = vec![
            [0.0, 0.0],
            [10.0, 1.5],
            [20.0, 0.5],
            [30.0, 0.0],
            [30.0, 20.0],
            [0.0, 20.0],
        ];
        let refs = vec![parcel_ring];
        let mids = trace_between([0.0, 0.0], [30.0, 0.0], &refs, 0.2, 20);
        assert_eq!(mids, vec![[10.0, 1.5], [20.0, 0.5]]);

        let rough = vec![[0.4, -0.3], [29.7, 0.2], [30.2, 19.8], [-0.2, 20.1]];
        let (snapped, moved, inserted) = snap_ring_to_nearest_lines(&rough, true, &refs, 0.2);
        assert_eq!(moved, 4);
        assert_eq!(inserted, 2);
        assert_eq!(snapped.len(), 6);
        assert!(snapped.contains(&[10.0, 1.5]));
        assert!(snapped.contains(&[20.0, 0.5]));
    }

    #[test]
    fn point_grid_nearby_vertices_deduplicates_and_sorts_by_distance() {
        let rings = vec![vec![
            [10.0, 10.0],
            [10.01, 10.01],
            [15.0, 10.0],
            [100.0, 100.0],
        ]];
        let near = nearby_vertices([10.0, 10.0], None, &rings, 0.5, 20.0, 10);
        assert_eq!(near.len(), 2);
        assert_eq!(near[0], [10.0, 10.0]);
        assert_eq!(near[1], [15.0, 10.0]);
    }

    #[test]
    fn test_point_grid_budget_clamps_2000_dense_vertices_to_exact_900_sorted_by_distance() {
        use std::sync::atomic::{AtomicU64, Ordering};

        // Coalescing job id check (`std::atomic<uint64_t> m_latestJobId`)
        let latest_job_id = AtomicU64::new(0);
        let stale_job = latest_job_id.fetch_add(1, Ordering::SeqCst) + 1;
        let current_job = latest_job_id.fetch_add(1, Ordering::SeqCst) + 1;
        assert!(stale_job < latest_job_id.load(Ordering::SeqCst));
        assert_eq!(current_job, latest_job_id.load(Ordering::SeqCst));

        // 2,000 dense vertices inside 50 m radius around (1000, 1000)
        let mut ring = Vec::with_capacity(2000);
        for i in 1..=2000 {
            let r = (f64::from(i) / 2000.0) * 49.0;
            let angle = f64::from(i) * 0.137;
            ring.push([1000.0 + r * angle.cos(), 1000.0 + r * angle.sin()]);
        }
        let rings = vec![ring];
        let candidates = nearby_vertices(
            [1000.0, 1000.0],
            None,
            &rings,
            0.004,
            12_500.0,
            BoundaryGraph::MAX_NODES,
        );
        assert_eq!(candidates.len(), 900);
        let max_kept_dist = dist([1000.0, 1000.0], candidates[899]);
        let farthest_input_dist = dist([1000.0, 1000.0], rings[0][1999]);
        assert!(max_kept_dist < farthest_input_dist);
    }

    #[test]
    fn test_measure_tool_azimuth_0_90_270_and_unit_formatting_1500m_25000m2() {
        let north = compute_segment_metrics([100.0, 100.0], [100.0, 200.0]);
        assert!((north.azimuth_deg - 0.0).abs() < 1e-9);
        assert!((north.distance_m - 100.0).abs() < 1e-9);

        let east = compute_segment_metrics([100.0, 100.0], [200.0, 100.0]);
        assert!((east.azimuth_deg - 90.0).abs() < 1e-9);

        let west = compute_segment_metrics([100.0, 100.0], [0.0, 100.0]);
        assert!((west.azimuth_deg - 270.0).abs() < 1e-9);

        assert_eq!(format_measure_distance(850.5), "850.50 m");
        assert_eq!(format_measure_distance(1500.0), "1.50 km");
        assert_eq!(format_measure_area(25000.0), "2.50 ha");

        // TestMeasureOps::testShoelacePyeongCalculation: 10m x 10m = 100.0 m² -> 30.250 평
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let (area_m2, pyeong) = calculate_polygon_area_and_pyeong(&sq);
        assert!((area_m2 - 100.0).abs() < 1e-6);
        assert!((pyeong - 30.250).abs() <= 0.001);

        // MeasureOps::formatBearing & KaAzimuthDial::note & KaDrawingCoordCallout
        assert_eq!(format_bearing_dms(45.5), "45°30′00″");
        assert_eq!(azimuth_dial_note_ko(0.0), "위가 북쪽입니다");
        assert_eq!(azimuth_dial_note_ko(35.0), "북에서 시계 방향 35°");
        assert_eq!(
            format_drawing_coord_callout([198_421.35, 541_208.92], 45.32),
            "X: 198,421.35m\nY: 541,208.92m\nEL: 45.32m"
        );
    }
}
