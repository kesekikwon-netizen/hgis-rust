use crate::atmosphere_agx::{agx_tonemap_srgb, nishita_sky_zenith_horizon_rgb};

pub const HGIS_Z_EXAG_MIN: f32 = 1.0;
pub const HGIS_Z_EXAG_MAX: f32 = 5.0;
pub const HGIS_Z_EXAG_DEFAULT: f32 = 1.5;
pub const HGIS_DEM_NODATA_VAL: f32 = -9999.0;

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
        self.z_exaggeration = self.z_exaggeration.clamp(HGIS_Z_EXAG_MIN, HGIS_Z_EXAG_MAX);
    }

    /// C++ `Terrain3dRenderer::setVerticalExaggeration` — 수직 과장 배율 `[1.0, 5.0]` 클램핑
    pub fn set_vertical_exaggeration(&mut self, scale: f32) {
        self.z_exaggeration = scale.clamp(HGIS_Z_EXAG_MIN, HGIS_Z_EXAG_MAX);
    }
}

/// C++ `SectionProfileExtractor::sampleDemBilinear` — 2x2 인접 셀 쌍선형 보간 및 격자 바깥/NoData 방어
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn sample_dem_bilinear(
    dem_grid: &[f32],
    width: usize,
    height: usize,
    grid_min_x: f64,
    grid_min_y: f64,
    cell_size: f64,
    target_x: f64,
    target_y: f64,
    strict_nodata: bool,
) -> f32 {
    if width < 2 || height < 2 || cell_size <= 0.0 || dem_grid.len() < width * height {
        return HGIS_DEM_NODATA_VAL;
    }
    let gx = (target_x - grid_min_x) / cell_size;
    let gy = (target_y - grid_min_y) / cell_size;
    let max_x = (width - 1) as f64;
    let max_y = (height - 1) as f64;
    if gx < 0.0 || gy < 0.0 || gx > max_x || gy > max_y {
        return HGIS_DEM_NODATA_VAL;
    }
    let c0 = (gx.floor() as usize).min(width - 2);
    let r0 = (gy.floor() as usize).min(height - 2);
    let c1 = c0 + 1;
    let r1 = r0 + 1;
    let tx = (gx - c0 as f64).clamp(0.0, 1.0) as f32;
    let ty = (gy - r0 as f64).clamp(0.0, 1.0) as f32;

    let z00 = dem_grid[r0 * width + c0];
    let z10 = dem_grid[r0 * width + c1];
    let z01 = dem_grid[r1 * width + c0];
    let z11 = dem_grid[r1 * width + c1];
    let corners = [z00, z10, z01, z11];
    let is_valid = |v: f32| v.is_finite() && v > HGIS_DEM_NODATA_VAL + 1.0;

    if strict_nodata {
        if corners.iter().any(|&v| !is_valid(v)) {
            return HGIS_DEM_NODATA_VAL;
        }
    } else {
        let valid_vals: Vec<f32> = corners.iter().copied().filter(|&v| is_valid(v)).collect();
        if valid_vals.len() < 2 {
            return HGIS_DEM_NODATA_VAL;
        }
        if valid_vals.len() < 4 {
            let fallback = valid_vals[0];
            let f00 = if is_valid(z00) { z00 } else { fallback };
            let f10 = if is_valid(z10) { z10 } else { fallback };
            let f01 = if is_valid(z01) { z01 } else { fallback };
            let f11 = if is_valid(z11) { z11 } else { fallback };
            return f00 * (1.0 - tx) * (1.0 - ty)
                + f10 * tx * (1.0 - ty)
                + f01 * (1.0 - tx) * ty
                + f11 * tx * ty;
        }
    }

    z00 * (1.0 - tx) * (1.0 - ty)
        + z10 * tx * (1.0 - ty)
        + z01 * (1.0 - tx) * ty
        + z11 * tx * ty
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

    #[test]
    fn test_section_profile_bilinear_interpolation_center_10_20_30_40_yields_exact_25() {
        let mut cam = OrbitCamera3D::default();
        cam.set_vertical_exaggeration(12.0);
        assert!((cam.z_exaggeration - 5.0).abs() < 1e-6);
        cam.set_vertical_exaggeration(0.2);
        assert!((cam.z_exaggeration - 1.0).abs() < 1e-6);

        let grid = [10.0_f32, 20.0, 30.0, 40.0];
        let z_center = sample_dem_bilinear(&grid, 2, 2, 0.0, 0.0, 10.0, 5.0, 5.0, true);
        assert!((z_center - 25.0).abs() < 1e-6);

        let z_outside = sample_dem_bilinear(&grid, 2, 2, 0.0, 0.0, 10.0, 25.0, 5.0, true);
        assert_eq!(z_outside, HGIS_DEM_NODATA_VAL);
    }
}
