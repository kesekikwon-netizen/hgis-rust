//! 표고 단면 샘플링·범례 눈금·구덩이 깊이 캡션.
//!
//! `trench_profile3d` 에서 분리했다.

/// 지형 단면도 기본 수직 과장 배율 (5.0배, C++ `HGIS_PROFILE_Z_EXAGGERATION`)
pub const HGIS_PROFILE_Z_EXAGGERATION: f64 = 5.0;

/// DEM 기본 급간 수 (7단계, C++ `HGIS_DEFAULT_DEM_CLASS_COUNT`)
pub const HGIS_DEFAULT_DEM_CLASS_COUNT: usize = 7;

/// DEM 표고 급간 분류 모드 (C++ `ElevationClassMode`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevationClassMode {
    EqualInterval,
    Quantile,
    CustomBreaks,
}

/// DEM 급간 색상 및 범례 항목 (C++ `DemClassInterval`)
#[derive(Debug, Clone, PartialEq)]
pub struct DemClassInterval {
    pub min_elev_m: f64,
    pub max_elev_m: f64,
    pub color_rgba: [u8; 4],
    pub label: String,
}

/// 지형 단면 프로파일 샘플링 점 (C++ `ElevationProfilePoint`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationProfilePoint {
    pub chainage_m: f64,
    pub elevation_m: f64,
    pub slope_deg: f64,
}

/// DEM 고도 구간에 대해 비선형 표준 고도 눈금(`{0, 100, 200, 500, 1000, 1500, 2000}`)과 `5..=7`단계 범례 눈금을 생성
/// (C++ `DemColorRampLegend::buildDemLegendTicks` 1:1 대응)
#[must_use]
pub fn build_dem_legend_ticks(minimum: f64, maximum: f64) -> Vec<f64> {
    if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
        return Vec::new();
    }
    const STANDARD_STOPS: [f64; 7] = [0.0, 100.0, 200.0, 500.0, 1000.0, 1500.0, 2000.0];
    let mut ticks = vec![minimum];
    for &s in &STANDARD_STOPS {
        if s > minimum + 1.0 && s < maximum - 1.0 {
            ticks.push(s);
        }
    }
    ticks.push(maximum);

    if ticks.len() < 5 {
        let mut even = Vec::with_capacity(5);
        for i in 0..5 {
            let t = f64::from(i) / 4.0;
            even.push(minimum + (maximum - minimum) * t);
        }
        return even;
    }
    if ticks.len() > HGIS_DEFAULT_DEM_CLASS_COUNT {
        let mut thinned = Vec::with_capacity(HGIS_DEFAULT_DEM_CLASS_COUNT);
        thinned.push(ticks[0]);
        let inner = HGIS_DEFAULT_DEM_CLASS_COUNT - 2;
        for i in 1..=inner {
            let pos = (i as f64) * ((ticks.len() - 1) as f64) / ((inner + 1) as f64);
            let idx = (pos.round() as usize).clamp(1, ticks.len() - 2);
            if thinned
                .last()
                .is_none_or(|&last| (last - ticks[idx]).abs() > 1e-6)
            {
                thinned.push(ticks[idx]);
            }
        }
        thinned.push(*ticks.last().unwrap_or(&maximum));
        return thinned;
    }
    ticks
}

