/// C++ `HGIS_TRENCH_PROFILE_SAMPLES` (100) — 트렌치 단면 기본 등간격 샘플 분할 수
pub const HGIS_TRENCH_PROFILE_SAMPLES: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct TrenchProfile3d {
    pub start_xy: [f64; 2],
    pub end_xy: [f64; 2],
    pub length_m: f64,
    pub azimuth_deg: f64,
    pub slope_deg: f64,
    pub samples_dist_elev: Vec<[f64; 2]>,
}

impl TrenchProfile3d {
    pub fn from_endpoints(start_xy: [f64; 2], end_xy: [f64; 2], start_z: f64, end_z: f64) -> Self {
        let dx = end_xy[0] - start_xy[0];
        let dy = end_xy[1] - start_xy[1];
        let length_m = (dx * dx + dy * dy).sqrt();
        let azimuth_deg = if length_m > 1e-6 {
            dx.atan2(dy).to_degrees().rem_euclid(360.0)
        } else {
            0.0
        };
        let slope_deg = if length_m > 1e-6 {
            (end_z - start_z).atan2(length_m).to_degrees()
        } else {
            0.0
        };
        let steps = 16usize;
        let mut samples_dist_elev = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = (i as f64) / (steps as f64);
            samples_dist_elev.push([length_m * t, start_z + (end_z - start_z) * t]);
        }
        Self {
            start_xy,
            end_xy,
            length_m,
            azimuth_deg,
            slope_deg,
            samples_dist_elev,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trench_profile_3d_slope_angle_45deg_and_zero_length_fallback() {
        // TestKaTrenchProfile3D::testSlopeAngle45Deg: (0,0,100) -> (10,0,110) => 45.0 deg
        let p = TrenchProfile3d::from_endpoints([0.0, 0.0], [10.0, 0.0], 100.0, 110.0);
        assert!((p.length_m - 10.0).abs() < 1e-6);
        assert!((p.slope_deg - 45.0).abs() < 1e-6);
        assert!((p.azimuth_deg - 90.0).abs() < 1e-6);

        // Zero horizontal length fallback -> 0.0 deg slope (no NaN / 90 deg spike)
        let zero = TrenchProfile3d::from_endpoints([5.0, 5.0], [5.0, 5.0], 100.0, 125.0);
        assert_eq!(zero.length_m, 0.0);
        assert_eq!(zero.slope_deg, 0.0);
    }
}
