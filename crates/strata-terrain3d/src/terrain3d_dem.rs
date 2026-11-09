//! DEM 겹선형 표고 샘플링과 3차원 벡터 헬퍼.
//!
//! `viewport3d_renderer` 에서 분리했다.

use crate::atmosphere_agx::sample_bilinear;

pub const HGIS_DEM_NODATA_VAL: f32 = -9999.0;

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
pub(crate) fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub(crate) fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub(crate) fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
pub(crate) fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len2 = dot3(v, v);
    if len2 <= 1e-12 {
        [0.0, 0.0, 1.0]
    } else {
        let inv = len2.sqrt().recip();
        [v[0] * inv, v[1] * inv, v[2] * inv]
    }
}

/// DEM 격자 `(u, v) ∈ [0, 1]²`에서 표고(`18.5m ~ 142.0m`)와 하도 거리를 산출
pub(crate) fn sample_dem_elevation_m(
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
