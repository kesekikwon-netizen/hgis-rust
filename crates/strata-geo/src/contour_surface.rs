//! A height grid and its contour lines.
//!
//! C++ builds the same surface with `GDALGridCreate(GGA_Linear)` and
//! `GDALContourGenerateEx` (`target/cpp-ref/src/core/SurveyContourWrite.cpp`).
//! Those two calls have no safe wrapper in gdal 0.19, and this workspace forbids
//! `unsafe`, so the grid is the same linear Delaunay interpolation and the lines
//! are the same interval crossings of that grid. `spade` builds the triangulation.

use spade::{DelaunayTriangulation, HasPosition, Point2, Triangulation};

pub(super) const NODATA: f32 = -9999.0;

#[path = "contour_surface_lines.rs"]
mod lines;
/// `SurveyContourBuilder` refuses a grid above this many cells.
const MAX_CELLS: usize = 4_000_000;

#[derive(Clone, Copy)]
struct Vertex {
    pos: Point2<f64>,
    z: f64,
}

impl HasPosition for Vertex {
    type Scalar = f64;
    fn position(&self) -> Point2<f64> {
        self.pos
    }
}

/// One contour polyline, in the same coordinates as the input points.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceContour {
    pub elevation_m: f64,
    /// Every fifth level from a base of 0 (`indexEvery` 5, `KaSurveyContourDialog::job`).
    pub index_line: bool,
    pub xy: Vec<[f64; 2]>,
}

/// North-up heights. `NODATA` is outside the triangulation.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightGrid {
    pub west: f64,
    pub north: f64,
    pub cell: f64,
    pub width: usize,
    pub height: usize,
    pub values: Vec<f32>,
}

/// The grid and the contour lines of `points` (x, y, z metres).
///
/// # Errors
/// Fewer than three points, a straight line of points, or a grid past the cell limit.
pub fn surface_contours(
    points: &[[f64; 3]],
    interval_m: f64,
    cell_m: f64,
) -> Result<(HeightGrid, Vec<SurfaceContour>), &'static str> {
    if points.len() < 3 {
        return Err("등고선을 만들려면 점이 3개 이상 필요합니다.");
    }
    if interval_m <= 0.0 || cell_m <= 0.0 {
        return Err("등고선 간격과 결과 폴더가 필요합니다.");
    }
    let grid = height_grid(points, cell_m)?;
    let lines = contour_lines(&grid, interval_m);
    Ok((grid, lines))
}

fn height_grid(points: &[[f64; 3]], cell: f64) -> Result<HeightGrid, &'static str> {
    let (min_x, min_y, max_x, max_y, min_z, max_z) = bounds(points);
    if (max_z - min_z).abs() < 1e-9 || collinear(points) {
        return Err("점이 일직선이라 삼각망을 만들 수 없습니다.");
    }
    let pad = cell * 0.5;
    let west = min_x - pad;
    let north = max_y + pad;
    let width = ((max_x - min_x + cell) / cell).ceil().max(2.0) as usize;
    let height = ((max_y - min_y + cell) / cell).ceil().max(2.0) as usize;
    if width.saturating_mul(height) > MAX_CELLS {
        return Err("격자 칸이 너무 많습니다. 격자 크기를 키우세요.");
    }
    let mut tri = DelaunayTriangulation::<Vertex>::new();
    for p in points {
        let _ = tri.insert(Vertex {
            pos: Point2::new(p[0], p[1]),
            z: p[2],
        });
    }
    let faces: Vec<[Vertex; 3]> = tri
        .inner_faces()
        .map(|face| face.vertices().map(|v| *v.data()))
        .collect();
    if faces.is_empty() {
        return Err("점이 일직선이라 삼각망을 만들 수 없습니다.");
    }
    let mut values = vec![NODATA; width * height];
    for row in 0..height {
        for col in 0..width {
            let x = west + (col as f64 + 0.5) * cell;
            let y = north - (row as f64 + 0.5) * cell;
            if let Some(z) = sample(&faces, x, y) {
                values[row * width + col] = z as f32;
            }
        }
    }
    Ok(HeightGrid {
        west,
        north,
        cell,
        width,
        height,
        values,
    })
}

fn bounds(points: &[[f64; 3]]) -> (f64, f64, f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for p in points {
        min_x = min_x.min(p[0]);
        min_y = min_y.min(p[1]);
        max_x = max_x.max(p[0]);
        max_y = max_y.max(p[1]);
        min_z = min_z.min(p[2]);
        max_z = max_z.max(p[2]);
    }
    (min_x, min_y, max_x, max_y, min_z, max_z)
}

fn collinear(points: &[[f64; 3]]) -> bool {
    let first = points[0];
    let far = points.iter().max_by(|a, b| {
        let da = (a[0] - first[0]).hypot(a[1] - first[1]);
        let db = (b[0] - first[0]).hypot(b[1] - first[1]);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });
    let Some(far) = far else { return true };
    let dx = far[0] - first[0];
    let dy = far[1] - first[1];
    let len = dx.hypot(dy);
    if len < 1e-9 {
        return true;
    }
    points
        .iter()
        .all(|p| ((p[0] - first[0]) * dy - (p[1] - first[1]) * dx).abs() / len < 0.05)
}

fn sample(faces: &[[Vertex; 3]], x: f64, y: f64) -> Option<f64> {
    for [a, b, c] in faces {
        if let Some((u, v, w)) = bary(x, y, a.pos, b.pos, c.pos) {
            return Some(u * a.z + v * b.z + w * c.z);
        }
    }
    None
}

fn bary(x: f64, y: f64, a: Point2<f64>, b: Point2<f64>, c: Point2<f64>) -> Option<(f64, f64, f64)> {
    let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if den.abs() < 1e-12 {
        return None;
    }
    let u = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / den;
    let v = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / den;
    let w = 1.0 - u - v;
    ((u >= -1e-8) && (v >= -1e-8) && (w >= -1e-8)).then_some((u, v, w))
}

fn contour_lines(grid: &HeightGrid, interval: f64) -> Vec<SurfaceContour> {
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for v in &grid.values {
        if *v == NODATA {
            continue;
        }
        let z = f64::from(*v);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    if !min_z.is_finite() {
        return Vec::new();
    }
    let mut k = (min_z / interval).ceil() as i64;
    if (k as f64) * interval <= min_z + 1e-6 {
        k += 1;
    }
    let mut end = (max_z / interval).floor() as i64;
    if (end as f64) * interval >= max_z - 1e-6 {
        end -= 1;
    }
    let interval_cm = (interval * 100.0).round() as i32;
    let mut lines = Vec::new();
    while k <= end {
        let level = k as f64 * interval;
        for xy in lines::chain(grid, level) {
            let cm = (level * 100.0).round() as i32;
            lines.push(SurfaceContour {
                elevation_m: level,
                index_line: interval_cm > 0 && cm.rem_euclid(interval_cm.saturating_mul(5)) == 0,
                xy,
            });
        }
        k += 1;
    }
    lines
}

#[cfg(test)]
#[path = "contour_surface_tests.rs"]
mod tests;
