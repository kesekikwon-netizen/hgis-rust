use geo::algorithm::bool_ops::BooleanOps;
use geo::{Coord, LineString, MultiPolygon, Polygon};
use strata_types::error::{StrataError, StrataResult};

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
    subject.difference(&blade).0
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
}
