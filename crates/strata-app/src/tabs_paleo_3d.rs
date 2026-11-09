use strata_paleo::borehole_strata::{decluster_boreholes_by_grid, BoreholeRawInput, StratumLayer};
use strata_paleo::pipeline::{run_paleo_pipeline, PaleoAnalysisResult};
use strata_paleo::section_renderer::render_cross_section_rgba;
use strata_paleo::terrain_edt_tpi::{
    compute_imhof_multidirectional_hillshade_and_slope_rgba, GridTransform,
};
use strata_terrain3d::quadtree_lod::{build_quadtree_patches, QuadPatch};
use strata_terrain3d::viewport3d_renderer::{render_terrain3d_rgba, OrbitCamera3D};

#[derive(Debug, Clone)]
pub struct PaleoTerrain3dTabState {
    pub radius_km: f64,
    pub source_dl_radius_km: f64,
    pub z_exaggeration: f32,
    pub show_imhof_slope_overlay: bool,
    /// KaTerrain3dStudio.cpp:190-248 — 3D 표면에 입히는 그림(0 위성, 1 음영기복, 2 고지형)
    pub texture_mode_idx: usize,
    pub declustered_borehole_count: usize,
    pub geology_feature_count: usize,
    pub camera_3d: OrbitCamera3D,
    pub last_paleo_output: Option<PaleoAnalysisResult>,
    pub terrain_patches: Vec<QuadPatch>,
    pub cross_section_rgba: Vec<u8>,
    pub imhof_slope_rgba: Vec<u8>,
    pub terrain3d_rgba: Vec<u8>,
    /// C++ `GeoInfoBoreholeMaps.cpp:179-294` 로 마지막에 만든 시추 도면들.
    /// 파일만 만들고 끝내면 지도에 색이 안 칠해지므로, 화면에 그릴 격자와 램프를 함께 들고 있다.
    pub last_borehole_maps: Option<strata_paleo::borehole_maps::BoreholeMapBundle>,
    /// 지금 화면에 칠하고 있는 시추 도면 번호(0부터). `None` 이면 칠하지 않는다.
    pub shown_borehole_map_idx: Option<usize>,
}

impl Default for PaleoTerrain3dTabState {
    fn default() -> Self {
        let mut s = Self {
            radius_km: 10.0,
            source_dl_radius_km: 5.0,
            z_exaggeration: 2.0,
            show_imhof_slope_overlay: true,
            texture_mode_idx: 1,
            declustered_borehole_count: 12,
            geology_feature_count: 4,
            camera_3d: OrbitCamera3D::default(),
            last_paleo_output: None,
            terrain_patches: Vec::new(),
            cross_section_rgba: Vec::new(),
            imhof_slope_rgba: Vec::new(),
            terrain3d_rgba: Vec::new(),
            last_borehole_maps: None,
            shown_borehole_map_idx: None,
        };
        s.run_sample_10km_analysis();
        s
    }
}

