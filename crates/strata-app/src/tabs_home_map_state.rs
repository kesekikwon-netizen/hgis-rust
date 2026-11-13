//! `HomeMapTabState` — 홈 지도 탭 상태와 그 조작.
//!
//! `tabs_home_map.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 옮긴 항목의 이름·시그니처·동작은 그대로다.

use strata_canvas2d::cad_interaction::CadSessionController;
use strata_canvas2d::canvas_painter::MapSceneModel;
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;
use strata_net::address_search::sido_presets;
use strata_net::tile_fetcher::BasemapSource;

use crate::real_survey::RealSurveyData;

use super::tabs_home_map_undo::{HGIS_BASEMAP_DEFAULT_BRIGHTNESS, HGIS_BASEMAP_DEFAULT_OPACITY};

/// 도형선택 겹친 도형 돌기 — (지난 후보 (표, fid), 고른 차례, 누른 화면 자리).
pub type PickCycle = (Vec<(String, i64)>, usize, [f32; 2]);

#[derive(Debug, Clone)]
pub struct HomeMapTabState {
    pub recent_filter: String,
    // D8 KaLayerInformationPanel.cpp:25-190 '표시 설정' 상태
    pub layer_display_settings_open: bool,
    pub layer_label_field_idx: usize,
    pub layer_label_with_area: bool,
    pub layer_label_size_pt: i32,
    pub layer_label_ink_idx: usize,
    pub layer_label_buffer_halo: bool,
    pub internet_checked: bool,
    pub files_open: bool,
    pub layer_filter: String,
    pub selected_feature_idx: Option<usize>,
    /// 도형선택으로 고른 조사 도형(표 이름, fid) — C++ layer->selectedFeatureIds.
    pub selected_shape: Option<(String, i64)>,
    /// 방금 그린 면(표 이름, fid) — C++ m_lastDrawnLayer·m_lastDrawnFid(「가까운 선에 붙이기」가 고른 것이 없을 때 쓴다).
    pub last_drawn_shape: Option<(String, i64)>,
    /// 도형선택 겹친 도형 돌기 — (지난 후보, 고른 차례, 누른 화면 자리)(m_lastPickCandidates·Index·Pos).
    pub pick_cycle: Option<PickCycle>,
    /// 열려 있는 「조사구역 그리기 설정」 창의 값(없으면 창이 닫혀 있다).
    pub(crate) survey_area_setup: Option<crate::survey_area_setup::SurveyAreaSetup>,
    /// 열려 있는 「이름·번호」 창(그린 도형 하나의 값).
    pub(crate) feature_form: Option<crate::feature_form_state::FeatureForm>,
    /// 지도 우클릭 메뉴를 연 자리의 조사 도형(표, fid) — 「이 도형 기록 입력」.
    pub(crate) menu_hit: Option<(String, i64)>,
    /// 열려 있는 「도형 속성」 창.
    pub(crate) feature_record: Option<crate::feature_record::FeatureRecordForm>,
    pub grid_enabled: bool,
    pub cursor_world_xy: [f64; 2],
    /// 마우스가 지도 칸 위에 한 번이라도 왔는가 — 그 전에는 상태줄 좌표가 「X —    Y —」(KaStatusBar.cpp:173-175).
    pub cursor_seen: bool,
    pub viewport: ViewportTransform,
    /// 다음 지도 화면에서 한반도 전체 보기를 한 번 다시 맞출지(지도 칸 크기가 그때 정해진다).
    pub korea_view_pending: bool,
    /// QGIS 지도 범위 이력(「이전·다음 지도 범위」).
    pub(crate) extent_history: crate::extent_history::ExtentHistory,
    pub draw_guide_message: Option<String>,
    pub scene: MapSceneModel,
    pub selected_sido_idx: usize,
    pub cad: CadSessionController,
    pub active_basemap: BasemapSource,
    pub basemap_opacity: f32,
    pub basemap_brightness: f32,
    pub real_survey: RealSurveyData,
    /// 실제로 받은 DEM 표고 격자(없으면 「지형 음영」을 그리지 않는다).
    /// C++ 은 QgsRasterLayer 가 있을 때만 지형 음영 레이어를 만든다
    /// (LayerOps::addElevationHillshadeMap). 값은 (변환, 표고 배열) 쌍이다.
    pub dem_grid: Option<(
        strata_paleo::terrain_edt_tpi::GridTransform,
        Vec<f64>,
    )>,
}

