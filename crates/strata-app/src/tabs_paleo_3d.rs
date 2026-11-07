use strata_paleo::borehole_strata::BoreholeRawInput;
use strata_paleo::pipeline::{run_paleo_pipeline, PaleoAnalysisResult};
use strata_paleo::section_renderer::render_cross_section_rgba;
use strata_paleo::terrain_edt_tpi::GridTransform;
use strata_terrain3d::quadtree_lod::{build_quadtree_patches, QuadPatch};
use strata_terrain3d::viewport3d_renderer::{render_terrain3d_rgba, OrbitCamera3D};

#[derive(Debug, Clone)]
pub struct PaleoTerrain3dTabState {
    pub radius_km: f64,
    pub z_exaggeration: f32,
    pub camera_3d: OrbitCamera3D,
    pub last_paleo_output: Option<PaleoAnalysisResult>,
    pub terrain_patches: Vec<QuadPatch>,
    pub cross_section_rgba: Vec<u8>,
    pub terrain3d_rgba: Vec<u8>,
}

impl Default for PaleoTerrain3dTabState {
    fn default() -> Self {
        let mut s = Self {
            radius_km: 10.0,
            z_exaggeration: 2.0,
            camera_3d: OrbitCamera3D::default(),
            last_paleo_output: None,
            terrain_patches: Vec::new(),
            cross_section_rgba: Vec::new(),
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
        let raw_boreholes: Vec<BoreholeRawInput> = Vec::new();
        let out = run_paleo_pipeline(
            &z_curr,
            &tf,
            &raw_boreholes,
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
}
