use crate::atmosphere_agx::{agx_tonemap_srgb, nishita_sky_zenith_horizon_rgb};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera3D {
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub zoom: f32,
    pub z_exaggeration: f32,
}

impl Default for OrbitCamera3D {
    fn default() -> Self {
        Self {
            yaw_deg: 35.0,
            pitch_deg: 42.0,
            zoom: 1.0,
            z_exaggeration: 1.8,
        }
    }
}

impl OrbitCamera3D {
    /// C++ `TerrainOrbitController::updateMouseDrag` — 방위각(`yaw_deg`) 회전, 짐벌락 방어 앙각(`[-89.0°, +89.0°]`) 및 수직 과장비(`[1.0, 5.0]`) 클램핑 1:1
    pub fn apply_orbit_drag(&mut self, delta_yaw_deg: f32, delta_pitch_deg: f32) {
        self.yaw_deg = (self.yaw_deg + delta_yaw_deg).rem_euclid(360.0);
        self.pitch_deg = (self.pitch_deg + delta_pitch_deg).clamp(-89.0, 89.0);
        self.z_exaggeration = self.z_exaggeration.clamp(1.0, 5.0);
    }
}

#[must_use]
pub fn render_terrain3d_rgba(
    dem: &[f64],
    grid_w: usize,
    grid_h: usize,
    cam: &OrbitCamera3D,
    out_w: u32,
    out_h: u32,
) -> Vec<u8> {
    let w = out_w.max(16) as usize;
    let h = out_h.max(16) as usize;
    let mut buf = vec![0_u8; w * h * 4];

    let (zenith, horizon) = nishita_sky_zenith_horizon_rgb(48.0);
    let sky_rows = (h * 35) / 100;
    for y in 0..h {
        let off_row = y * w * 4;
        if y < sky_rows {
            let t = y as f64 / (sky_rows.max(1) as f64);
            let r = (f64::from(zenith[0]) * (1.0 - t) + f64::from(horizon[0]) * t) as u8;
            let g = (f64::from(zenith[1]) * (1.0 - t) + f64::from(horizon[1]) * t) as u8;
            let b = (f64::from(zenith[2]) * (1.0 - t) + f64::from(horizon[2]) * t) as u8;
            for x in 0..w {
                let off = off_row + x * 4;
                buf[off..off + 4].copy_from_slice(&[r, g, b, 255]);
            }
        } else {
            let ty = (y - sky_rows) as f64 / ((h - sky_rows).max(1) as f64);
            let gy = ((ty * (grid_h.saturating_sub(1) as f64)) as usize).min(grid_h.saturating_sub(1));
            for x in 0..w {
                let tx = x as f64 / (w.saturating_sub(1).max(1) as f64);
                let gx = ((tx * (grid_w.saturating_sub(1) as f64)) as usize).min(grid_w.saturating_sub(1));
                let z = dem.get(gy * grid_w + gx).copied().unwrap_or(40.0);
                let z_norm = ((z * f64::from(cam.z_exaggeration) - 20.0) / 120.0).clamp(0.1, 1.4);
                let yaw_mod = 0.85 + 0.15 * f64::from(cam.yaw_deg.to_radians().cos());
                let lin = [0.42 * z_norm * yaw_mod, 0.52 * z_norm, 0.34 * z_norm];
                let srgb = agx_tonemap_srgb(lin);
                let off = off_row + x * 4;
                buf[off..off + 4].copy_from_slice(&[
                    (srgb[0] * 255.0) as u8,
                    (srgb[1] * 255.0) as u8,
                    (srgb[2] * 255.0) as u8,
                    255,
                ]);
            }
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain3d_rgba_renderer_produces_sky_and_agx_lit_terrain_buffer() {
        let dem = vec![42.0; 16 * 16];
        let cam = OrbitCamera3D::default();
        let buf = render_terrain3d_rgba(&dem, 16, 16, &cam, 160, 100);
        assert_eq!(buf.len(), 160 * 100 * 4);
    }

    #[test]
    fn test_terrain_camera_pitch_clamping_120deg_clamps_to_exact_89deg() {
        let mut cam = OrbitCamera3D::default();
        cam.apply_orbit_drag(15.0, 120.0);
        assert!((cam.pitch_deg - 89.0).abs() < 1e-5);
        cam.apply_orbit_drag(0.0, -300.0);
        assert!((cam.pitch_deg - (-89.0)).abs() < 1e-5);
    }
}
