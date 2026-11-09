//! 표준 시굴 트렌치(20·15·10m 고정) 배치.
//!

use geo::{Area, BoundingRect, Contains, Coord, LineString, Polygon, Rotate};
use serde::{Deserialize, Serialize};

use crate::trench_types::TrenchStandardLength;

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
