//! 헬머트·아핀 최소제곱 정합(중심화).
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use strata_types::error::{StrataError, StrataResult};
use crate::boolean_split::{HelmertStatus, HelmertTransform2D};

/// C++ `GeorefHelmert::calculateTransform(gcps, outResult) -> HelmertStatus`
/// 2점 미만(`InsufficientPoints`), 공선점/퇴화 행렬(`SingularMatrix`) 검사를 포함합니다.
#[must_use]
pub fn calculate_helmert_transform_checked(
    src_pts: &[[f64; 2]],
    dst_pts: &[[f64; 2]],
) -> (HelmertStatus, Option<HelmertTransform2D>) {
    if src_pts.len() < 2 || src_pts.len() != dst_pts.len() {
        return (HelmertStatus::InsufficientPoints, None);
    }
    let n = src_pts.len() as f64;
    let (mut sx, mut sy) = (0.0, 0.0);
    for s in src_pts {
        sx += s[0];
        sy += s[1];
    }
    sx /= n;
    sy /= n;

    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for s in src_pts {
        let dx = s[0] - sx;
        let dy = s[1] - sy;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    if sxx + syy < 1e-12 {
        return (HelmertStatus::SingularMatrix, None);
    }
    if src_pts.len() >= 3 && (sxx * syy - sxy * sxy).abs() < 1e-9 {
        return (HelmertStatus::SingularMatrix, None);
    }
    match solve_helmert_georef(src_pts, dst_pts) {
        Ok(tf) if tf.scale > 1e-9 && tf.scale.is_finite() => (HelmertStatus::Ok, Some(tf)),
        _ => (HelmertStatus::SingularMatrix, None),
    }
}

pub fn solve_helmert_georef(
    src_pts: &[[f64; 2]],
    dst_pts: &[[f64; 2]],
) -> StrataResult<HelmertTransform2D> {
    if src_pts.len() < 2 || src_pts.len() != dst_pts.len() {
        return Err(StrataError::InvalidInput(
            "정합(GCP) 기준점은 최소 2쌍 이상 동일 개수여야 합니다".into(),
        ));
    }
    let n = src_pts.len() as f64;
    let (mut sx, mut sy, mut dx, mut dy) = (0.0, 0.0, 0.0, 0.0);
    for (s, d) in src_pts.iter().zip(dst_pts.iter()) {
        sx += s[0];
        sy += s[1];
        dx += d[0];
        dy += d[1];
    }
    sx /= n;
    sy /= n;
    dx /= n;
    dy /= n;

    let (mut num_a, mut num_b, mut den) = (0.0, 0.0, 0.0);
    for (s, d) in src_pts.iter().zip(dst_pts.iter()) {
        let xs = s[0] - sx;
        let ys = s[1] - sy;
        let xd = d[0] - dx;
        let yd = d[1] - dy;
        num_a += xs * xd + ys * yd;
        num_b += xs * yd - ys * xd;
        den += xs * xs + ys * ys;
    }
    if den < 1e-12 {
        return Err(StrataError::InvalidInput("퇴화된 GCP 좌표쌍입니다".into()));
    }
    let a = num_a / den;
    let b = num_b / den;
    let scale = (a * a + b * b).sqrt();
    let rotation_rad = b.atan2(a);
    let tx = dx - (a * sx - b * sy);
    let ty = dy - (b * sx + a * sy);

    let mut tf = HelmertTransform2D {
        scale,
        rotation_rad,
        tx,
        ty,
        rms_error_m: 0.0,
    };
    let mut sq_err = 0.0;
    for (s, d) in src_pts.iter().zip(dst_pts.iter()) {
        let p = tf.apply(*s);
        sq_err += (p[0] - d[0]).powi(2) + (p[1] - d[1]).powi(2);
    }
    tf.rms_error_m = (sq_err / n).sqrt();
    Ok(tf)
}
