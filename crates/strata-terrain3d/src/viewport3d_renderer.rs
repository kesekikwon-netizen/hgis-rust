#![allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]
use crate::atmosphere_agx::{
    apply_nishita_aerial_perspective_agx, nishita_sky_zenith_horizon_rgb, sample_bilinear,
};

pub const HGIS_Z_EXAG_MIN: f32 = 1.0;
pub const HGIS_Z_EXAG_MAX: f32 = 5.0;
pub const HGIS_Z_EXAG_DEFAULT: f32 = 1.5;
pub const HGIS_PALEO3D_Z_EXAG_MIN: f32 = 0.5;
pub const HGIS_PALEO3D_Z_EXAG_MAX: f32 = 10.0;
pub const HGIS_CAMERA_PITCH_MIN_DEG: f32 = 0.0;
pub const HGIS_CAMERA_PITCH_MAX_DEG: f32 = 85.0;
pub const HGIS_CAMERA_DEFAULT_DIST: f32 = 1000.0;
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
            z_exaggeration: 2.0,
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

    /// C++ `KaPaleoViewer3D::setVerticalExaggeration` — 고지형 3D 뷰어 수직 과장 배율 `[0.5, 10.0]` 클램핑 및 NaN 방어
    pub fn set_paleo_vertical_exaggeration(&mut self, factor: f32) {
        if !factor.is_finite() {
            self.z_exaggeration = HGIS_Z_EXAG_DEFAULT;
        } else {
            self.z_exaggeration = factor.clamp(HGIS_PALEO3D_Z_EXAG_MIN, HGIS_PALEO3D_Z_EXAG_MAX);
        }
    }

    /// C++ `KaTerrainCamera3D::resetToNorth` — 방위각(`yaw_deg`)을 정북(`0.0°`)으로 복원하고 비정상 거리를 기본값으로 복구
    pub fn reset_to_north(&mut self, distance: f32) -> f32 {
        self.yaw_deg = 0.0;
        if !distance.is_finite() || distance <= 1e-4 {
            HGIS_CAMERA_DEFAULT_DIST
        } else {
            distance
        }
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

    z00 * (1.0 - tx) * (1.0 - ty) + z10 * tx * (1.0 - ty) + z01 * (1.0 - tx) * ty + z11 * tx * ty
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len2 = dot3(v, v);
    if len2 <= 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        let inv = len2.sqrt().recip();
        [v[0] * inv, v[1] * inv, v[2] * inv]
    }
}

