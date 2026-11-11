//! `HomeMapTabState` — 홈 지도 탭 상태와 그 조작.
//!
//! `tabs_home_map.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 옮긴 항목의 이름·시그니처·동작은 그대로다.

use strata_canvas2d::cad_interaction::{snap_ring_to_nearest_lines, CadSessionController};
use strata_canvas2d::canvas_painter::MapSceneModel;
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;
use strata_net::address_search::sido_presets;
use strata_net::tile_fetcher::BasemapSource;

use crate::real_survey::{ring_area_m2, RealSurveyData};

use super::tabs_home_map_undo::{HGIS_BASEMAP_DEFAULT_BRIGHTNESS, HGIS_BASEMAP_DEFAULT_OPACITY};

#[derive(Debug, Clone)]
pub struct HomeMapTabState {
    /// 실제로 연 조사 기록(설정 폴더에 남는다)과 그것을 홈 표에 보일 줄.
    pub recent_list: Vec<crate::recent_surveys::RecentSurvey>,
    pub recent_rows: Vec<crate::recent_surveys::RecentRow>,
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
    pub grid_enabled: bool,
    pub cursor_world_xy: [f64; 2],
    pub viewport: ViewportTransform,
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
        // 시작할 때는 열린 조사가 없다(C++: 홈 화면만, 자동 복원 없음).
        let real_survey = RealSurveyData::new_empty_survey(&std::env::temp_dir());
        let recent_list = crate::recent_surveys::load_recent_surveys();
        let recent_rows = crate::recent_surveys::build_recent_rows(&recent_list);
        let center = real_survey.survey_center;
        Self {
            recent_list,
            recent_rows,
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
            grid_enabled: false,
            cursor_world_xy: center,
            viewport: ViewportTransform::new(center, 2.55, [1060.0, 680.0]),
            draw_guide_message: None,
            scene: MapSceneModel::new_demo_gyeongju_scene(),
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

    /// `SnapTraceOps::snapCanvasShapesToNearestLines` 1:1 대응
    pub fn snap_drawn_shapes_to_nearest_lines(&mut self) -> (usize, usize, usize) {
        if self.real_survey.survey_rings.is_empty() && self.scene.polygons.is_empty() {
            self.draw_guide_message = Some(
                "먼저 지적도·도면 위에 도형을 그리거나 도형선택으로 도형을 고르세요.".to_owned(),
            );
            return (0, 0, 0);
        }
        let refs = self.snap_reference_rings(self.real_survey.survey_center, 800.0);
        let mupp = self.viewport.meters_per_pixel;
        let mut changed_count = 0usize;
        let mut total_moved = 0usize;
        let mut total_inserted = 0usize;

        if let Some(last_ring) = self.real_survey.survey_rings.last_mut() {
            let (snapped, moved, ins) = snap_ring_to_nearest_lines(last_ring, true, &refs, mupp);
            if moved > 0 || ins > 0 {
                *last_ring = snapped;
                changed_count += 1;
                total_moved += moved;
                total_inserted += ins;
            }
        }
        if changed_count > 0 {
            self.real_survey.survey_area_m2 = self
                .real_survey
                .survey_rings
                .iter()
                .map(|r| ring_area_m2(r))
                .sum();
        }
        for poly in &mut self.scene.polygons {
            if poly.layer_key == "survey_area" || poly.layer_key == "feature_poly" {
                let (snapped, moved, ins) =
                    snap_ring_to_nearest_lines(&poly.world_pts, true, &refs, mupp);
                if moved > 0 || ins > 0 {
                    poly.world_pts = snapped;
                    changed_count += 1;
                    total_moved += moved;
                    total_inserted += ins;
                }
            }
        }
        self.draw_guide_message = Some(if changed_count > 0 {
            format!(
                "도형을 가장 가까운 선에 붙였습니다 (꼭짓점 {total_moved}개 맞춤 · 사이점 {total_inserted}개 채움). Ctrl+Z로 되돌릴 수 있습니다."
            )
        } else {
            "이미 가장 가까운 선에 붙어 있거나 주변에 붙일 기준 선(지적·도면)이 없습니다."
                .to_owned()
        });
        (changed_count, total_moved, total_inserted)
    }
}
