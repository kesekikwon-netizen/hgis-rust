//! 4점 호모그래피 해석.
//!
//! `section_renderer` 에서 분리했다.

/// 정사 단면 기본 출력 해상도 (mm/px, C++ `HGIS_DEFAULT_SECTION_RES_MM`)
pub const HGIS_DEFAULT_SECTION_RES_MM: f64 = 2.0;

/// 단면 사진 타겟 핀 픽셀 좌표 및 단면 지상 좌표 쌍 (C++ `TargetPinPair`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetPinPair {
    pub photo_pixel: [f64; 2],
    pub section_world: [f64; 2],
}

/// 호모그래피 사영 행렬 산출 에러 (C++ `HomographyError`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomographyError {
    CollinearOrConcavePins,
}

fn is_convex_quad(pts: &[[f64; 2]; 4]) -> bool {
    let mut sign = 0_i8;
    for i in 0..4 {
        let a = pts[i];
        let b = pts[(i + 1) % 4];
        let c = pts[(i + 2) % 4];
        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        if cross.abs() <= 1e-6 {
            return false;
        }
        let s = if cross > 0.0 { 1 } else { -1 };
        if sign == 0 {
            sign = s;
        } else if sign != s {
            return false;
        }
    }
    true
}

#[allow(clippy::needless_range_loop)]
fn solve_8x8_linear_system(mut m: [[f64; 8]; 8], mut rhs: [f64; 8]) -> Option<[f64; 8]> {
    for col in 0..8 {
        let mut pivot = col;
        for row in (col + 1)..8 {
            if m[row][col].abs() > m[pivot][col].abs() {
                pivot = row;
            }
        }
        if m[pivot][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        let div = m[col][col];
        for j in col..8 {
            m[col][j] /= div;
        }
        rhs[col] /= div;
        for row in 0..8 {
            if row != col {
                let factor = m[row][col];
                for j in col..8 {
                    m[row][j] -= factor * m[col][j];
                }
                rhs[row] -= factor * rhs[col];
            }
        }
    }
    Some(rhs)
}

/// 4점 타겟 핀 기준 2D 사영 변환(DLT) 3x3 호모그래피 행렬(`h33 = 1.0`) 산출
/// (C++ `KaHomographySolver::solveHomography4Point` 1:1 대응)
pub fn solve_homography_4point(pins: &[TargetPinPair; 4]) -> Result<[f64; 9], HomographyError> {
    let src_quad = [
        pins[0].photo_pixel,
        pins[1].photo_pixel,
        pins[2].photo_pixel,
        pins[3].photo_pixel,
    ];
    let dst_quad = [
        pins[0].section_world,
        pins[1].section_world,
        pins[2].section_world,
        pins[3].section_world,
    ];
    if !is_convex_quad(&src_quad) || !is_convex_quad(&dst_quad) {
        return Err(HomographyError::CollinearOrConcavePins);
    }

    let mut mat = [[0.0_f64; 8]; 8];
    let mut rhs = [0.0_f64; 8];
    for (i, pin) in pins.iter().enumerate() {
        let u = pin.photo_pixel[0];
        let v = pin.photo_pixel[1];
        let x = pin.section_world[0];
        let y = pin.section_world[1];
        let r0 = i * 2;
        let r1 = r0 + 1;
        mat[r0] = [u, v, 1.0, 0.0, 0.0, 0.0, -u * x, -v * x];
        rhs[r0] = x;
        mat[r1] = [0.0, 0.0, 0.0, u, v, 1.0, -u * y, -v * y];
        rhs[r1] = y;
    }
    let sol = solve_8x8_linear_system(mat, rhs).ok_or(HomographyError::CollinearOrConcavePins)?;
    Ok([
        sol[0], sol[1], sol[2], sol[3], sol[4], sol[5], sol[6], sol[7], 1.0,
    ])
}

/// 3x3 호모그래피 행렬을 이용한 픽셀 좌표 `(u, v)` -> 단면 격자 좌표 `(d_proj, Z)` 사영 투영
#[must_use]
pub fn project_homography_point(h: &[f64; 9], uv: [f64; 2]) -> [f64; 2] {
    let u = uv[0];
    let v = uv[1];
    let w = h[6] * u + h[7] * v + h[8];
    if w.abs() <= 1e-12 {
        return [0.0, 0.0];
    }
    [
        (h[0] * u + h[1] * v + h[2]) / w,
        (h[3] * u + h[4] * v + h[5]) / w,
    ]
}
