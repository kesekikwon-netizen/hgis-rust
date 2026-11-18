use rstar::{primitives::Line, RTree};

#[cfg(test)]
#[path = "snap_index_tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Vertex,
    Segment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapHit {
    pub point: [f64; 2],
    pub kind: SnapKind,
    pub distance: f64,
}

#[derive(Debug, Default, Clone)]
pub struct SnapIndex {
    tree: RTree<[f64; 2]>,
    segments: RTree<Line<[f64; 2]>>,
}

impl SnapIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild(&mut self, vertices: Vec<[f64; 2]>, segments: Vec<[[f64; 2]; 2]>) {
        self.tree = RTree::bulk_load(vertices);
        self.segments = RTree::bulk_load(segments.into_iter().map(|[from, to]| Line::new(from, to)).collect());
    }

    pub fn query(&self, cursor: [f64; 2], tolerance: f64) -> Option<SnapHit> {
        // Nearest vertex/segment, target/cpp-ref/src/core/LayerOps.cpp:2852-2872;
        // preserve the existing alignment nearest_on policy. R-tree search is data-dependent, worst O(n).
        let mut best = self
            .tree
            .nearest_neighbor(&cursor)
            .map(|&point| SnapHit {
                point,
                kind: SnapKind::Vertex,
                distance: (point[0] - cursor[0]).hypot(point[1] - cursor[1]),
            })
            .filter(|hit| hit.distance <= tolerance);
        if let Some(segment) = self.segments.nearest_neighbor(&cursor) {
            let point = segment.nearest_point(&cursor);
            let distance = (point[0] - cursor[0]).hypot(point[1] - cursor[1]);
            if distance <= tolerance && best.is_none_or(|hit| distance < hit.distance) {
                best = Some(SnapHit { point, kind: SnapKind::Segment, distance });
            }
        }
        best
    }
}
