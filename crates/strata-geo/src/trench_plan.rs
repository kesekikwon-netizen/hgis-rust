//! Straight trial-trench plan types and grid builder (`target/cpp-ref/src/core/TrenchGridGenerator.h:15-130`).

pub use crate::trench_plan_area::{area_problem, build_in_area, pick_auto_fill_area};
pub use crate::trench_plan_ratio::{
    build_for_target_ratio, number_from_high_side, upslope_aspect, DEFAULT_MIN_SLOPE_PCT,
};

use geo::Geometry;

pub(crate) const MAX_WIDTH: f64 = 2.0;
pub(crate) const MAX_LENGTH: f64 = 20.0;
pub(crate) const MAX_CELLS: usize = 200_000;
const GEOMETRY_TOLERANCE: f64 = 1e-7;

/// Grid specification for straight trenches (`target/cpp-ref/src/core/TrenchGridGenerator.h:15-30`).
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub origin_x: f64,
    pub origin_y: f64,
    pub trench_width: f64,
    pub trench_length: f64,
    pub balk_width: f64,
    pub rows: i32,
    pub cols: i32,
    pub azimuth_deg: f64,
    pub name_prefix: String,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            origin_x: 0.0,
            origin_y: 0.0,
            trench_width: 2.0,
            trench_length: 20.0,
            balk_width: 1.0,
            rows: 1,
            cols: 1,
            azimuth_deg: 0.0,
            name_prefix: "Tr-".to_owned(),
        }
    }
}

/// One straight trench cell (`target/cpp-ref/src/core/TrenchGridGenerator.h:32-42`).
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub name: String,
    pub width: f64,
    pub length: f64,
    /// Closed 5-point ring in work CRS metres (`ring[4] == ring[0]`).
    pub ring: Vec<[f64; 2]>,
}

/// One `survey_area` feature in work CRS (`target/cpp-ref/src/core/TrenchGridGenerator.h:48-51`).
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyPoly {
    pub area: Option<Geometry<f64>>,
    pub fid: i64,
}

/// Survey area chosen for auto-fill (`target/cpp-ref/src/core/TrenchGridGenerator.h:53-59`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PickedArea {
    pub area: Option<Geometry<f64>>,
    pub area_m2: f64,
    pub used_count: usize,
    pub total_count: usize,
    pub used_selection: bool,
}

/// One elevation sample in work CRS metres (`target/cpp-ref/src/core/TrenchGridGenerator.h:88-92`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ElevSample {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Least-squares upslope azimuth and slope (`target/cpp-ref/src/core/TrenchGridGenerator.h:97-102`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SlopeAspect {
    pub azimuth_deg: f64,
    pub slope_pct: f64,
    pub valid: bool,
    pub samples: usize,
}

/// Result of target-ratio trench planning (`target/cpp-ref/src/core/TrenchGridGenerator.h:109-118`).
#[derive(Debug, Clone, PartialEq)]
pub struct RatioFill {
    pub cells: Vec<Cell>,
    pub length: f64,
    pub balk: f64,
    pub ratio_pct: f64,
    pub area_m2: f64,
    pub azimuth_deg: f64,
    pub error: String,
}

impl Default for RatioFill {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            length: 0.0,
            balk: 10.0,
            ratio_pct: 0.0,
            area_m2: 0.0,
            azimuth_deg: 0.0,
            error: String::new(),
        }
    }
}

/// Dimension bounds check (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:39-42`).
pub(crate) fn valid_dimensions(width: f64, length: f64) -> bool {
    width.is_finite()
        && width > 0.0
        && width <= MAX_WIDTH
        && length.is_finite()
        && length > 0.0
        && length <= MAX_LENGTH
}

/// Straight-trench spec validation (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:44-48`).
pub(crate) fn valid_spec(spec: &Spec) -> bool {
    valid_dimensions(spec.trench_width, spec.trench_length)
        && spec.balk_width.is_finite()
        && spec.balk_width >= 0.0
        && spec.azimuth_deg.is_finite()
        && spec.origin_x.is_finite()
        && spec.origin_y.is_finite()
}

