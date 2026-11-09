//! GCP 기반 헬머트·아핀 변환 풀이.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use crate::boolean_split::HelmertStatus;
use crate::helmert_solve::calculate_helmert_transform_checked;

/// 어핀 변환 최소 GCP 수 (C++ `HGIS_MIN_AFFINE_GCP_COUNT`)
pub const HGIS_MIN_AFFINE_GCP_COUNT: usize = 3;
/// 헬머트 변환 최소 GCP 수 (C++ `HGIS_MIN_HELMERT_GCP_COUNT`)
pub const HGIS_MIN_HELMERT_GCP_COUNT: usize = 2;
/// 실측도 허용 최대 RMSE 한계 (m, C++ `HGIS_MAX_ALLOWABLE_RMSE_M`)
pub const HGIS_MAX_ALLOWABLE_RMSE_M: f64 = 0.25;
/// 특이 행렬 판정 임계치 (C++ `HGIS_MATRIX_SINGULAR_EPSILON`)
pub const HGIS_MATRIX_SINGULAR_EPSILON: f64 = 1e-12;

/// 지오레퍼런싱 변환 모델 유형 (C++ `TransformType`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformType {
    /// 4-파라미터 헬머트 (최소 2점)
    Helmert4Param,
    /// 6-파라미터 2D 어핀 (`X = a*u + b*v + c`, `Y = d*u + e*v + f` / 최소 3점)
    Affine6Param,
}

/// 이미지 픽셀 좌표 - 실세계 좌표 기준점 쌍 (C++ `GcpPointPair`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcpPointPair {
    pub source_pixel: [f64; 2],
    pub target_world: [f64; 2],
    pub is_enabled: bool,
}

/// GCP 변환 해 산출 에러 (C++ `GcpSolverError`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcpSolverError {
    InsufficientGcpCount,
    SingularMatrixCollinear,
}

/// 지오레퍼런싱 변환 계수 및 잔차·RMSE 결과 (C++ `GeoreferenceResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct GeoreferenceResult {
    pub is_success: bool,
    pub transform_type: TransformType,
    /// `[a, b, c, d, e, f]` where `X = a*u + b*v + c`, `Y = d*u + e*v + f`
    pub transform_coeffs: [f64; 6],
    pub residuals_m: Vec<f64>,
    pub rmse_m: f64,
    pub exceeds_allowable_rmse: bool,
}

#[allow(clippy::needless_range_loop)]
fn solve_3x3_linear_system(mut m: [[f64; 3]; 3], mut rhs: [f64; 3]) -> Option<[f64; 3]> {
    for col in 0..3 {
        let mut pivot = col;
        for row in (col + 1)..3 {
            if m[row][col].abs() > m[pivot][col].abs() {
                pivot = row;
            }
        }
        if m[pivot][col].abs() < HGIS_MATRIX_SINGULAR_EPSILON {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        let div = m[col][col];
        for j in col..3 {
            m[col][j] /= div;
        }
        rhs[col] /= div;
        for row in 0..3 {
            if row != col {
                let factor = m[row][col];
                for j in col..3 {
                    m[row][j] -= factor * m[col][j];
                }
                rhs[row] -= factor * rhs[col];
            }
        }
    }
    Some(rhs)
}

/// 최소자승법 기반 6-파라미터 Affine 또는 4-파라미터 Helmert 지오레퍼런싱 변환 계수 및 RMSE 산출
/// (C++ `KaGcpTransformSolver::solveTransform` 1:1 대응)
pub fn solve_gcp_transform(
    gcp_list: &[GcpPointPair],
    transform_type: TransformType,
) -> Result<GeoreferenceResult, GcpSolverError> {
    let active: Vec<&GcpPointPair> = gcp_list.iter().filter(|g| g.is_enabled).collect();
    let min_req = match transform_type {
        TransformType::Affine6Param => HGIS_MIN_AFFINE_GCP_COUNT,
        TransformType::Helmert4Param => HGIS_MIN_HELMERT_GCP_COUNT,
    };
    if active.len() < min_req {
        return Err(GcpSolverError::InsufficientGcpCount);
    }

    let coeffs = match transform_type {
        TransformType::Affine6Param => {
            let mut ata = [[0.0_f64; 3]; 3];
            let mut atx = [0.0_f64; 3];
            let mut aty = [0.0_f64; 3];
            for g in &active {
                let u = g.source_pixel[0];
                let v = g.source_pixel[1];
                let x = g.target_world[0];
                let y = g.target_world[1];
                let row = [u, v, 1.0];
                for i in 0..3 {
                    for j in 0..3 {
                        ata[i][j] += row[i] * row[j];
                    }
                    atx[i] += row[i] * x;
                    aty[i] += row[i] * y;
                }
            }
            let abc =
                solve_3x3_linear_system(ata, atx).ok_or(GcpSolverError::SingularMatrixCollinear)?;
            let def =
                solve_3x3_linear_system(ata, aty).ok_or(GcpSolverError::SingularMatrixCollinear)?;
            [abc[0], abc[1], abc[2], def[0], def[1], def[2]]
        }
        TransformType::Helmert4Param => {
            let src_pts: Vec<[f64; 2]> = active.iter().map(|g| g.source_pixel).collect();
            let dst_pts: Vec<[f64; 2]> = active.iter().map(|g| g.target_world).collect();
            let (status, h) = calculate_helmert_transform_checked(&src_pts, &dst_pts);
            match status {
                HelmertStatus::Ok => {
                    let h = h.ok_or(GcpSolverError::SingularMatrixCollinear)?;
                    let c = h.rotation_rad.cos() * h.scale;
                    let s = h.rotation_rad.sin() * h.scale;
                    [c, -s, h.tx, s, c, h.ty]
                }
                HelmertStatus::InsufficientPoints => {
                    return Err(GcpSolverError::InsufficientGcpCount);
                }
                HelmertStatus::SingularMatrix => {
                    return Err(GcpSolverError::SingularMatrixCollinear);
                }
            }
        }
    };

    let mut residuals_m = Vec::with_capacity(active.len());
    let mut sum_sq = 0.0_f64;
    for g in &active {
        let u = g.source_pixel[0];
        let v = g.source_pixel[1];
        let x_hat = coeffs[0] * u + coeffs[1] * v + coeffs[2];
        let y_hat = coeffs[3] * u + coeffs[4] * v + coeffs[5];
        let res = (x_hat - g.target_world[0]).hypot(y_hat - g.target_world[1]);
        residuals_m.push(res);
        sum_sq += res * res;
    }
    let rmse_m = (sum_sq / active.len() as f64).sqrt();
    Ok(GeoreferenceResult {
        is_success: true,
        transform_type,
        transform_coeffs: coeffs,
        residuals_m,
        rmse_m,
        exceeds_allowable_rmse: rmse_m > HGIS_MAX_ALLOWABLE_RMSE_M,
    })
}
