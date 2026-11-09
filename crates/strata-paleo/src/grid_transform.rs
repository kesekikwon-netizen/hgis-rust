//! 격자 변환(`GridTransform`)과 무데이터 상수.
//!
//! `terrain_edt_tpi` 에서 분리했다. 호출부 경로가 가장 많이 걸린 타입이다.

use serde::{Deserialize, Serialize};

pub const HGIS_NODATA_FLOAT: f64 = -9999.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GridTransform {
    pub origin_x: f64,
    pub origin_y: f64,
    pub cell_size: f64,
    pub width: usize,
    pub height: usize,
}

impl GridTransform {
    pub const fn pixel_to_world(&self, r: usize, c: usize) -> [f64; 2] {
        [
            self.origin_x + (c as f64 + 0.5) * self.cell_size,
            self.origin_y - (r as f64 + 0.5) * self.cell_size,
        ]
    }

    #[must_use]
    pub fn world_to_cell(&self, wx: f64, wy: f64) -> Option<(usize, usize)> {
        if self.cell_size <= 0.0 {
            return None;
        }
        let col_f = ((wx - self.origin_x) / self.cell_size).floor();
        let row_f = ((self.origin_y - wy) / self.cell_size).floor();
        if col_f < 0.0 || row_f < 0.0 {
            return None;
        }
        let c = col_f as usize;
        let r = row_f as usize;
        if r < self.height && c < self.width {
            Some((r, c))
        } else {
            None
        }
    }
}