/// Validates that `cell` is a closed rectangle matching its attributes (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:50-68`).
pub(crate) fn valid_cell(cell: &Cell) -> bool {
    if cell.ring.len() != 5 || !valid_dimensions(cell.width, cell.length) {
        return false;
    }
    if cell.ring.iter().any(|pt| !pt[0].is_finite() || !pt[1].is_finite()) {
        return false;
    }
    let p = &cell.ring;
    let (ux, uy) = (p[1][0] - p[0][0], p[1][1] - p[0][1]);
    let (vx, vy) = (p[3][0] - p[0][0], p[3][1] - p[0][1]);
    let width = ux.hypot(uy);
    let length = vx.hypot(vy);
    if !width.is_finite()
        || width <= 0.0
        || !length.is_finite()
        || length <= 0.0
        || (width - cell.width).abs() > GEOMETRY_TOLERANCE
        || (length - cell.length).abs() > GEOMETRY_TOLERANCE
    {
        return false;
    }
    (p[4][0] - p[0][0]).hypot(p[4][1] - p[0][1]) <= GEOMETRY_TOLERANCE
        && ((p[2][0] - p[1][0]) - vx).hypot((p[2][1] - p[1][1]) - vy) <= GEOMETRY_TOLERANCE
        && ((ux / width) * (vx / length) + (uy / width) * (vy / length)).abs() <= GEOMETRY_TOLERANCE
}

/// Builds a manual `rows × cols` straight trench grid (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:72-111`).
#[must_use]
pub fn build(spec: &Spec) -> Vec<Cell> {
    if !valid_spec(spec) || spec.rows <= 0 || spec.cols <= 0 {
        return Vec::new();
    }
    let rows = spec.rows as usize;
    let cols = spec.cols as usize;
    if rows > MAX_CELLS / cols {
        return Vec::new();
    }
    let w = spec.trench_width;
    let len = spec.trench_length;
    let balk = spec.balk_width;
    // Match C++ `spec.azimuthDeg * M_PI / 180.0` (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:85`).
    let a = spec.azimuth_deg * std::f64::consts::PI / 180.0;
    let (c, s) = (a.cos(), a.sin());
    let world = |lx: f64, ly: f64| -> [f64; 2] {
        [lx * c + ly * s + spec.origin_x, -lx * s + ly * c + spec.origin_y]
    };

    let mut out = Vec::with_capacity(rows * cols);
    let mut n = 1usize;
    for r in 0..rows {
        for col in 0..cols {
            let x0 = (col as f64) * (w + balk);
            let y0 = (r as f64) * (len + balk);
            let p0 = world(x0, y0);
            let cell = Cell {
                name: format!("{}{n}", spec.name_prefix),
                width: w,
                length: len,
                ring: vec![
                    p0,
                    world(x0 + w, y0),
                    world(x0 + w, y0 + len),
                    world(x0, y0 + len),
                    p0,
                ],
            };
            n += 1;
            if !valid_cell(&cell) {
                return Vec::new();
            }
            out.push(cell);
        }
    }
    out
}

/// Sum of `width × length` across all cells (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:524-529`).
#[must_use]
pub fn total_area(cells: &[Cell]) -> f64 {
    let mut sum = 0.0;
    for c in cells {
        sum += c.width * c.length;
    }
    sum
}

/// Validates a non-empty list of straight trench cells (`target/cpp-ref/src/core/TrenchGridGenerator.cpp:531-543`).
///
/// # Errors
/// Returns the Korean validation message when the list is empty, over the cell limit or contains an invalid cell.
pub fn cells_valid(cells: &[Cell]) -> Result<(), String> {
    if cells.is_empty() || cells.len() > MAX_CELLS {
        return Err("기록할 시굴격자가 없거나 개수가 너무 많습니다.".to_owned());
    }
    for cell in cells {
        if !valid_cell(cell) {
            return Err(
                "시굴격자 규격과 도형이 일치하지 않습니다. 폭은 최대 2 m, 길이는 최대 20 m입니다."
                    .to_owned(),
            );
        }
    }
    Ok(())
}

/// Checks whether straight `cells` cover `target_pct` of `area_m2` (`target/cpp-ref/src/core/TrenchLShape.cpp:293-299`).
#[must_use]
pub fn meets_target(cells: &[Cell], area_m2: f64, target_pct: f64) -> bool {
    if target_pct <= 0.0 {
        return !cells.is_empty();
    }
    if cells.is_empty() || !area_m2.is_finite() || area_m2 <= 0.0 {
        return false;
    }
    let over = total_area(cells) - area_m2 * target_pct / 100.0;
    let tolerance = 1e-6_f64.max(area_m2 * 1e-8);
    over.abs() <= tolerance
}
