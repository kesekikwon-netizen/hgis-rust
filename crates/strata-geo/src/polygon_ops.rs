use geo::algorithm::bool_ops::BooleanOps;
use geo::{Area, Coord, LineString, MultiPolygon, Polygon};
use strata_types::error::{StrataError, StrataResult};

/// C++ `GeorefHelmert::calculateTransform` 반환 상태 (`enum class HelmertStatus`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelmertStatus {
    Ok,
    InsufficientPoints,
    SingularMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelmertTransform2D {
    pub scale: f64,
    pub rotation_rad: f64,
    pub tx: f64,
    pub ty: f64,
    pub rms_error_m: f64,
}

impl HelmertTransform2D {
    #[must_use]
    pub fn apply(&self, xy: [f64; 2]) -> [f64; 2] {
        let c = self.rotation_rad.cos() * self.scale;
        let s = self.rotation_rad.sin() * self.scale;
        [
            c * xy[0] - s * xy[1] + self.tx,
            s * xy[0] + c * xy[1] + self.ty,
        ]
    }
}

pub fn erase_polygon(subject: &Polygon<f64>, cutter: &Polygon<f64>) -> MultiPolygon<f64> {
    subject.difference(cutter)
}

pub fn union_polygons(a: &Polygon<f64>, b: &Polygon<f64>) -> MultiPolygon<f64> {
    a.union(b)
}

#[must_use]
pub fn split_polygon_by_line(
    subject: &Polygon<f64>,
    line_start: [f64; 2],
    line_end: [f64; 2],
) -> Vec<Polygon<f64>> {
    let dx = line_end[0] - line_start[0];
    let dy = line_end[1] - line_start[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return vec![subject.clone()];
    }
    // 0.002m(2mm) 절단선 블레이드로 Boolean difference를 수행해 조각을 얻는다.
    let nx = -dy / len * 0.001;
    let ny = dx / len * 0.001;
    let blade = Polygon::new(
        LineString::from(vec![
            Coord {
                x: line_start[0] + nx,
                y: line_start[1] + ny,
            },
            Coord {
                x: line_end[0] + nx,
                y: line_end[1] + ny,
            },
            Coord {
                x: line_end[0] - nx,
                y: line_end[1] - ny,
            },
            Coord {
                x: line_start[0] - nx,
                y: line_start[1] - ny,
            },
            Coord {
                x: line_start[0] + nx,
                y: line_start[1] + ny,
            },
        ]),
        vec![],
    );
    subject
        .difference(&blade)
        .0
        .into_iter()
        .filter(|p| p.exterior().0.len() >= 4 && p.unsigned_area() >= 1e-4)
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_split_and_helmert_georef_produce_two_pieces_and_submillimeter_rms() -> StrataResult<()>
    {
        let rect = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 100.0, y: 0.0 },
                Coord { x: 100.0, y: 100.0 },
                Coord { x: 0.0, y: 100.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let pieces = split_polygon_by_line(&rect, [50.0, -10.0], [50.0, 110.0]);
        assert_eq!(pieces.len(), 2);

        let src = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]];
        let dst = [[200_000.0, 500_000.0], [200_200.0, 500_000.0], [200_000.0, 500_200.0]];
        let tf = solve_helmert_georef(&src, &dst)?;
        assert!((tf.scale - 2.0).abs() < 1e-6);
        assert!(tf.rms_error_m < 1e-6);
        let mapped = tf.apply([50.0, 50.0]);
        assert!((mapped[0] - 200_100.0).abs() < 1e-5);
        assert!((mapped[1] - 500_100.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_georef_helmert_45deg_scale2_and_collinear_singular_matrix_detection() {
        let rot = std::f64::consts::FRAC_PI_4; // 45 deg = 0.785398163...
        let scale = 2.0_f64;
        let tx = 100.0_f64;
        let ty = 200.0_f64;
        let c = scale * rot.cos();
        let s = scale * rot.sin();

        let src = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let dst: Vec<[f64; 2]> = src
            .iter()
            .map(|&[x, y]| [c * x - s * y + tx, s * x + c * y + ty])
            .collect();

        let (status, tf_opt) = calculate_helmert_transform_checked(&src, &dst);
        assert_eq!(status, HelmertStatus::Ok);
        let tf = tf_opt.expect("helmert transform must exist");
        assert!((tf.scale - 2.0).abs() < 1e-6);
        assert!((tf.rotation_rad - rot).abs() < 1e-6);
        assert!(tf.rms_error_m < 1e-9);

        // 4 collinear points on y = 2x -> SingularMatrix
        let collinear_src = [[0.0, 0.0], [1.0, 2.0], [2.0, 4.0], [3.0, 6.0]];
        let (col_status, col_tf) = calculate_helmert_transform_checked(&collinear_src, &dst);
        assert_eq!(col_status, HelmertStatus::SingularMatrix);
        assert!(col_tf.is_none());
    }

    #[test]
    fn test_polygon_ops_split_10x10_square_yields_two_50m2_pieces() {
        let sq = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let pieces = split_polygon_by_line(&sq, [5.0, -2.0], [5.0, 12.0]);
        assert_eq!(pieces.len(), 2);
        let total_area: f64 = pieces.iter().map(Area::unsigned_area).sum();
        assert!((total_area - 100.0).abs() < 0.1);
        for p in &pieces {
            assert!((p.unsigned_area() - 50.0).abs() < 0.1);
        }
    }
}