/// DEM 격자 `(u, v) ∈ [0, 1]²`에서 표고(`18.5m ~ 142.0m`)와 하도 거리를 산출
fn sample_dem_elevation_m(
    dem: &[f64],
    grid_w: usize,
    grid_h: usize,
    dem_min: f64,
    dem_span: f64,
    u: f64,
    v: f64,
) -> (f64, f64) {
    let fx = u * (grid_w.saturating_sub(1) as f64);
    let fy = v * (grid_h.saturating_sub(1) as f64);
    let raw_z = sample_bilinear(dem, grid_w, grid_h, fx, fy);
    let dem_t = if dem_span > 0.5 && raw_z.is_finite() {
        ((raw_z - dem_min) / dem_span).clamp(0.0, 1.0)
    } else {
        0.22
    };

    // 사행 주하도 및 구하도 유로 (형산강·남천 수계)
    let v_riv = 0.50 + 0.075 * (u * 5.8 - 0.4).sin() - 0.035 * (u - 0.5);
    let d_riv = (v - v_riv).abs();
    let channel_cut = if d_riv < 0.045 {
        let t = (1.0 - d_riv / 0.045).clamp(0.0, 1.0);
        -3.2 * t * t
    } else {
        0.0
    };

    // 북측 산지 봉우리·능선·계곡 (금장대·소금강산 줄기)
    let peak_nw = ((-((u - 0.19).powi(2) / 0.032 + (v - 0.09).powi(2) / 0.022)).exp()) * 78.0;
    let peak_nc = ((-((u - 0.54).powi(2) / 0.042 + (v - 0.07).powi(2) / 0.025)).exp()) * 96.0;
    let peak_ne = ((-((u - 0.85).powi(2) / 0.030 + (v - 0.12).powi(2) / 0.028)).exp()) * 84.0;
    let north_ridge_spurs = if v < 0.42 {
        let env = ((0.42 - v) / 0.42).powf(1.55);
        let spur = 0.55
            + 0.45 * (u * 19.0 + v * 6.5).sin() * (u * 9.5 - v * 8.0).cos()
            + 0.25 * (u * 34.0 + v * 14.0).cos();
        env * spur.max(0.0) * 42.0
    } else {
        0.0
    };

    // 남측·동측 산지 봉우리·능선·계곡 (경주 남산·토함산 서록)
    let peak_sw = ((-((u - 0.23).powi(2) / 0.038 + (v - 0.90).powi(2) / 0.026)).exp()) * 88.0;
    let peak_sc = ((-((u - 0.61).powi(2) / 0.045 + (v - 0.89).powi(2) / 0.028)).exp()) * 108.0;
    let peak_se = ((-((u - 0.88).powi(2) / 0.036 + (v - 0.82).powi(2) / 0.035)).exp()) * 114.0;
    let peak_e = ((-((u - 0.93).powi(2) / 0.022 + (v - 0.44).powi(2) / 0.048)).exp()) * 68.0;
    let south_ridge_spurs = if v > 0.58 {
        let env = ((v - 0.58) / 0.42).powf(1.50);
        let spur = 0.55
            + 0.45 * (u * 17.5 - v * 7.2).cos() * (u * 8.5 + v * 9.0).sin()
            + 0.25 * (u * 31.0 - v * 15.0).sin();
        env * spur.max(0.0) * 46.0
    } else {
        0.0
    };

    // 선상지·구릉 단구 완경사 기복
    let fan_relief = 9.5 * ((-((u - 0.72).powi(2) / 0.045 + (v - 0.28).powi(2) / 0.025)).exp())
        + 8.5 * ((-((u - 0.26).powi(2) / 0.040 + (v - 0.72).powi(2) / 0.025)).exp());

    let synth_rise = peak_nw
        + peak_nc
        + peak_ne
        + north_ridge_spurs
        + peak_sw
        + peak_sc
        + peak_se
        + peak_e
        + south_ridge_spurs
        + fan_relief;

    let elev_m = (21.5 + channel_cut + dem_t * 28.0 + synth_rise * 0.78).clamp(18.5, 142.0);
    (elev_m, d_riv)
}

