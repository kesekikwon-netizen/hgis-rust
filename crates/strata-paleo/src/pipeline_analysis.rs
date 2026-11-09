//! 퇴적상 분류와 4단계 파이프라인 실행.
//!
//! `pipeline` 에서 분리했다.

use serde::{Deserialize, Serialize};

use crate::borehole_strata::{parse_single_borehole, BoreholeRawInput, ParsedBoreholeSummary};
use crate::terrain_edt_tpi::{
    compute_river_distance_edt, compute_slope_and_hillshade, compute_tpi_integral_image,
    idw_interpolate_grid, GridTransform,
};

use crate::pipeline_cutline::CrossSectionProfile;
use crate::pipeline_stage::{
    PaleoPipelineStatus, HGIS_STAGE1_PROGRESS_END, HGIS_STAGE2_PROGRESS_END,
    HGIS_STAGE3_PROGRESS_END, HGIS_STAGE4_PROGRESS_END,
};

/// C++ `KaPaleoPipelineRunner::runPipeline` — 4단계 진행률 콜백(`20% -> 50% -> 80% -> 100%`) 및 단계별 취소 원자 플래그 검사
pub fn run_paleo_pipeline_staged<F>(
    cancel_after_stage: Option<u8>,
    is_cancelled: &std::sync::atomic::AtomicBool,
    mut on_progress: F,
) -> (PaleoPipelineStatus, u8)
where
    F: FnMut(u8, &'static str),
{
    use std::sync::atomic::Ordering;

    let stages: [(u8, u8, &'static str); 4] = [
        (1, HGIS_STAGE1_PROGRESS_END, "DEM 로드 및 결측 보정"),
        (
            2,
            HGIS_STAGE2_PROGRESS_END,
            "하천망 추출 및 유클리드 거리장(EDT) 계산",
        ),
        (
            3,
            HGIS_STAGE3_PROGRESS_END,
            "지형위치지수(TPI) 및 비고(REM) 지형 분류",
        ),
        (
            4,
            HGIS_STAGE4_PROGRESS_END,
            "고고학 입지 적합도 다기준 가중치 스코어링",
        ),
    ];

    let mut last_progress = 0u8;
    for (stage_idx, pct, label) in stages {
        if is_cancelled.load(Ordering::Relaxed) {
            return (PaleoPipelineStatus::UserCancelled, last_progress);
        }
        on_progress(pct, label);
        last_progress = pct;
        if cancel_after_stage == Some(stage_idx) {
            is_cancelled.store(true, Ordering::Relaxed);
        }
    }
    if is_cancelled.load(Ordering::Relaxed) && last_progress < HGIS_STAGE4_PROGRESS_END {
        return (PaleoPipelineStatus::UserCancelled, last_progress);
    }
    (PaleoPipelineStatus::Completed, last_progress)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaleoClass {
    NaturalLevee = 1,
    FluvialTerrace = 2,
    GentleHillslope = 3,
    PaleochannelBackswamp = 4,
    ActiveFloodplain = 5,
    ArtificialFill = 6,
    SteepMountain = 7,
}

impl PaleoClass {
    pub const ALL: [Self; 7] = [
        Self::NaturalLevee,
        Self::PaleochannelBackswamp,
        Self::ArtificialFill,
        Self::FluvialTerrace,
        Self::ActiveFloodplain,
        Self::GentleHillslope,
        Self::SteepMountain,
    ];

    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::NaturalLevee => "자연제방",
            Self::PaleochannelBackswamp => "구하도",
            Self::ArtificialFill => "배후습지",
            Self::FluvialTerrace => "선상지",
            Self::ActiveFloodplain => "범람원",
            Self::GentleHillslope => "구릉사면",
            Self::SteepMountain => "산지",
        }
    }

    pub const fn arch_potential_ko(self) -> &'static str {
        match self {
            Self::NaturalLevee => "유적 입지 최우선 고잠재력 (취락·생산)",
            Self::PaleochannelBackswamp => "유기물·목기·수리시설 매장이상대",
            Self::ArtificialFill => "저습지·고대 수전(논) 유구 유망",
            Self::FluvialTerrace => "선사·고대 생활유구 완경사 미고지",
            Self::ActiveFloodplain => "충적층 심도별 다층 문화층 부존 가능",
            Self::GentleHillslope => "분묘·주거지 사면 말단부 입지",
            Self::SteepMountain => "산성·관방·봉수·제사 유적 입지",
        }
    }

    pub const fn color_rgba(self) -> [u8; 4] {
        match self {
            Self::NaturalLevee => [217, 119, 6, 240],
            Self::PaleochannelBackswamp => [37, 99, 235, 240],
            Self::ArtificialFill => [16, 185, 129, 240],
            Self::FluvialTerrace => [234, 179, 8, 240],
            Self::ActiveFloodplain => [132, 204, 22, 240],
            Self::GentleHillslope => [180, 83, 9, 240],
            Self::SteepMountain => [120, 113, 108, 240],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaleoAnalysisResult {
    pub transform: GridTransform,
    pub z_paleo: Vec<f64>,
    pub rem: Vec<f64>,
    pub slope_deg: Vec<f64>,
    pub tpi_50m: Vec<f64>,
    pub dist_to_river_m: Vec<f64>,
    pub multidirectional_hillshade: Vec<f64>,
    pub paleo_class_grid: Vec<PaleoClass>,
    pub paleo_rgba_buffer: Vec<u8>,
    pub parsed_boreholes: Vec<ParsedBoreholeSummary>,
    pub center_cross_section: CrossSectionProfile,
    pub aoi_summary_text: String,
}

#[allow(clippy::too_many_arguments)]
pub fn run_paleo_pipeline(
    z_curr: &[f64],
    tf: &GridTransform,
    raw_boreholes: &[BoreholeRawInput],
    _soil_grid: &[u8],
    _geol_grid: &[u8],
    river_mask: &[u8],
    aoi_mask: &[u8],
    _aoi_center_xy: (f64, f64),
) -> PaleoAnalysisResult {
    let n = tf.width * tf.height;
    let parsed_boreholes: Vec<ParsedBoreholeSummary> =
        raw_boreholes.iter().map(parse_single_borehole).collect();

    let fill_pts: Vec<[f64; 3]> = parsed_boreholes
        .iter()
        .map(|b| [b.x, b.y, b.fill_thickness_m])
        .collect();
    let base_pts: Vec<[f64; 3]> = parsed_boreholes
        .iter()
        .map(|b| [b.x, b.y, b.weathered_top_depth_m])
        .collect();

    let fill_grid = idw_interpolate_grid(tf, &fill_pts, 2.0, 0.5);
    let base_grid = idw_interpolate_grid(tf, &base_pts, 2.0, 4.0);
    let z_paleo: Vec<f64> = z_curr
        .iter()
        .zip(fill_grid.iter())
        .map(|(&z, &f)| z - f)
        .collect();

    let dist_to_river_m = compute_river_distance_edt(tf, river_mask);
    let tpi_50m = compute_tpi_integral_image(tf, &z_paleo, 50.0);
    let (slope_deg, multidirectional_hillshade) = compute_slope_and_hillshade(tf, &z_paleo);

    let min_z = z_paleo.iter().copied().fold(f64::INFINITY, f64::min);
    let rem: Vec<f64> = z_paleo.iter().map(|&z| (z - min_z).max(0.0)).collect();

    let mut paleo_class_grid = Vec::with_capacity(n);
    let mut paleo_rgba_buffer = Vec::with_capacity(n * 4);
    let mut levee_cnt = 0usize;

    for i in 0..n {
        let s = slope_deg[i];
        let r_m = rem[i];
        let tpi = tpi_50m[i];
        let d_riv = dist_to_river_m[i];
        let is_river_cell = river_mask.get(i).copied().unwrap_or(0) != 0;

        let cls = if s >= 13.0 || r_m >= 26.0 {
            PaleoClass::SteepMountain
        } else if s >= 5.2 || r_m >= 12.5 {
            PaleoClass::GentleHillslope
        } else if (s >= 1.9 && (3.6..12.5).contains(&r_m) && tpi >= -0.25)
            || (4.8..12.5).contains(&r_m)
        {
            PaleoClass::FluvialTerrace
        } else if is_river_cell || d_riv <= 42.0 || (r_m < 1.35 && tpi < -0.16) {
            PaleoClass::PaleochannelBackswamp
        } else if (d_riv <= 220.0 && (1.05..4.2).contains(&r_m) && tpi >= 0.04)
            || ((1.35..3.8).contains(&r_m) && tpi >= 0.14)
        {
            levee_cnt += usize::from(aoi_mask.get(i).copied().unwrap_or(1) != 0);
            PaleoClass::NaturalLevee
        } else if (r_m < 2.4 && tpi < -0.025) || fill_grid[i] >= 3.5 {
            PaleoClass::ArtificialFill
        } else if d_riv <= 500.0 && tpi >= 0.0 && r_m < 1.05 {
            levee_cnt += usize::from(aoi_mask.get(i).copied().unwrap_or(1) != 0);
            PaleoClass::NaturalLevee
        } else {
            PaleoClass::ActiveFloodplain
        };
        paleo_class_grid.push(cls);
    }

    if tf.width >= 16 && tf.height >= 16 {
        let mut smoothed = paleo_class_grid.clone();
        for r in 0..tf.height {
            for c in 0..tf.width {
                let mut counts = [0u16; 8];
                for dr in -1isize..=1 {
                    let rr = (r as isize + dr).clamp(0, tf.height as isize - 1) as usize;
                    for dc in -1isize..=1 {
                        let cc = (c as isize + dc).clamp(0, tf.width as isize - 1) as usize;
                        let code = paleo_class_grid[rr * tf.width + cc] as usize;
                        if code < counts.len() {
                            counts[code] += if dr == 0 && dc == 0 { 2 } else { 1 };
                        }
                    }
                }
                let mut best = paleo_class_grid[r * tf.width + c];
                let mut best_cnt = counts[best as usize];
                for cand in PaleoClass::ALL {
                    let cnt = counts[cand as usize];
                    if cnt > best_cnt {
                        best_cnt = cnt;
                        best = cand;
                    }
                }
                smoothed[r * tf.width + c] = best;
            }
        }
        paleo_class_grid = smoothed;
    }

    for i in 0..n {
        let cls = paleo_class_grid[i];
        let [r, g, b, a] = cls.color_rgba();
        let shade = multidirectional_hillshade[i].clamp(0.72, 1.06);
        paleo_rgba_buffer.extend_from_slice(&[
            ((r as f64) * shade).round().clamp(0.0, 255.0) as u8,
            ((g as f64) * shade).round().clamp(0.0, 255.0) as u8,
            ((b as f64) * shade).round().clamp(0.0, 255.0) as u8,
            a,
        ]);
    }

    let mid_row = tf.height / 2;
    let mut profile = CrossSectionProfile::default();
    for c in 0..tf.width {
        let idx = mid_row * tf.width + c;
        profile.dist_m.push((c as f64) * tf.cell_size);
        profile.z_surface_curr_m.push(z_curr[idx]);
        profile.z_paleo_top_m.push(z_paleo[idx]);
        profile.z_bedrock_base_m.push(z_curr[idx] - base_grid[idx]);
    }

    let aoi_summary_text = format!(
        "고지형 4단계 정밀 판독 요약: 시추공 {}공 분석 완료, 자연제방·미고지 격자 {}셀 확인.",
        parsed_boreholes.len(),
        levee_cnt
    );

    PaleoAnalysisResult {
        transform: *tf,
        z_paleo,
        rem,
        slope_deg,
        tpi_50m,
        dist_to_river_m,
        multidirectional_hillshade,
        paleo_class_grid,
        paleo_rgba_buffer,
        parsed_boreholes,
        center_cross_section: profile,
        aoi_summary_text,
    }
}
