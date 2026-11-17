//! Physical page planning, target/cpp-ref/src/core/TilePrint.cpp:261-338.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct DevicePaper {
    pub name: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub portrait: [f64; 4],
    pub landscape: [f64; 4],
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PrintMode {
    Single,
    Tiled { overlap_mm: f64, marks: bool, overview: bool },
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Page {
    pub source_mm: [f64; 4],
    pub target_mm: [f64; 4],
    pub row: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PrintPlan {
    pub landscape: bool,
    pub paper_mm: [f64; 2],
    pub printable_mm: [f64; 4],
    pub ratio: f64,
    pub overlap_mm: f64,
    pub rows: u32,
    pub cols: u32,
    pub marks: bool,
    pub overview: bool,
    pub pages: Vec<Page>,
}

pub(crate) fn build_plan(drawing: [f64; 2], paper: &DevicePaper, mode: PrintMode) -> Result<PrintPlan, String> {
    if drawing.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err("도면이나 용지 크기를 알 수 없습니다.".to_owned());
    }
    match mode {
        PrintMode::Single => {
            let landscape = drawing[0] > drawing[1];
            let (paper_mm, printable) = paper.oriented(landscape);
            let actual = drawing[0] <= paper_mm[0] + 0.5 && drawing[1] <= paper_mm[1] + 0.5;
            let ratio = if actual { 1.0 } else { (printable[2] / drawing[0]).min(printable[3] / drawing[1]) };
            let size = drawing.map(|v| v * ratio);
            let target_mm = [(paper_mm[0] - size[0]) * 0.5, (paper_mm[1] - size[1]) * 0.5, size[0], size[1]];
            Ok(PrintPlan {
                landscape,
                paper_mm,
                printable_mm: printable,
                ratio,
                overlap_mm: 0.0,
                rows: 1,
                cols: 1,
                marks: false,
                overview: false,
                pages: vec![Page { source_mm: [0.0, 0.0, drawing[0], drawing[1]], target_mm, row: 0, col: 0 }],
            })
        }
        PrintMode::Tiled { overlap_mm, marks, overview } => {
            if !overlap_mm.is_finite() || overlap_mm < 0.0 {
                return Err("겹침과 여백이 용지보다 큽니다. 겹침을 줄여 주세요.".to_owned());
            }
            let portrait = tiled(drawing, paper, false, overlap_mm, marks, overview);
            let landscape = tiled(drawing, paper, true, overlap_mm, marks, overview);
            match (portrait, landscape) {
                (Ok(a), Ok(b)) => Ok(if b.pages.len() < a.pages.len() { b } else { a }),
                (Ok(plan), Err(_)) | (Err(_), Ok(plan)) => Ok(plan),
                (Err(why), Err(_)) => Err(why),
            }
        }
    }
}

impl DevicePaper {
    pub(crate) fn oriented(&self, landscape: bool) -> ([f64; 2], [f64; 4]) {
        if landscape {
            ([self.height_mm, self.width_mm], self.landscape)
        } else {
            ([self.width_mm, self.height_mm], self.portrait)
        }
    }
}

// C++ sheetsAlong and bestPlan, target/cpp-ref/src/core/TilePrint.cpp:39-42,261-310. O(rows * cols).
fn tiled(drawing: [f64; 2], paper: &DevicePaper, landscape: bool, overlap: f64, marks: bool, overview: bool) -> Result<PrintPlan, String> {
    let (paper_mm, printable) = paper.oriented(landscape);
    let content = [printable[2], printable[3] - if marks { 6.0 } else { 0.0 }];
    if content.iter().any(|v| !v.is_finite() || *v <= overlap + 1.0) {
        return Err("겹침과 여백이 용지보다 큽니다. 겹침을 줄여 주세요.".to_owned());
    }
    let along = |length: f64, side: f64| if length <= side + 1e-6 { 1.0 } else { ((length - overlap) / (side - overlap) - 1e-9).ceil() };
    let counts = [along(drawing[0], content[0]), along(drawing[1], content[1])];
    if counts[0] * counts[1] > 10_000.0 {
        return Err("인쇄 장수가 너무 많습니다.".to_owned());
    }
    let [cols, rows] = counts.map(|v| v as u32);
    let mut pages = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            pages.push(Page {
                source_mm: [f64::from(col) * (content[0] - overlap), f64::from(row) * (content[1] - overlap), content[0], content[1]],
                target_mm: [printable[0], printable[1], content[0], content[1]],
                row,
                col,
            });
        }
    }
    Ok(PrintPlan { landscape, paper_mm, printable_mm: printable, ratio: 1.0, overlap_mm: overlap, rows, cols, marks, overview, pages })
}

#[cfg(test)]
#[path = "print_plan_tests.rs"]
mod tests;
