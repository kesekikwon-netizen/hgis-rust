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
}
