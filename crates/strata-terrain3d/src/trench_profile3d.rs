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
        let azimuth_deg = dx.atan2(dy).to_degrees().rem_euclid(360.0);
        let slope_deg = if length_m > 1e-6 {
            ((end_z - start_z) / length_m).atan().to_degrees()
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