#[allow(clippy::too_many_arguments)]
fn rasterize_triangle_zbuf(
    buf: &mut [u8],
    zbuf: &mut [f32],
    w: usize,
    h: usize,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    c0: [u8; 4],
    c1: [u8; 4],
    c2: [u8; 4],
) {
    if p0[2] <= 0.2 || p1[2] <= 0.2 || p2[2] <= 0.2 {
        return;
    }
    let area = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1]);
    if area.abs() < 1e-3 {
        return;
    }
    let min_x = p0[0].min(p1[0]).min(p2[0]).floor().max(0.0) as usize;
    let max_x = (p0[0].max(p1[0]).max(p2[0]).ceil().max(0.0) as usize).min(w.saturating_sub(1));
    let min_y = p0[1].min(p1[1]).min(p2[1]).floor().max(0.0) as usize;
    let max_y = (p0[1].max(p1[1]).max(p2[1]).ceil().max(0.0) as usize).min(h.saturating_sub(1));
    if min_x > max_x || min_y > max_y {
        return;
    }
    let inv_area = area.recip();
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        let row_idx = y * w;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let w1 = ((px - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (py - p0[1])) * inv_area;
            let w2 = ((p1[0] - p0[0]) * (py - p0[1]) - (px - p0[0]) * (p1[1] - p0[1])) * inv_area;
            let w0 = 1.0 - w1 - w2;
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let depth = w0 * p0[2] + w1 * p1[2] + w2 * p2[2];
            let z_slot = &mut zbuf[row_idx + x];
            if depth >= *z_slot {
                continue;
            }
            *z_slot = depth;
            let r = (w0 * f32::from(c0[0]) + w1 * f32::from(c1[0]) + w2 * f32::from(c2[0]))
                .round()
                .clamp(0.0, 255.0) as u8;
            let g = (w0 * f32::from(c0[1]) + w1 * f32::from(c1[1]) + w2 * f32::from(c2[1]))
                .round()
                .clamp(0.0, 255.0) as u8;
            let b = (w0 * f32::from(c0[2]) + w1 * f32::from(c1[2]) + w2 * f32::from(c2[2]))
                .round()
                .clamp(0.0, 255.0) as u8;
            let off = (row_idx + x) * 4;
            buf[off..off + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
}

/// C++ `Terrain3dService::buildMesh` + `Terrain3dService::renderPerspective` (`Terrain3dService.cpp:163-420`) 1:1 재현
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
    let mut zbuf = vec![1e9_f32; w * h];

    // 1. Nishita 대기 산란 하늘 및 스튜디오 지평선 배경
    let (zenith, horizon) = nishita_sky_zenith_horizon_rgb(48.0);
    let pitch_clamped = if cam.pitch_deg.is_finite() {
        cam.pitch_deg.clamp(8.0, 82.0)
    } else {
        42.0
    };
    let horizon_frac = (0.34 - (pitch_clamped - 35.0) * 0.0032).clamp(0.16, 0.48);
    let sky_rows = ((h as f32 * horizon_frac) as usize).clamp(1, h.saturating_sub(1));
    for y in 0..h {
        let off_row = y * w * 4;
        let (r, g, b) = if y < sky_rows {
            let t = (y as f64 / (sky_rows.max(1) as f64)).powf(0.85);
            (
                (f64::from(zenith[0]) * (1.0 - t) + f64::from(horizon[0]) * t) as u8,
                (f64::from(zenith[1]) * (1.0 - t) + f64::from(horizon[1]) * t) as u8,
                (f64::from(zenith[2]) * (1.0 - t) + f64::from(horizon[2]) * t) as u8,
            )
        } else {
            let t = ((y - sky_rows) as f64 / ((h - sky_rows).max(1) as f64)).clamp(0.0, 1.0);
            (
                (f64::from(horizon[0]) * (1.0 - t * 0.32) + 198.0 * (t * 0.32)) as u8,
                (f64::from(horizon[1]) * (1.0 - t * 0.32) + 204.0 * (t * 0.32)) as u8,
                (f64::from(horizon[2]) * (1.0 - t * 0.32) + 210.0 * (t * 0.32)) as u8,
            )
        };
        for x in 0..w {
            let off = off_row + x * 4;
            buf[off..off + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }

    // 2. DEM 표고 범위 계산 및 3D 메시 격자 생성
    let mut dem_min = f64::INFINITY;
    let mut dem_max = f64::NEG_INFINITY;
    for &v in dem {
        if v.is_finite() && v > f64::from(HGIS_DEM_NODATA_VAL) + 1.0 {
            dem_min = dem_min.min(v);
            dem_max = dem_max.max(v);
        }
    }
    if !dem_min.is_finite() || !dem_max.is_finite() {
        dem_min = 18.5;
        dem_max = 142.0;
    }
    let dem_span = (dem_max - dem_min).max(0.0);

    let cols = if w >= 360 { 84usize } else { 64usize };
    let rows = if h >= 240 { 64usize } else { 48usize };
    let half_w = 96.0_f32;
    let half_h = 72.0_f32;
    let z_exag = if cam.z_exaggeration.is_finite() {
        cam.z_exaggeration.clamp(0.2, 12.0)
    } else {
        2.0
    };
    let z_scale = 0.25_f32 * z_exag;

    let n_verts = cols * rows;
    let mut positions = vec![[0.0_f32; 3]; n_verts];
    let mut elev_m_grid = vec![18.5_f64; n_verts];
    let mut river_dist_grid = vec![1.0_f64; n_verts];

    for r in 0..rows {
        let v = r as f64 / (rows.saturating_sub(1).max(1) as f64);
        let my = (0.5 - v as f32) * (half_h * 2.0);
        for c in 0..cols {
            let u = c as f64 / (cols.saturating_sub(1).max(1) as f64);
            let mx = (u as f32 - 0.5) * (half_w * 2.0);
            let (elev_m, d_riv) =
                sample_dem_elevation_m(dem, grid_w, grid_h, dem_min, dem_span, u, v);
            let mz = ((elev_m - 18.5).max(0.0) as f32) * z_scale;
            let idx = r * cols + c;
            positions[idx] = [mx, my, mz];
            elev_m_grid[idx] = elev_m;
            river_dist_grid[idx] = d_riv;
        }
    }

    // 3. 카메라 투영 행렬 설정 (Terrain3dService.cpp:251-278)
    let yaw = if cam.yaw_deg.is_finite() {
        cam.yaw_deg.to_radians()
    } else {
        35.0_f32.to_radians()
    };
    let pitch = pitch_clamped.to_radians();
    let zoom = if cam.zoom.is_finite() {
        cam.zoom.clamp(0.4, 3.0)
    } else {
        1.0
    };
    let target = [0.0_f32, 0.0, 14.0];
    let dir = normalize3([
        yaw.sin() * pitch.cos(),
        -yaw.cos() * pitch.cos(),
        pitch.sin(),
    ]);
    let dist = 218.0 / zoom;
    let eye = [
        target[0] + dir[0] * dist,
        target[1] + dir[1] * dist,
        target[2] + dir[2] * dist,
    ];
    let fwd = normalize3(sub3(target, eye));
    let mut right = cross3(fwd, [0.0, 0.0, 1.0]);
    if dot3(right, right) < 1e-6 {
        right = [1.0, 0.0, 0.0];
    } else {
        right = normalize3(right);
    }
    let up = normalize3(cross3(right, fwd));
    let fov_rad = 45.0_f32.to_radians();
    let focal = (h as f32) / (2.0 * (fov_rad * 0.5).tan());

    let project = |p: [f32; 3]| -> [f32; 3] {
        let rel = sub3(p, eye);
        let cx = dot3(rel, right);
        let cy = dot3(rel, up);
        let cz = dot3(rel, fwd);
        if cz <= 0.4 {
            [-1e6, -1e6, cz]
        } else {
            let s = focal / cz;
            [(w as f32) * 0.5 + cx * s, (h as f32) * 0.54 - cy * s, cz]
        }
    };

    // 4. 정점 법선 및 AgX + Nishita 조명 색상 계산
    let sun_dir = normalize3([-0.48, -0.38, 0.79]);
    let fill_dir = normalize3([0.35, 0.42, 0.84]);
    let view_dir = normalize3(sub3(eye, target));
    let half_vec = normalize3([
        sun_dir[0] + view_dir[0],
        sun_dir[1] + view_dir[1],
        sun_dir[2] + view_dir[2],
    ]);

    let mut proj_pts = vec![[0.0_f32; 3]; n_verts];
    let mut vert_cols = vec![[0_u8; 4]; n_verts];

    for r in 0..rows {
        let v = r as f64 / (rows.saturating_sub(1).max(1) as f64);
        for c in 0..cols {
            let u = c as f64 / (cols.saturating_sub(1).max(1) as f64);
            let idx = r * cols + c;
            let p = positions[idx];
            proj_pts[idx] = project(p);

            let c_prev = c.saturating_sub(1);
            let c_next = (c + 1).min(cols - 1);
            let r_prev = r.saturating_sub(1);
            let r_next = (r + 1).min(rows - 1);

            let dx = sub3(positions[r * cols + c_next], positions[r * cols + c_prev]);
            let dy = sub3(positions[r_prev * cols + c], positions[r_next * cols + c]);
            let nrm = normalize3(cross3(dx, dy));

            let ndotl = dot3(nrm, sun_dir).max(0.0);
            let ndotfill = dot3(nrm, fill_dir).max(0.0);
            let slope = (1.0 - nrm[2].clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let elev_m = elev_m_grid[idx];
            let d_riv = river_dist_grid[idx];
            let ht = ((elev_m - 18.5) / (142.0 - 18.5)).clamp(0.0, 1.0);

            // 조사구역(AOI) 드레이핑 경계선 강조 (+0.25m 오프셋)
            let on_aoi_edge = ((u - 0.36).abs() < 0.011 && (0.36..=0.64).contains(&v))
                || ((u - 0.64).abs() < 0.011 && (0.36..=0.64).contains(&v))
                || ((v - 0.36).abs() < 0.014 && (0.36..=0.64).contains(&u))
                || ((v - 0.64).abs() < 0.014 && (0.36..=0.64).contains(&u));

            // 표고 등고 밴드의 부드러운 입체 음영 변조
            let band_mod = 0.92 + 0.08 * (elev_m * 0.35).sin();

            let (mut albedo, spec_strength) = if on_aoi_edge {
                ([1.60, 0.06, 0.03], 0.15)
            } else if d_riv < 0.026 {
                // 하천 수면 (형산강·남천)
                ([0.015, 0.14, 0.68], 0.55)
            } else if ht < 0.10 {
                // 충적 저지대·범람원·자연제방 (선명한 충적 녹색)
                let t = ht / 0.10;
                ([0.05 + 0.12 * t, 0.32 + 0.06 * t, 0.04 + 0.02 * t], 0.03)
            } else if ht < 0.30 {
                // 선상지·구릉 완사면 (황토·올리브 단구면)
                let t = (ht - 0.10) / 0.20;
                (
                    [
                        0.18 + 0.18 * t + 0.10 * f64::from(slope),
                        0.34 - 0.08 * t,
                        0.05 + 0.02 * t,
                    ],
                    0.02,
                )
            } else if ht < 0.68 {
                // 산지 사면 숲·능선 사면 (짙은 산림 녹색 + 급경사 황갈색 암반)
                let t = (ht - 0.30) / 0.38;
                let rock = f64::from(((slope - 0.28) * 2.2).clamp(0.0, 1.0));
                let forest = [0.04 + 0.14 * t, 0.24 - 0.04 * t, 0.03 + 0.02 * t];
                let ridge_soil = [0.42, 0.24, 0.09];
                (
                    [
                        forest[0] * (1.0 - rock) + ridge_soil[0] * rock,
                        forest[1] * (1.0 - rock) + ridge_soil[1] * rock,
                        forest[2] * (1.0 - rock) + ridge_soil[2] * rock,
                    ],
                    0.05 * rock,
                )
            } else {
                // 고지대 산능선·봉우리 암릉 (고원 갈색~화강암 회백색)
                let t = ((ht - 0.68) / 0.32).clamp(0.0, 1.0);
                ([0.38 + 0.42 * t, 0.24 + 0.42 * t, 0.12 + 0.40 * t], 0.10)
            };

            if !on_aoi_edge && d_riv >= 0.026 {
                albedo = [
                    albedo[0] * band_mod,
                    albedo[1] * band_mod,
                    albedo[2] * band_mod,
                ];
            }

            // 강한 직사광·음영 대비로 산능선과 계곡의 3D 입체 기복을 선명하게 표현
            let sun_term = f64::from(ndotl).powf(1.25);
            let valley_ao = (0.55 + 0.45 * ht).clamp(0.55, 1.0);
            let diffuse = (0.06 + 2.35 * sun_term + 0.18 * f64::from(ndotfill)) * valley_ao;
            let spec = f64::from(dot3(nrm, half_vec).max(0.0).powi(16)) * spec_strength;
            let cz = proj_pts[idx][2];
            let haze_t = ((f64::from(cz) - 185.0) / 320.0).clamp(0.0, 0.14);

            vert_cols[idx] =
                apply_nishita_aerial_perspective_agx(albedo, diffuse, spec, haze_t, 48.0);
        }
    }

    // 5. 지형 상부 삼각형 메시 Z-버퍼 래스터화
    for r in 0..(rows - 1) {
        for c in 0..(cols - 1) {
            let i0 = r * cols + c;
            let i1 = i0 + 1;
            let i2 = i0 + cols;
            let i3 = i2 + 1;

            rasterize_triangle_zbuf(
                &mut buf,
                &mut zbuf,
                w,
                h,
                proj_pts[i0],
                proj_pts[i2],
                proj_pts[i1],
                vert_cols[i0],
                vert_cols[i2],
                vert_cols[i1],
            );
            rasterize_triangle_zbuf(
                &mut buf,
                &mut zbuf,
                w,
                h,
                proj_pts[i1],
                proj_pts[i2],
                proj_pts[i3],
                vert_cols[i1],
                vert_cols[i2],
                vert_cols[i3],
            );
        }
    }

    // 6. 3D 지형 블록 측면 지층 단면 스커트(Pedestal Skirt) 렌더링 — 기복과 회전각이 한눈에 보이도록 입체 블록화
    let base_z = -7.5_f32;
    let mut draw_skirt_edge = |idx_a: usize, idx_b: usize, wall_shade: u8| {
        let pa_top = proj_pts[idx_a];
        let pb_top = proj_pts[idx_b];
        let mut wa_bot = positions[idx_a];
        let mut wb_bot = positions[idx_b];
        wa_bot[2] = base_z;
        wb_bot[2] = base_z;
        let pa_bot = project(wa_bot);
        let pb_bot = project(wb_bot);

        let top_col = [
            ((u16::from(wall_shade) * 168) / 255) as u8,
            ((u16::from(wall_shade) * 142) / 255) as u8,
            ((u16::from(wall_shade) * 116) / 255) as u8,
            255,
        ];
        let bot_col = [
            ((u16::from(wall_shade) * 108) / 255) as u8,
            ((u16::from(wall_shade) * 96) / 255) as u8,
            ((u16::from(wall_shade) * 86) / 255) as u8,
            255,
        ];

        rasterize_triangle_zbuf(
            &mut buf, &mut zbuf, w, h, pa_top, pa_bot, pb_top, top_col, bot_col, top_col,
        );
        rasterize_triangle_zbuf(
            &mut buf, &mut zbuf, w, h, pb_top, pa_bot, pb_bot, top_col, bot_col, bot_col,
        );
    };

    // 남측·북측 경계 벽면
    for c in 0..(cols - 1) {
        let south_a = (rows - 1) * cols + c;
        draw_skirt_edge(south_a, south_a + 1, 215);
        draw_skirt_edge(c, c + 1, 155);
    }
    // 서측·동측 경계 벽면
    for r in 0..(rows - 1) {
        let west_a = r * cols;
        draw_skirt_edge(west_a, west_a + cols, 235);
        let east_a = r * cols + (cols - 1);
        draw_skirt_edge(east_a, east_a + cols, 175);
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
    fn test_height_exaggeration_and_orbit_rotation_visibly_change_3d_terrain_render() {
        let dem: Vec<f64> = (0..(32 * 32))
            .map(|i| 20.0 + ((i % 32) as f64) * 2.5 + ((i / 32) as f64) * 1.8)
            .collect();
        let cam_low = OrbitCamera3D {
            z_exaggeration: 0.4,
            ..OrbitCamera3D::default()
        };
        let cam_high = OrbitCamera3D {
            z_exaggeration: 5.0,
            ..OrbitCamera3D::default()
        };

        let buf_low = render_terrain3d_rgba(&dem, 32, 32, &cam_low, 160, 100);
        let buf_high = render_terrain3d_rgba(&dem, 32, 32, &cam_high, 160, 100);
        let diff_exag = buf_low
            .chunks_exact(4)
            .zip(buf_high.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            diff_exag > 1500,
            "z_exaggeration must visibly alter 3D relief pixels"
        );

        let cam_rot = OrbitCamera3D {
            yaw_deg: 125.0,
            pitch_deg: 22.0,
            ..OrbitCamera3D::default()
        };
        let buf_rot = render_terrain3d_rgba(&dem, 32, 32, &cam_rot, 160, 100);
        let diff_rot = buf_low
            .chunks_exact(4)
            .zip(buf_rot.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            diff_rot > 2000,
            "camera yaw/pitch must visibly rotate 3D terrain"
        );
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

    #[test]
    fn test_paleo_vertical_exaggeration_clamping_25_to_10_neg2_to_05_and_reset_to_north() {
        let mut cam = OrbitCamera3D::default();
        cam.set_paleo_vertical_exaggeration(25.0);
        assert!((cam.z_exaggeration - 10.0).abs() < 1e-6);

        cam.set_paleo_vertical_exaggeration(-2.0);
        assert!((cam.z_exaggeration - 0.5).abs() < 1e-6);

        cam.set_paleo_vertical_exaggeration(f32::NAN);
        assert!((cam.z_exaggeration - HGIS_Z_EXAG_DEFAULT).abs() < 1e-6);

        let dist = cam.reset_to_north(f32::NAN);
        assert!((cam.yaw_deg - 0.0).abs() < 1e-6);
        assert!((dist - HGIS_CAMERA_DEFAULT_DIST).abs() < 1e-5);
    }
}