impl PaleoTerrain3dTabState {
    pub fn run_sample_10km_analysis(&mut self) {
        let width = 120usize;
        let height = 84usize;
        let cell_size = 18.0_f64;
        let tf = GridTransform {
            origin_x: 218_600.0,
            origin_y: 361_680.0,
            cell_size,
            width,
            height,
        };
        let n = width * height;
        let mut z_curr = Vec::with_capacity(n);
        let mut river_mask = vec![0_u8; n];
        let aoi_mask = vec![1_u8; n];

        for r in 0..height {
            let v = (r as f64 + 0.5) / (height as f64);
            for c in 0..width {
                let u = (c as f64 + 0.5) / (width as f64);
                // 주 하천 유로 (서남서 -> 동북동 사행 유로)
                let v_riv = 0.50 + 0.08 * (u * 6.0 - 0.5).sin() - 0.04 * (u - 0.5);
                let d_riv_norm = (v - v_riv).abs();
                // 남측 우각호·구하도 분기 유로
                let v_oxbow = 0.65 + 0.06 * ((u - 0.24) * 8.2).sin();
                let in_oxbow_span = (0.20..=0.74).contains(&u);
                let d_oxbow_norm = if in_oxbow_span {
                    (v - v_oxbow).abs()
                } else {
                    1.0
                };

                // 기본 하상 경사 표고 (서쪽 22.0m -> 동쪽 24.4m)
                let base_river_z = 22.0 + 2.4 * u;

                // 1) 구하도 및 현하도 저수로 (청색)
                let channel_dip = if d_riv_norm < 0.026 {
                    river_mask[r * width + c] = 1;
                    -0.35
                } else if d_oxbow_norm < 0.026 {
                    river_mask[r * width + c] = 1;
                    -0.25
                } else {
                    0.0
                };

                // 2) 자연제방 미고지 (주황색: 하도 양안의 완만한 사질 미고지 띠)
                let levee_bump = if (0.026..=0.105).contains(&d_riv_norm) {
                    let t = 1.0 - ((d_riv_norm - 0.062).abs() / 0.045).clamp(0.0, 1.0);
                    2.45 * t
                } else if in_oxbow_span && (0.026..=0.075).contains(&d_oxbow_norm) {
                    let t = 1.0 - ((d_oxbow_norm - 0.048).abs() / 0.028).clamp(0.0, 1.0);
                    1.85 * t
                } else {
                    0.0
                };

                // 3) 배후습지 저지대 (청록색: 자연제방 배후의 폐쇄형 미저지)
                let swamp_north =
                    ((u - 0.34).powi(2) / (0.19 * 0.19)) + ((v - 0.31).powi(2) / (0.075 * 0.075));
                let swamp_south =
                    ((u - 0.60).powi(2) / (0.17 * 0.17)) + ((v - 0.72).powi(2) / (0.068 * 0.068));
                let backswamp_dip = if swamp_north < 1.0 {
                    -0.85 * (1.0 - swamp_north)
                } else if swamp_south < 1.0 {
                    -0.80 * (1.0 - swamp_south)
                } else {
                    0.0
                };

                // 4) 선상지·하안단구 완경사면 (황색: 북동·남서 곡구에서 퍼져 나오는 선상지)
                let fan_ne =
                    ((u - 0.72).powi(2) / (0.20 * 0.20)) + ((v - 0.24).powi(2) / (0.13 * 0.13));
                let fan_sw =
                    ((u - 0.22).powi(2) / (0.18 * 0.18)) + ((v - 0.77).powi(2) / (0.12 * 0.12));
                let fan_rise = if fan_ne < 1.0 {
                    8.6 * (1.0 - fan_ne).powf(1.15)
                } else if fan_sw < 1.0 {
                    7.8 * (1.0 - fan_sw).powf(1.15)
                } else {
                    0.0
                };

                // 5) 범람원 일반 평탄면 기본 비고 (+2.5m)
                let floodplain_base = if channel_dip < 0.0 {
                    0.3
                } else if backswamp_dip < -0.15 {
                    1.45 + backswamp_dip
                } else {
                    2.55
                };

                // 6) 구릉사면 (갈색: 북측·남측 산록 완사면, 경사 6~12°) 및 7) 산지 (회색: 외곽 급경사 산지, 경사 14°+)
                let north_margin = (0.24 - v).max(0.0) / 0.24;
                let south_margin = (v - 0.78).max(0.0) / 0.22;
                let east_ridge =
                    ((u - 0.94).max(0.0) / 0.06) * (1.0 - (v - 0.5).abs() * 1.4).max(0.0);
                let hill_mtn_rise = 28.0 * north_margin.powf(1.85)
                    + 26.0 * south_margin.powf(1.85)
                    + 18.0 * east_ridge.powf(1.6);

                let z = base_river_z
                    + floodplain_base
                    + channel_dip
                    + levee_bump
                    + fan_rise
                    + hill_mtn_rise;
                z_curr.push(z);
            }
        }

        // A-A' 단면선(중간 행 r = height / 2)을 따라 배치된 8개 대표 시추공 (BH-01 ~ BH-08)
        let mid_y = tf.origin_y - (height as f64 * 0.5) * cell_size;
        let raw_boreholes: Vec<BoreholeRawInput> = (0..8)
            .map(|idx| {
                let frac = (idx as f64 + 0.6) / 8.2;
                let bx = tf.origin_x + frac * (width as f64 * cell_size);
                let col = ((frac * width as f64) as usize).min(width - 1);
                let ground_z = z_curr[(height / 2) * width + col];
                let fill_m = 0.8 + (idx % 3) as f64 * 0.35;
                let alluv_m = match idx {
                    1 | 2 | 5 => 3.8 + (idx % 2) as f64 * 0.9, // 자연제방·선상지 두꺼운 사질 충적층
                    3 | 4 => 4.6,                              // 구하도·배후습지 유기질 점토층
                    _ => 2.6,
                };
                let is_coarse = matches!(idx, 1 | 2 | 5 | 6);
                let uscs = if is_coarse { "SM" } else { "CL" };
                let desc = if is_coarse {
                    "황갈색 세립~중립 사질토 (자연제방·문화층 유망)"
                } else {
                    "암회색 유기물 혼입 실트질 점토 (구하도·수전 유망)"
                };
                BoreholeRawInput {
                    hole_id: format!("BH-{:02}", idx + 1),
                    x: bx,
                    y: mid_y + ((idx as f64 - 3.5) * 18.0),
                    ground_elev_m: ground_z,
                    water_depth_m: 2.1 + (idx % 3) as f64 * 0.45,
                    strata: vec![
                        StratumLayer {
                            depth_from_m: 0.0,
                            depth_to_m: fill_m,
                            layer_name: "성토층".to_owned(),
                            uscs_code: "SM".to_owned(),
                            description: "현대 매립·경작토".to_owned(),
                            n_value: 6.0,
                        },
                        StratumLayer {
                            depth_from_m: fill_m,
                            depth_to_m: fill_m + alluv_m,
                            layer_name: "충적층".to_owned(),
                            uscs_code: uscs.to_owned(),
                            description: desc.to_owned(),
                            n_value: if is_coarse { 12.0 } else { 5.0 },
                        },
                        StratumLayer {
                            depth_from_m: fill_m + alluv_m,
                            depth_to_m: fill_m + alluv_m + 4.2,
                            layer_name: "풍화토".to_owned(),
                            uscs_code: "SC".to_owned(),
                            description: "기반암 풍화대 (생토면)".to_owned(),
                            n_value: 50.0,
                        },
                    ],
                }
            })
            .collect();
        let declustered = decluster_boreholes_by_grid(&raw_boreholes, 30.0, 50);
        self.declustered_borehole_count = declustered.len();
        let (_, imhof_rgba) = compute_imhof_multidirectional_hillshade_and_slope_rgba(&tf, &z_curr);
        self.imhof_slope_rgba = imhof_rgba;
        let center_xy = (
            tf.origin_x + (width as f64 * cell_size) * 0.5,
            tf.origin_y - (height as f64 * cell_size) * 0.5,
        );
        let out = run_paleo_pipeline(
            &z_curr,
            &tf,
            &declustered,
            &[],
            &[],
            &river_mask,
            &aoi_mask,
            center_xy,
        );
        self.cross_section_rgba = render_cross_section_rgba(&out.center_cross_section, 320, 120);
        self.terrain_patches =
            build_quadtree_patches([center_xy.0, center_xy.1], width as f64 * cell_size, 2);
        self.last_paleo_output = Some(out);
        self.rebuild_3d_preview();
    }

