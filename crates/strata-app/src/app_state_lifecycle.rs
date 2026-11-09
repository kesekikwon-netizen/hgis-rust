//! 시작 상태·스튜디오 탭 열고 닫기·도구 전환·스모크 요약.
//!
//! `app_state.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 메서드는 그대로다.
//! 같은 타입에 여러 파일의 `impl` 을 두는 것은 Rust 가 허용한다.

use strata_canvas2d::map_tools::MapToolKind;
use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;
use strata_types::theme_tokens::{DisplayOptions, ThemeTokens};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ModalDialogController;
use super::app_state_types::MainTab;
use crate::dialogs_account::AccountDialogState;
use crate::left_layer_panel::LeftLayerPanel;
use crate::ribbon::{KaBeginnerRibbon, RibbonCategory};
use crate::right_inspector::RightInspectorState;
use crate::tab_drawing::DrawingTabState;
use crate::tabs_home_map::HomeMapTabState;
use crate::tabs_paleo_3d::PaleoTerrain3dTabState;

impl StrataAppState {
    #[must_use]
    pub fn new_startup() -> Self {
        let accounts = AccountDialogState::probe_user_accounts();
        let default_opts = DisplayOptions {
            paper_look: true,
            ..DisplayOptions::default()
        };
        Self {
            active_tab: MainTab::Home,
            open_studio_tabs: Vec::new(),
            scale_menu_open: false,
            map_rendering_enabled: true,
            scale_input_buf: String::new(),
            map_context_menu_open: false,
            map_context_menu_pos: [0.0, 0.0],
            map_context_menu_world: [0.0, 0.0],
            layer_context_menu_open: false,
            layer_context_menu_pos: [0.0, 0.0],
            layer_context_menu_key: None,
            layer_menu_sub: 0,
            last_drop_message: None,
            close_prompt_open: false,
            cad_import_path: String::new(),
            status_bar_hint: String::new(),
            align: crate::gui_ribbon_bar::AlignSessionState::default(),
            theme_menu_open: false,
            large_font_mode: false,
            display_options: default_opts,
            work_crs: WorkCrs::Epsg5187,
            theme: ThemeTokens::for_options(&default_opts),
            theme_dirty: true,
            survey_path: None,
            paper_look: true,
            project_title: "새 고고학 지표·시굴조사".to_owned(),
            is_dirty: false,
            active_tool: MapToolKind::Pan,
            ribbon: KaBeginnerRibbon::default(),
            left_layers: LeftLayerPanel::new_empty(),
            right_inspector: RightInspectorState::new_default(),
            home_map: HomeMapTabState::default(),
            paleo_3d: PaleoTerrain3dTabState::default(),
            drawing: DrawingTabState::default(),
            accounts,
            modals: ModalDialogController::default(),
            borehole_dialog: crate::dialogs_borehole::BoreholeDialogController::new(),
        }
    }

    /// `MainWindowExport.cpp:252` — 도면 화면을 처음 열 때 상태줄에 남기는 C++ 원문.
    pub const DRAWING_STUDIO_WELCOME: &'static str =
        "도면 화면입니다. 좌표점은 용지 아래 아이콘으로 찍습니다.";

    /// C++ `MainWindowChrome::openDrawingStudio` 등 스튜디오 탭 열기 생명주기 1:1
    pub fn open_studio_tab(&mut self, tab: MainTab) {
        if matches!(tab, MainTab::Drawing | MainTab::Paleo | MainTab::Terrain3d)
            && !self.open_studio_tabs.contains(&tab)
        {
            self.open_studio_tabs.push(tab);
        }
        self.active_tab = tab;
    }

    /// C++ `MainWindowChrome::closeViewTab` 스튜디오 탭 `×` 닫기 및 메인 지도 복귀 1:1
    pub fn close_studio_tab(&mut self, tab: MainTab) {
        self.open_studio_tabs.retain(|&t| t != tab);
        if self.active_tab == tab {
            self.active_tab = MainTab::Map;
        }
    }

    #[must_use]
    pub fn window_title(&self) -> String {
        let dirty_suffix = if self.is_dirty { " *" } else { "" };
        format!(
            "Strata (ka-hgis) — {}{} [{}]",
            self.project_title,
            dirty_suffix,
            self.work_crs.auth_id()
        )
    }

    pub fn trigger_map_ribbon_tool(&mut self, tool: MapToolKind) {
        if KaBeginnerRibbon::is_map_only_tool(tool) && self.active_tab != MainTab::Map {
            self.active_tab = MainTab::Map;
        }
        self.active_tool = tool;
        self.home_map.update_guide_for_tool(tool);
        if matches!(
            tool,
            MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
        ) {
            self.left_layers
                .ensure_domain_layer(DomainLayerKey::SurveyArea);
            self.ribbon.active_category = RibbonCategory::Survey;
        }
    }

    #[must_use]
    pub fn smoke_summary(&self) -> String {
        let tab_names: Vec<&str> = MainTab::all().iter().map(|t| t.label_ko()).collect();
        format!(
            "SMOKE_OK title=\"{}\" start_tab=\"{}\" tabs=[{}] crs={} paper_look={}",
            self.window_title(),
            self.active_tab.label_ko(),
            tab_names.join(","),
            self.work_crs.auth_id(),
            self.paper_look
        )
    }

    pub fn run_event_loop_step(&mut self) {
        self.paleo_3d.run_sample_10km_analysis();
    }
}