impl Default for HomeMapTabState {
    fn default() -> Self {
        // 시작에는 조사 자료가 없다(C++ 은 홈 화면만 띄운다). 열기·새 조사가 같은 빈 상태로 비운다.
        let real_survey = RealSurveyData::new_empty_survey(&std::env::temp_dir());
        let center = real_survey.survey_center;
        Self {
            recent_filter: String::new(),
            layer_display_settings_open: false,
            layer_label_field_idx: 0,
            layer_label_with_area: false,
            layer_label_size_pt: 9,
            layer_label_ink_idx: 0,
            layer_label_buffer_halo: true,
            internet_checked: false,
            files_open: true,
            layer_filter: String::new(),
            selected_feature_idx: None,
            selected_shape: None,
            last_drawn_shape: None,
            pick_cycle: None,
            survey_area_setup: None,
            feature_form: None,
            feature_record: None,
            menu_hit: None,
            grid_enabled: false,
            cursor_world_xy: center,
            cursor_seen: false,
            viewport: ViewportTransform::new(center, 2.55, [1060.0, 680.0]),
            korea_view_pending: false,
            extent_history: crate::extent_history::ExtentHistory::default(),
            draw_guide_message: None,
            scene: MapSceneModel::default(),
            selected_sido_idx: 0,
            cad: CadSessionController::default(),
            active_basemap: BasemapSource::VworldSatellite,
            basemap_opacity: HGIS_BASEMAP_DEFAULT_OPACITY,
            basemap_brightness: HGIS_BASEMAP_DEFAULT_BRIGHTNESS,
            real_survey,
            dem_grid: None,
        }
    }
}

impl HomeMapTabState {
    /// C++ `MapCanvasOverlay::setBasemapOpacity` — 배경지도 투명도 `[0.0, 1.0]` 클램핑 1:1
    pub fn set_basemap_opacity(&mut self, opacity: f32) {
        self.basemap_opacity = opacity.clamp(0.0, 1.0);
    }

    /// C++ `MapCanvasOverlay::setBasemapBrightness` — 배경지도 밝기 `[-1.0, 1.0]` 클램핑 1:1
    pub fn set_basemap_brightness(&mut self, brightness: f32) {
        self.basemap_brightness = brightness.clamp(-1.0, 1.0);
    }

    /// C++ `MapCanvasOverlay::syncUndoRedoPills` — 실행취소/다시실행 스택 상태에 따른 알약 버튼 활성화 여부 1:1
    #[must_use]
    pub const fn sync_undo_redo_pills(undo_count: usize, redo_count: usize) -> (bool, bool) {
        (undo_count > 0, redo_count > 0)
    }
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

    pub fn fit_to_survey(&mut self) {
        self.viewport.center_xy = self.real_survey.survey_center;
        self.viewport.meters_per_pixel = 2.55;
    }

    #[must_use]
    pub fn snap_reference_rings(&self, around_xy: [f64; 2], radius_m: f64) -> Vec<Vec<[f64; 2]>> {
        let r_sq = radius_m * radius_m;
        let mut out = Vec::new();
        for p in &self.real_survey.cadastral_parcels {
            if p.ring.first().is_some_and(|pt| {
                let dx = pt[0] - around_xy[0];
                let dy = pt[1] - around_xy[1];
                dx * dx + dy * dy <= r_sq
            }) {
                out.push(p.ring.clone());
            }
        }
        for h in &self.real_survey.heritage_polys {
            if h.ring.first().is_some_and(|pt| {
                let dx = pt[0] - around_xy[0];
                let dy = pt[1] - around_xy[1];
                dx * dx + dy * dy <= r_sq
            }) {
                out.push(h.ring.clone());
            }
        }
        out
    }

    pub fn update_cursor_snap_preview(&mut self) {
        let refs = self.snap_reference_rings(self.cursor_world_xy, 220.0);
        let mupp = self.viewport.meters_per_pixel;
        self.cad
            .update_cursor_preview(self.cursor_world_xy, &self.scene, &refs, mupp);
    }

    pub fn on_canvas_left_click(&mut self, world_xy: [f64; 2], tool: MapToolKind) {
        self.on_canvas_left_click_on_layer(world_xy, tool, None);
    }

    /// `KaSnapSettingsWidget.cpp:24-27` — 「현재 레이어」를 고르면 `current_layer_key` 도형에만 붙는다
    pub fn on_canvas_left_click_on_layer(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        current_layer_key: Option<&str>,
    ) {
        let refs = self.snap_reference_rings(world_xy, 300.0);
        let mupp = self.viewport.meters_per_pixel;
        self.cad.on_left_click_with_refs_and_layer(
            world_xy,
            tool,
            &mut self.scene,
            &refs,
            mupp,
            current_layer_key,
        );
    }

}
