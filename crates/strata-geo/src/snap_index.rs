use rstar::RTree;

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
    segments: Vec<[[f64; 2]; 2]>,
}

impl SnapIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild(&mut self, vertices: Vec<[f64; 2]>, segments: Vec<[[f64; 2]; 2]>) {
        self.tree = RTree::bulk_load(vertices);
        self.segments = segments;
    }

    pub fn query(&self, cursor: [f64; 2], tolerance: f64) -> Option<SnapHit> {
        if let Some(&v) = self.tree.nearest_neighbor(&cursor) {
            let d = ((v[0] - cursor[0]).powi(2) + (v[1] - cursor[1]).powi(2)).sqrt();
            if d <= tolerance {
                return Some(SnapHit {
                    point: v,
                    kind: SnapKind::Vertex,
                    distance: d,
                });
            }
        }
        let mut best_seg: Option<SnapHit> = None;
        for seg in &self.segments {
            let proj = project_point_to_segment(cursor, seg[0], seg[1]);
            let d = ((proj[0] - cursor[0]).powi(2) + (proj[1] - cursor[1]).powi(2)).sqrt();
            if d <= tolerance && best_seg.is_none_or(|b| d < b.distance) {
                best_seg = Some(SnapHit {
                    point: proj,
                    kind: SnapKind::Segment,
                    distance: d,
                });
            }
        }
        best_seg
    }
}

fn project_point_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-12 {
        return a;
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
    [a[0] + t * dx, a[1] + t * dy]
}
