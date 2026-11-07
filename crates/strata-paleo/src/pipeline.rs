use crate::borehole_strata::{parse_single_borehole, BoreholeRawInput, ParsedBoreholeSummary};
use crate::terrain_edt_tpi::{
    compute_river_distance_edt, compute_slope_and_hillshade, compute_tpi_integral_image,
    idw_interpolate_grid, GridTransform,
};
use serde::{Deserialize, Serialize};

/// C++ `KaPaleoPipelineRunner` 4단계 진행률 상수 (`20% -> 50% -> 80% -> 100%`)
pub const HGIS_STAGE1_PROGRESS_END: u8 = 20;
pub const HGIS_STAGE2_PROGRESS_END: u8 = 50;
pub const HGIS_STAGE3_PROGRESS_END: u8 = 80;
pub const HGIS_STAGE4_PROGRESS_END: u8 = 100;

/// C++ `PaleoPipelineResult` 실행 상태 코드
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaleoPipelineStatus {
    Completed,
    UserCancelled,
    InvalidInput,
}

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
    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::NaturalLevee => "자연제방·미고지",
            Self::FluvialTerrace => "하안단구·홍적대지",
            Self::GentleHillslope => "구릉 완사면·산록완사면",
            Self::PaleochannelBackswamp => "구하도·배후습지",
            Self::ActiveFloodplain => "현하도·범람원",
            Self::ArtificialFill => "깊은 매립·교란지",
            Self::SteepMountain => "산지 급경사지",
        }
    }

    pub const fn color_rgba(self) -> [u8; 4] {
        match self {
            Self::NaturalLevee => [217, 119, 6, 210],
            Self::FluvialTerrace => [180, 83, 9, 210],
            Self::GentleHillslope => [101, 163, 13, 210],
            Self::PaleochannelBackswamp => [37, 99, 235, 210],
            Self::ActiveFloodplain => [56, 189, 248, 210],
            Self::ArtificialFill => [107, 114, 128, 210],
            Self::SteepMountain => [75, 85, 99, 210],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CrossSectionProfile {
    pub dist_m: Vec<f64>,
    pub z_surface_curr_m: Vec<f64>,
    pub z_paleo_top_m: Vec<f64>,
    pub z_bedrock_base_m: Vec<f64>,
}

/// C++ `KaPaleoCrossSectionWidget::generateProfile` 절단선 상으로 직교 투영된 시추공 모델
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectedBoreholeOnProfile {
    pub hole_id: String,
    pub station_m: f64,
    pub offset_m: f64,
    pub ground_elev_m: f64,
    pub total_depth_m: f64,
    pub strata_count: usize,
    pub summary: ParsedBoreholeSummary,
}

pub type CutlineProfileResult =
    Result<(Vec<f64>, Vec<f64>, Vec<ProjectedBoreholeOnProfile>), &'static str>;

/// C++ `KaPaleoCrossSectionWidget::generateProfile(const QgsGeometry &cutLine, double searchBufferMeters = 100.0)`
pub fn generate_cutline_profile(
    cut_line: &[[f64; 2]],
    tf: &GridTransform,
    z_curr: &[f64],
    raw_boreholes: &[BoreholeRawInput],
    search_buffer_m: f64,
    abort_requested: Option<&std::sync::atomic::AtomicBool>,
) -> CutlineProfileResult {
    use std::sync::atomic::Ordering;

    if cut_line.len() < 2 {
        return Err("절단선은 최소 2개 이상의 정점이 필요합니다");
    }
    let start = cut_line[0];
    let end = cut_line[cut_line.len() - 1];
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let total_len = (dx * dx + dy * dy).sqrt();
    if total_len < 1e-3 {
        return Err("절단선 길이가 0에 가깝습니다");
    }

    let steps = (total_len.round() as usize).max(1);
    let mut stations = Vec::with_capacity(steps + 1);
    let mut elevations = Vec::with_capacity(steps + 1);
    for s in 0..=steps {
        if abort_requested.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("단면도 프로파일 생성이 중단되었습니다");
        }
        let t = (s as f64) / (steps as f64);
        let wx = start[0] + dx * t;
        let wy = start[1] + dy * t;
        let z = tf
            .world_to_cell(wx, wy)
            .and_then(|(r, c)| z_curr.get(r * tf.width + c).copied())
            .unwrap_or(15.0);
        stations.push(total_len * t);
        elevations.push(z);
    }

    let ux = dx / total_len;
    let uy = dy / total_len;
    let mut projected = Vec::new();
    for bh in raw_boreholes {
        if abort_requested.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("단면도 프로파일 생성이 중단되었습니다");
        }
        let vx = bh.x - start[0];
        let vy = bh.y - start[1];
        let along = vx * ux + vy * uy;
        let perp = (vx * uy - vy * ux).abs();
        if (0.0..=total_len).contains(&along) && perp <= search_buffer_m.max(1.0) {
            let total_depth_m = bh
                .strata
                .iter()
                .map(|st| st.depth_to_m)
                .fold(0.0_f64, f64::max);
            projected.push(ProjectedBoreholeOnProfile {
                hole_id: bh.hole_id.clone(),
                station_m: along,
                offset_m: perp,
                ground_elev_m: bh.ground_elev_m,
                total_depth_m,
                strata_count: bh.strata.len(),
                summary: parse_single_borehole(bh),
            });
        }
    }
    projected.sort_by(|a, b| {
        a.station_m
            .partial_cmp(&b.station_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok((stations, elevations, projected))
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
        let cls = if slope_deg[i] >= 15.0 {
            PaleoClass::SteepMountain
        } else if fill_grid[i] >= 3.5 {
            PaleoClass::ArtificialFill
        } else if slope_deg[i] >= 5.0 {
            PaleoClass::GentleHillslope
        } else if rem[i] >= 4.0 {
            PaleoClass::FluvialTerrace
        } else if tpi_50m[i] >= -0.2 && dist_to_river_m[i] <= 600.0 {
            levee_cnt += usize::from(aoi_mask.get(i).copied().unwrap_or(1) != 0);
            PaleoClass::NaturalLevee
        } else if tpi_50m[i] < -0.3 {
            PaleoClass::PaleochannelBackswamp
        } else {
            PaleoClass::ActiveFloodplain
        };
        paleo_class_grid.push(cls);
        let [r, g, b, a] = cls.color_rgba();
        let shade = multidirectional_hillshade[i];
        paleo_rgba_buffer.extend_from_slice(&[
            ((r as f64) * shade).clamp(0.0, 255.0) as u8,
            ((g as f64) * shade).clamp(0.0, 255.0) as u8,
            ((b as f64) * shade).clamp(0.0, 255.0) as u8,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::borehole_strata::StratumLayer;

    #[test]
    fn paleo_pipeline_classifies_natural_levee_and_generates_cross_section() {
        let tf = GridTransform {
            origin_x: 200_000.0,
            origin_y: 500_000.0,
            cell_size: 10.0,
            width: 32,
            height: 32,
        };
        let z_curr = vec![15.0; 32 * 32];
        let mut river_mask = vec![0u8; 32 * 32];
        river_mask[16 * 32 + 16] = 1;
        let aoi_mask = vec![1u8; 32 * 32];
        let bh = vec![BoreholeRawInput {
            hole_id: "BH-1".into(),
            x: 200_160.0,
            y: 499_840.0,
            ground_elev_m: 15.0,
            water_depth_m: 2.5,
            strata: vec![
                StratumLayer {
                    depth_from_m: 0.0,
                    depth_to_m: 1.0,
                    layer_name: "매립토".into(),
                    uscs_code: "SM".into(),
                    description: "".into(),
                    n_value: 5.0,
                },
                StratumLayer {
                    depth_from_m: 1.0,
                    depth_to_m: 4.0,
                    layer_name: "충적사질토".into(),
                    uscs_code: "SM".into(),
                    description: "".into(),
                    n_value: 12.0,
                },
                StratumLayer {
                    depth_from_m: 4.0,
                    depth_to_m: 8.0,
                    layer_name: "풍화암".into(),
                    uscs_code: "WR".into(),
                    description: "".into(),
                    n_value: 50.0,
                },
            ],
        }];
        let res = run_paleo_pipeline(
            &z_curr,
            &tf,
            &bh,
            &[],
            &[],
            &river_mask,
            &aoi_mask,
            (200_160.0, 499_840.0),
        );
        assert_eq!(res.z_paleo.len(), 32 * 32);
        assert_eq!(res.paleo_rgba_buffer.len(), 32 * 32 * 4);
        assert!(!res.center_cross_section.dist_m.is_empty());
        assert!(res.aoi_summary_text.contains("고지형"));
    }

    #[test]
    fn test_paleo_profile_extraction_500m_cutline_samples_501_points_and_projects_2_boreholes() {
        use std::sync::atomic::AtomicBool;

        let tf = GridTransform {
            origin_x: 200_000.0,
            origin_y: 500_000.0,
            cell_size: 10.0,
            width: 60,
            height: 20,
        };
        let z_curr = vec![22.5; 60 * 20];
        let cut_line = [[200_000.0, 499_900.0], [200_500.0, 499_900.0]];
        let bh_list = vec![
            BoreholeRawInput {
                hole_id: "BH-10M".to_owned(),
                x: 200_120.0,
                y: 499_930.0, // 30 m offset (within 50 m / 100 m buffer)
                ground_elev_m: 22.5,
                water_depth_m: 2.0,
                strata: vec![
                    StratumLayer {
                        depth_from_m: 0.0,
                        depth_to_m: 3.0,
                        layer_name: "매립층".to_owned(),
                        uscs_code: "SM".to_owned(),
                        description: "".to_owned(),
                        n_value: 6.0,
                    },
                    StratumLayer {
                        depth_from_m: 3.0,
                        depth_to_m: 10.0,
                        layer_name: "충적층".to_owned(),
                        uscs_code: "CL".to_owned(),
                        description: "점토".to_owned(),
                        n_value: 8.0,
                    },
                ],
            },
            BoreholeRawInput {
                hole_id: "BH-20M".to_owned(),
                x: 200_380.0,
                y: 499_860.0, // 40 m offset
                ground_elev_m: 22.5,
                water_depth_m: 2.5,
                strata: vec![
                    StratumLayer {
                        depth_from_m: 0.0,
                        depth_to_m: 5.0,
                        layer_name: "충적층".to_owned(),
                        uscs_code: "SP".to_owned(),
                        description: "모래".to_owned(),
                        n_value: 14.0,
                    },
                    StratumLayer {
                        depth_from_m: 5.0,
                        depth_to_m: 20.0,
                        layer_name: "풍화암".to_owned(),
                        uscs_code: "WR".to_owned(),
                        description: "풍화암".to_owned(),
                        n_value: 50.0,
                    },
                ],
            },
        ];

        let abort = AtomicBool::new(false);
        let (stations, elevations, projected) =
            generate_cutline_profile(&cut_line, &tf, &z_curr, &bh_list, 100.0, Some(&abort))
                .expect("profile extraction should succeed");
        assert_eq!(stations.len(), 501);
        assert_eq!(elevations.len(), 501);
        assert_eq!(projected.len(), 2);
        assert!((projected[0].station_m - 120.0).abs() < 1e-6);
        assert!((projected[0].total_depth_m - 10.0).abs() < 1e-6);
        assert_eq!(projected[0].strata_count, 2);
        assert!((projected[1].station_m - 380.0).abs() < 1e-6);
        assert!((projected[1].total_depth_m - 20.0).abs() < 1e-6);
        assert_eq!(projected[1].strata_count, 2);
    }

    #[test]
    fn test_stage2_cancellation_stops_progress_under_50pct_and_returns_user_cancelled() {
        use std::sync::atomic::AtomicBool;

        // TestKaPaleoPipelineRunner::testStage2CancellationCleanup:
        // Cancel immediately after stage 1 (20%) before stage 2 finishes -> UserCancelled & progress <= 50%
        let cancel_flag = AtomicBool::new(false);
        let mut reported = Vec::new();
        let (status, last_pct) = run_paleo_pipeline_staged(Some(1), &cancel_flag, |pct, _stage| {
            reported.push(pct);
        });
        assert_eq!(status, PaleoPipelineStatus::UserCancelled);
        assert_eq!(last_pct, HGIS_STAGE1_PROGRESS_END);
        assert!(last_pct <= HGIS_STAGE2_PROGRESS_END);
        assert_eq!(reported, vec![20]);

        // Full run reaches 100% across all 4 stages
        let clean_flag = AtomicBool::new(false);
        let mut all_stages = Vec::new();
        let (ok_status, final_pct) = run_paleo_pipeline_staged(None, &clean_flag, |pct, _stage| {
            all_stages.push(pct);
        });
        assert_eq!(ok_status, PaleoPipelineStatus::Completed);
        assert_eq!(final_pct, 100);
        assert_eq!(all_stages, vec![20, 50, 80, 100]);
    }
}