/// 2D DEM 격자에서 단면 절단선(`section_line_pts`)을 따라 `sample_step_m` 간격으로 양선형(Bilinear) 표고 및 경사도 프로파일 추출
/// (C++ `DemPresentation::sampleElevationProfile` 1:1 대응)
#[must_use]
pub fn sample_elevation_profile_bilinear(
    dem_grid: &[f32],
    cols: usize,
    rows: usize,
    cell_size_m: f64,
    origin_xy: [f64; 2],
    section_line_pts: &[[f64; 2]],
    sample_step_m: f64,
) -> Vec<ElevationProfilePoint> {
    if dem_grid.len() < cols.saturating_mul(rows)
        || cols == 0
        || rows == 0
        || !cell_size_m.is_finite()
        || cell_size_m <= 0.0
        || section_line_pts.len() < 2
    {
        return Vec::new();
    }
    let step = if sample_step_m.is_finite() && sample_step_m > 1e-3 {
        sample_step_m
    } else {
        1.0
    };

    let sample_at = |wx: f64, wy: f64| -> f64 {
        let gx = ((wx - origin_xy[0]) / cell_size_m).clamp(0.0, (cols - 1) as f64);
        let gy = ((wy - origin_xy[1]) / cell_size_m).clamp(0.0, (rows - 1) as f64);
        let c0 = (gx.floor() as usize).min(cols - 1);
        let r0 = (gy.floor() as usize).min(rows - 1);
        let c1 = (c0 + 1).min(cols - 1);
        let r1 = (r0 + 1).min(rows - 1);
        let tx = gx - (c0 as f64);
        let ty = gy - (r0 as f64);
        let z00 = f64::from(dem_grid[r0 * cols + c0]);
        let z10 = f64::from(dem_grid[r0 * cols + c1]);
        let z01 = f64::from(dem_grid[r1 * cols + c0]);
        let z11 = f64::from(dem_grid[r1 * cols + c1]);
        let top = z00 * (1.0 - tx) + z10 * tx;
        let bot = z01 * (1.0 - tx) + z11 * tx;
        top * (1.0 - ty) + bot * ty
    };

    let mut total_len = 0.0_f64;
    for seg in section_line_pts.windows(2) {
        total_len += (seg[1][0] - seg[0][0]).hypot(seg[1][1] - seg[0][1]);
    }
    let n_samples = ((total_len / step).round() as usize) + 1;
    let mut out = Vec::with_capacity(n_samples);

    let first_pt = section_line_pts[0];
    for idx in 0..n_samples {
        let chainage = ((idx as f64) * step).min(total_len);
        // Locate point along polyline at `chainage`
        let mut rem = chainage;
        let mut pos = first_pt;
        for seg in section_line_pts.windows(2) {
            let seg_len = (seg[1][0] - seg[0][0]).hypot(seg[1][1] - seg[0][1]);
            if rem <= seg_len || seg_len <= 1e-9 {
                let t = if seg_len > 1e-9 { rem / seg_len } else { 0.0 };
                pos = [
                    seg[0][0] + (seg[1][0] - seg[0][0]) * t,
                    seg[0][1] + (seg[1][1] - seg[0][1]) * t,
                ];
                break;
            }
            rem -= seg_len;
            pos = seg[1];
        }
        let elev = sample_at(pos[0], pos[1]);
        out.push(ElevationProfilePoint {
            chainage_m: chainage,
            elevation_m: elev,
            slope_deg: 0.0,
        });
    }
    for i in 0..out.len() {
        let prev = if i > 0 { i - 1 } else { i };
        let next = if i + 1 < out.len() { i + 1 } else { i };
        let dx = out[next].chainage_m - out[prev].chainage_m;
        if dx > 1e-6 {
            let dz = (out[next].elevation_m - out[prev].elevation_m).abs();
            out[i].slope_deg = (dz / dx).atan().to_degrees();
        }
    }
    out
}

/// 유구 상면·바닥 표고(`top_elev_m`, `bottom_elev_m`) 및 수동 깊이(`typed_depth_m`)로부터 유효 깊이와 접근성 설명 캡션을 계산
/// (C++ `KaElevationFigure::setValues` / `depthOf` 1:1 대응)
#[must_use]
pub fn compute_pit_elevation_depth_and_caption(
    top_elev_m: Option<f64>,
    bottom_elev_m: Option<f64>,
    typed_depth_m: Option<f64>,
) -> (Option<f64>, String) {
    let depth = match typed_depth_m.filter(|v| v.is_finite() && *v >= 0.0) {
        Some(d) => Some(d),
        None => match (
            top_elev_m.filter(|v| v.is_finite()),
            bottom_elev_m.filter(|v| v.is_finite()),
        ) {
            (Some(t), Some(b)) if t >= b => Some(t - b),
            _ => None,
        },
    };
    let top_text = top_elev_m
        .filter(|v| v.is_finite())
        .map_or_else(|| "미입력".to_owned(), |v| format!("{v:.2}"));
    let bottom_text = bottom_elev_m
        .filter(|v| v.is_finite())
        .map_or_else(|| "미입력".to_owned(), |v| format!("{v:.2}"));
    let caption = match depth {
        Some(d) => format!("상면 표고 {top_text}, 바닥 표고 {bottom_text}, 깊이 {d:.2} 미터"),
        None => format!("상면 표고 {top_text}, 바닥 표고 {bottom_text}"),
    };
    (depth, caption)
}
