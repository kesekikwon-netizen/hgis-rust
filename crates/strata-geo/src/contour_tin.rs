use spade::{DelaunayTriangulation, HasPosition, Point2, Triangulation};

#[derive(Debug, Clone, Copy)]
struct ElevVertex {
    pos: Point2<f64>,
    z: f64,
}

impl HasPosition for ElevVertex {
    type Scalar = f64;
    fn position(&self) -> Point2<f64> {
        self.pos
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContourLine {
    pub elevation_m: f64,
    pub is_index_contour: bool,
    pub segments: Vec<[[f64; 2]; 2]>,
}

pub fn build_contours_from_points(pts: &[[f64; 3]], interval_m: f64) -> Vec<ContourLine> {
    if pts.len() < 3 || interval_m <= 0.0 {
        return Vec::new();
    }
    let mut tri: DelaunayTriangulation<ElevVertex> = DelaunayTriangulation::new();
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    for &[x, y, z] in pts {
        z_min = z_min.min(z);
        z_max = z_max.max(z);
        let _ = tri.insert(ElevVertex {
            pos: Point2::new(x, y),
            z,
        });
    }
    let start_k = (z_min / interval_m).ceil() as i64;
    let end_k = (z_max / interval_m).floor() as i64;
    let mut out = Vec::new();

    for k in start_k..=end_k {
        let level = (k as f64) * interval_m;
        let mut segments = Vec::new();
        for face in tri.inner_faces() {
            let [v0, v1, v2] = face.vertices().map(|v| *v.data());
            if let Some(seg) = intersect_triangle_level(v0, v1, v2, level) {
                segments.push(seg);
            }
        }
        if !segments.is_empty() {
            out.push(ContourLine {
                elevation_m: level,
                is_index_contour: k % 5 == 0,
                segments,
            });
        }
    }
    out
}

fn intersect_triangle_level(
    v0: ElevVertex,
    v1: ElevVertex,
    v2: ElevVertex,
    level: f64,
) -> Option<[[f64; 2]; 2]> {
    let edges = [(v0, v1), (v1, v2), (v2, v0)];
    let mut hits = Vec::with_capacity(2);
    for (a, b) in edges {
        if (a.z <= level && b.z > level) || (b.z <= level && a.z > level) {
            let t = (level - a.z) / (b.z - a.z);
            let x = a.pos.x + t * (b.pos.x - a.pos.x);
            let y = a.pos.y + t * (b.pos.y - a.pos.y);
            hits.push([x, y]);
        }
    }
    if hits.len() >= 2 {
        Some([hits[0], hits[1]])
    } else {
        None
    }
}
