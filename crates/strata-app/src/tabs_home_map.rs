use strata_canvas2d::cad_interaction::CadSessionController;
use strata_canvas2d::canvas_painter::MapSceneModel;
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;
use strata_net::address_search::sido_presets;
use strata_net::tile_fetcher::BasemapSource;

#[derive(Debug, Clone)]
pub struct HomeMapTabState {
    pub recent_files: Vec<String>,
    pub recent_filter: String,
    pub internet_checked: bool,
    pub files_open: bool,
    pub layer_filter: String,
    pub selected_feature_idx: Option<usize>,
    pub grid_enabled: bool,
    pub cursor_world_xy: [f64; 2],
    pub viewport: ViewportTransform,
    pub draw_guide_message: Option<String>,
    pub scene: MapSceneModel,
    pub selected_sido_idx: usize,
    pub cad: CadSessionController,
    pub active_basemap: BasemapSource,
}

impl Default for HomeMapTabState {
    fn default() -> Self {
        Self {
            recent_files: vec![
                "경주_월성북편_시굴조사.gpkg".to_owned(),
                "부여_관북리_지표조사.gpkg".to_owned(),
                "익산_왕궁리_정밀발굴조사.gpkg".to_owned(),
            ],
            recent_filter: String::new(),
            internet_checked: false,
            files_open: true,
            layer_filter: String::new(),
            selected_feature_idx: None,
            grid_enabled: true,
            cursor_world_xy: [219_850.0, 360_120.0],
            viewport: ViewportTransform::new([219_850.0, 360_120.0], 0.5, [1000.0, 720.0]),
            draw_guide_message: None,
            scene: MapSceneModel::new_demo_gyeongju_scene(),
            selected_sido_idx: 0,
            cad: CadSessionController::default(),
            active_basemap: BasemapSource::VworldSatellite,
        }
    }
}

impl HomeMapTabState {
    pub fn update_guide_for_tool(&mut self, tool: MapToolKind) {
        let hint = tool.guide_hint_ko();
        if hint.is_empty() {
            self.draw_guide_message = None;
        } else {
            self.draw_guide_message = Some(hint.to_owned());
        }
    }

    pub fn jump_to_sido(&mut self, idx: usize) {
        let presets = sido_presets();
        if let Some(p) = presets.get(idx) {
            self.selected_sido_idx = idx;
            self.viewport.center_xy = p.center_5187;
        }
    }
}
