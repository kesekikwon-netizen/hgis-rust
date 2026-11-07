use geo::{Area, BoundingRect, Contains, Coord, LineString, Polygon, Rotate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrenchStandardLength {
    M20,
    M15,
    M10,
}

impl TrenchStandardLength {
    pub const fn meters(self) -> f64 {
        match self {
            Self::M20 => 20.0,
            Self::M15 => 15.0,
            Self::M10 => 10.0,
        }
    }

    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::M20 => "20 m",
            Self::M15 => "15 m",
            Self::M10 => "10 m",
        }
    }
}

/// `KaTrenchDialogPlan.cpp:321-324` — 곧은 트렌치 및 ㄱ·ㄴ자 4방향 트렌치(팔 15m + 15m, 폭 2m)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrenchShapeKind {
    #[default]
    Straight,
    LShapeSw,
    LShapeNe,
    LShapeSe,
    LShapeNw,
}

impl TrenchShapeKind {
    pub const ALL: [Self; 5] = [
        Self::Straight,
        Self::LShapeSw,
        Self::LShapeNe,
        Self::LShapeSe,
        Self::LShapeNw,
    ];

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Straight => "곧은 트렌치",
            Self::LShapeSw => "ㄴ자 트렌치 (15 m + 15 m)",
            Self::LShapeNe => "ㄱ자 트렌치 (15 m + 15 m)",
            Self::LShapeSe => "ㄴ자 좌우 반대 트렌치 (15 m + 15 m)",
            Self::LShapeNw => "ㄱ자 좌우 반대 트렌치 (15 m + 15 m)",
        }
    }

    #[must_use]
    pub const fn arm_bend(self) -> Option<u8> {
        match self {
            Self::Straight => None,
            Self::LShapeSw => Some(0),
            Self::LShapeNe => Some(1),
            Self::LShapeSe => Some(2),
            Self::LShapeNw => Some(3),
        }
    }
}

/// `TrenchLShape.cpp::squareL` — 15 m × 15 m 칸 안에서 바깥 모서리 방향(0..=3)에 따른 6각 ㄱ·ㄴ자 폴리곤 생성
#[must_use]
pub fn build_lshape_ring(u0: f64, v0: f64, arm: f64, width: f64, bend: u8, grid_step: Option<f64>) -> Vec<[f64; 2]> {
    let (u_base, v_base) = if let Some(step) = grid_step.filter(|&s| s > 0.0) {
        ((u0 / step).round() * step, (v0 / step).round() * step)
    } else {
        (u0, v0)
    };
    let east = bend == 1 || bend == 2;
    let north = bend == 1 || bend == 3;
    let cx = if east { u_base + arm } else { u_base };
    let cy = if north { v_base + arm } else { v_base };
    let sx = if east { -1.0 } else { 1.0 };
    let sy = if north { -1.0 } else { 1.0 };
    let at = |du: f64, dv: f64| -> [f64; 2] { [cx + du * sx, cy + dv * sy] };
    vec![
        at(0.0, 0.0),
        at(arm, 0.0),
        at(arm, width),
        at(width, width),
        at(width, arm),
        at(0.0, arm),
        at(0.0, 0.0),
    ]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrenchConfig {
    pub standard_length: TrenchStandardLength,
    pub width_m: f64,
    pub coverage_ratio: f64,
    pub angle_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrenchBox {
    pub name: String,
    pub polygon: Polygon<f64>,
    pub width_m: f64,
    pub length_m: f64,
}

pub fn generate_standard_trenches(area: &Polygon<f64>, cfg: &TrenchConfig) -> Vec<TrenchBox> {
    let Some(bbox) = area.bounding_rect() else {
        return Vec::new();
    };
    let len = cfg.standard_length.meters();
    let wid = cfg.width_m.max(1.0);
    let single_area = len * wid;
    let target_area = area.unsigned_area() * cfg.coverage_ratio.clamp(0.01, 0.50);
    let max_count = ((target_area / single_area).ceil() as usize).max(1);

    let step_x = len + 6.0;
    let step_y = wid + 6.0;
    let mut out = Vec::new();
    let mut idx = 1usize;

    let mut y = bbox.min().y + 2.0;
    while y + wid <= bbox.max().y - 2.0 && out.len() < max_count {
        let mut x = bbox.min().x + 2.0;
        while x + len <= bbox.max().x - 2.0 && out.len() < max_count {
            let rect = Polygon::new(
                LineString::from(vec![
                    Coord { x, y },
                    Coord { x: x + len, y },
                    Coord {
                        x: x + len,
                        y: y + wid,
                    },
                    Coord { x, y: y + wid },
                    Coord { x, y },
                ]),
                vec![],
            );
            let rotated = if cfg.angle_deg.abs() > 1e-6 {
                rect.rotate_around_centroid(cfg.angle_deg)
            } else {
                rect
            };
            if area.contains(&rotated) {
                out.push(TrenchBox {
                    name: format!("T-{idx}"),
                    polygon: rotated,
                    width_m: wid,
                    length_m: len,
                });
                idx += 1;
            }
            x += step_x;
        }
        y += step_y;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;

    #[test]
    fn trench_grid_uses_only_selected_standard_length_without_shrinking() {
        let area = wkt!(POLYGON((0.0 0.0, 100.0 0.0, 100.0 80.0, 0.0 80.0, 0.0 0.0)));
        let cfg = TrenchConfig {
            standard_length: TrenchStandardLength::M20,
            width_m: 2.0,
            coverage_ratio: 0.10,
            angle_deg: 0.0,
        };
        let trenches = generate_standard_trenches(&area, &cfg);
        assert!(!trenches.is_empty());
        for t in &trenches {
            assert!((t.length_m - 20.0).abs() < 1e-6);
            assert!((t.width_m - 2.0).abs() < 1e-6);
        }
    }

    #[test]
    fn lshape_trench_all_four_bends_keep_exact_15m_arms_and_snap_to_coordinate_grid() {
        for bend in 0..=3 {
            let ring = build_lshape_ring(102.3, 204.7, 15.0, 2.0, bend, Some(5.0));
            assert_eq!(ring.len(), 7);
            let poly = Polygon::new(
                LineString::from(
                    ring.iter()
                        .map(|&p| Coord { x: p[0], y: p[1] })
                        .collect::<Vec<_>>(),
                ),
                vec![],
            );
            assert!((poly.unsigned_area() - 56.0).abs() < 1e-6);
            assert!((ring[0][0] % 5.0).abs() < 1e-6);
            assert!((ring[0][1] % 5.0).abs() < 1e-6);
        }
    }
}