    pub fn rebuild_3d_preview(&mut self) {
        self.camera_3d.z_exaggeration = self.z_exaggeration;
        if let Some(ref out) = self.last_paleo_output {
            self.terrain3d_rgba = render_terrain3d_rgba(
                &out.z_paleo,
                out.transform.width,
                out.transform.height,
                &self.camera_3d,
                320,
                200,
            );
        }
    }

    pub const HGIS_MIN_CUTLINE_LENGTH_M: f64 = 5.0;
    pub const HGIS_MAX_SECTION_SAMPLES: usize = 500;
    pub const HGIS_CORRIDOR_HALF_WIDTH_MIN: f64 = 50.0;
    pub const HGIS_CORRIDOR_HALF_WIDTH_MAX: f64 = 500.0;

    /// C++ `KaPaleoStudio::generateCrossSection` — 단면 절단선 최소 길이 5.0m 방어, 최대 샘플 개수 500개 제한, 시추공 투영 복도 반폭 `[50.0m, 500.0m]` 클램핑 1:1
    #[allow(clippy::type_complexity)]
    pub fn generate_cross_section_samples(
        cut_line: &[[f64; 2]],
        search_corridor_half_width: f64,
        max_sample_count: usize,
    ) -> Result<(Vec<[f64; 2]>, f64), (PaleoSectionError, String)> {
        if cut_line.len() < 2 {
            return Err((
                PaleoSectionError::CutLineTooShort,
                "단면 절단선 길이가 너무 짧습니다 (최소 5.0m 이상 작도 필요, 현재: 0.00 m)"
                    .to_owned(),
            ));
        }
        let mut total_len = 0.0_f64;
        for pair in cut_line.windows(2) {
            total_len += (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
        }
        if total_len < Self::HGIS_MIN_CUTLINE_LENGTH_M {
            return Err((
                PaleoSectionError::CutLineTooShort,
                format!(
                    "단면 절단선 길이가 너무 짧습니다 (최소 5.0m 이상 작도 필요, 현재: {total_len:.2} m)"
                ),
            ));
        }
        let corridor = search_corridor_half_width.clamp(
            Self::HGIS_CORRIDOR_HALF_WIDTH_MIN,
            Self::HGIS_CORRIDOR_HALF_WIDTH_MAX,
        );
        let cap = max_sample_count.clamp(2, Self::HGIS_MAX_SECTION_SAMPLES);
        let raw_samples = (total_len.floor() as usize + 1).min(cap);
        let start = cut_line[0];
        let end = cut_line[cut_line.len() - 1];
        let mut pts = Vec::with_capacity(raw_samples);
        for idx in 0..raw_samples {
            let t = idx as f64 / (raw_samples.saturating_sub(1).max(1) as f64);
            pts.push([
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ]);
        }
        Ok((pts, corridor))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaleoSectionError {
    CutLineTooShort,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paleo_studio_cutline_under_5m_rejected_and_1000m_clamped_to_500_samples() {
        let short_line = [[100.0, 100.0], [101.5, 101.3228756]];
        let err = PaleoTerrain3dTabState::generate_cross_section_samples(&short_line, 100.0, 500)
            .expect_err("2m line must fail");
        assert_eq!(err.0, PaleoSectionError::CutLineTooShort);
        assert!(err.1.contains("최소 5.0m"));

        let long_line = [[0.0, 0.0], [1000.0, 0.0]];
        let (samples, corridor) =
            PaleoTerrain3dTabState::generate_cross_section_samples(&long_line, 800.0, 500)
                .expect("1000m line must succeed");
        assert_eq!(samples.len(), 500);
        assert!((corridor - 500.0).abs() < 1e-6);
    }
}
