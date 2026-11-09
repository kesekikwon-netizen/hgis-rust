//! 격자 근접점 버킷(`PointGrid`)과 선분 투영 기하.
//!
//! `cad_interaction` 에서 분리했다.

use std::collections::HashMap;

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
pub(crate) fn sqr_dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

#[inline]
pub(crate) fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
