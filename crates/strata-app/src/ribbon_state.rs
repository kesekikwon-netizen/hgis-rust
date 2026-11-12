//! 리본 상태(`KaBeginnerRibbon`)와 그 조작.
//!
//! `ribbon.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

use strata_canvas2d::map_tools::MapToolKind;

use super::ribbon_appbar::KaAppBarSearch;
use super::ribbon_types::{DrawSubTarget, RibbonCategory, SubToolsMode};

#[derive(Debug, Clone)]
pub struct KaBeginnerRibbon {
    pub active_category: RibbonCategory,
    pub sub_toolbar_visible: bool,
    // KaPaleoMenu::install — 리본 우클릭/클릭 팝업 메뉴 상태
    pub paleo_menu_open: bool,
    pub geology_menu_open: bool,
    pub dem_menu_open: bool,
    pub soil_menu_open: bool,
    pub cadastral_menu_open: bool,
    pub topo_menu_open: bool,
    pub old_map_menu_open: bool,
    pub more_menu_open: bool,
    pub context_menu_anchor: eframe::egui::Pos2,
    pub sub_mode: SubToolsMode,
    pub draw_sub_target: DrawSubTarget,
    pub snap_assist_menu_open: bool,
    /// C++ `m_continuousDraw` — 그리기 줄 「연속 그리기」.
    pub continuous_draw: bool,
    pub app_bar: KaAppBarSearch,
}

impl Default for KaBeginnerRibbon {
    fn default() -> Self {
        Self {
            active_category: RibbonCategory::Survey,
            sub_toolbar_visible: false,
            paleo_menu_open: false,
            geology_menu_open: false,
            dem_menu_open: false,
            soil_menu_open: false,
            cadastral_menu_open: false,
            topo_menu_open: false,
            old_map_menu_open: false,
            more_menu_open: false,
            context_menu_anchor: eframe::egui::Pos2::ZERO,
            sub_mode: SubToolsMode::None,
            draw_sub_target: DrawSubTarget::SurveyArea,
            snap_assist_menu_open: false,
            continuous_draw: false,
            app_bar: KaAppBarSearch::default(),
        }
    }
}

impl KaBeginnerRibbon {
    #[must_use]
    pub const fn categories() -> &'static [RibbonCategory] {
        &[
            RibbonCategory::Survey,
            RibbonCategory::Record,
            RibbonCategory::Download,
            RibbonCategory::Basemap,
            RibbonCategory::Georef,
            RibbonCategory::Export,
            RibbonCategory::Misc,
        ]
    }

    #[must_use]
    pub const fn is_map_only_tool(tool: MapToolKind) -> bool {
        matches!(
            tool,
            MapToolKind::DrawPolygon
                | MapToolKind::DrawLine
                | MapToolKind::PlacePoint
                | MapToolKind::TrenchPlace
                | MapToolKind::GeorefAlign
                | MapToolKind::Measure
                | MapToolKind::FeatureCardNote
        )
    }

    pub fn toggle_sub_mode(&mut self, mode: SubToolsMode) {
        if self.sub_toolbar_visible && self.sub_mode == mode {
            self.sub_toolbar_visible = false;
            self.sub_mode = SubToolsMode::None;
        } else {
            self.sub_toolbar_visible = true;
            self.sub_mode = mode;
        }
    }

    pub const HGIS_RIBBON_COMPACT_WIDTH_PX: f32 = 1100.0;

    /// C++ `KaRibbonBar::isCompactMode` — 창 너비 1,100px 미만 시 24x24 컴팩트 아이콘 모드 전환
    #[must_use]
    pub fn is_compact_mode(window_width_px: f32) -> bool {
        window_width_px < Self::HGIS_RIBBON_COMPACT_WIDTH_PX
    }

    /// C++ `KaRibbonBar::setActiveTool` — 도구 전환 시 미완성 러버밴드 버퍼 초기화 및 하단 상태바 안내 문구 매핑
    pub fn switch_active_tool_clearing_rubberband(
        active_tool: &mut MapToolKind,
        sketch_buffer: &mut Vec<[f64; 2]>,
        next_tool: MapToolKind,
    ) -> &'static str {
        if *active_tool != next_tool {
            sketch_buffer.clear();
            *active_tool = next_tool;
        }
        Self::status_bar_hint_for_tool(*active_tool)
    }

    /// C++ `KaStatusBar` 도구별 하단 상태바 작업 안내 문구 매핑 1:1
    #[must_use]
    pub const fn status_bar_hint_for_tool(tool: MapToolKind) -> &'static str {
        match tool {
            MapToolKind::Pan => "지도를 드래그하여 이동하거나 마우스 휠로 확대/축소하세요.",
            MapToolKind::Select => "피처를 클릭하여 선택하거나 드래그하여 다중 선택하세요.",
            MapToolKind::DrawPolygon => {
                "시작점을 클릭하세요. [Shift]: 직교/45도 스냅, [우클릭]: 폐합 완료"
            }
            MapToolKind::DrawLine => "선 시작점을 클릭하세요. [우클릭]: 선 작도 완료",
            MapToolKind::PlacePoint => "지도 위 원하는 위치를 클릭하여 유물·기준점을 배치하세요.",
            MapToolKind::TrenchPlace => {
                "조사구역 내부를 클릭하여 표준 시굴 트렌치(20m×2m / L자형)를 배치하세요."
            }
            MapToolKind::GeorefAlign => {
                "정점을 클릭하여 드래그 이동하세요. [Delete]: 선택 정점 삭제"
            }
            MapToolKind::Measure => {
                "클릭하여 측정을 시작하세요. [우클릭/더블클릭]: 측정 완료, [ESC]: 취소"
            }
            MapToolKind::FeatureCardNote => "유구를 클릭하여 조사카드 상세 기록 창을 여세요.",
        }
    }
}
