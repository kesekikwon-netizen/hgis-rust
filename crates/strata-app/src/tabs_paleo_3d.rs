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
    pub declustered_borehole_count: usize,
    pub geology_feature_count: usize,
    pub camera_3d: OrbitCamera3D,
    pub last_paleo_output: Option<PaleoAnalysisResult>,
    pub terrain_patches: Vec<QuadPatch>,
    pub cross_section_rgba: Vec<u8>,
    pub imhof_slope_rgba: Vec<u8>,
    pub terrain3d_rgba: Vec<u8>,
}

impl Default for PaleoTerrain3dTabState {
    fn default() -> Self {
        let mut s = Self {
            radius_km: 10.0,
            source_dl_radius_km: 5.0,
            z_exaggeration: 2.0,
            show_imhof_slope_overlay: true,
            declustered_borehole_count: 12,
            geology_feature_count: 4,
            camera_3d: OrbitCamera3D::default(),
            last_paleo_output: None,
            terrain_patches: Vec::new(),
            cross_section_rgba: Vec::new(),
            imhof_slope_rgba: Vec::new(),
            terrain3d_rgba: Vec::new(),
        };
        s.run_sample_10km_analysis();
        s
    }
}

impl PaleoTerrain3dTabState {
    pub fn run_sample_10km_analysis(&mut self) {
        let tf = GridTransform {
            origin_x: 0.0,
            origin_y: 240.0,
            cell_size: 30.0,
            width: 8,
            height: 8,
        };
        let mut z_curr = Vec::with_capacity(64);
        for r in 0..8 {
            for c in 0..8 {
                let ridge = ((r as f64 - 3.5).powi(2) + (c as f64 - 4.0).powi(2)).sqrt();
                z_curr.push(28.0 + (6.0 - ridge).max(0.0) * 8.5);
            }
        }
        let mut river_mask = vec![0_u8; 64];
        river_mask[0] = 1;
        let aoi_mask = vec![1_u8; 64];
        let raw_boreholes: Vec<BoreholeRawInput> = (0..12)
            .map(|idx| BoreholeRawInput {
                hole_id: format!("BH-{:02}", idx + 1),
                x: 20.0 + (idx as f64) * 35.0,
                y: 40.0 + (idx as f64) * 15.0,
                ground_elev_m: 16.5,
                water_depth_m: 2.8,
                strata: vec![
                    StratumLayer {
                        depth_from_m: 0.0,
                        depth_to_m: 1.2,
                        layer_name: "성토층".to_owned(),
                        uscs_code: "SM".to_owned(),
                        description: "매립".to_owned(),
                        n_value: 6.0,
                    },
                    StratumLayer {
                        depth_from_m: 1.2,
                        depth_to_m: 3.8 + (idx % 3) as f64,
                        layer_name: "충적층".to_owned(),
                        uscs_code: "CL".to_owned(),
                        description: "유기물 포함 암회색 점토".to_owned(),
                        n_value: 4.0,
                    },
                    StratumLayer {
                        depth_from_m: 3.8 + (idx % 3) as f64,
                        depth_to_m: 8.5,
                        layer_name: "풍화토".to_owned(),
                        uscs_code: "SC".to_owned(),
                        description: "기반암 풍화대".to_owned(),
                        n_value: 50.0,
                    },
                ],
            })
            .collect();
        let declustered = decluster_boreholes_by_grid(&raw_boreholes, 30.0, 50);
        self.declustered_borehole_count = declustered.len();
        let (_, imhof_rgba) = compute_imhof_multidirectional_hillshade_and_slope_rgba(&tf, &z_curr);
        self.imhof_slope_rgba = imhof_rgba;
        let out = run_paleo_pipeline(
            &z_curr,
            &tf,
            &declustered,
            &[],
            &[],
            &river_mask,
            &aoi_mask,
            (120.0, 120.0),
        );
        self.cross_section_rgba = render_cross_section_rgba(&out.center_cross_section, 320, 120);
        self.terrain_patches = build_quadtree_patches([120.0, 120.0], 240.0, 2);
        self.last_paleo_output = Some(out);
        self.rebuild_3d_preview();
    }

    pub fn rebuild_3d_preview(&mut self) {
        self.camera_3d.z_exaggeration = self.z_exaggeration;
        if let Some(ref out) = self.last_paleo_output {
            self.terrain3d_rgba =
                render_terrain3d_rgba(&out.z_paleo, 8, 8, &self.camera_3d, 320, 200);
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
