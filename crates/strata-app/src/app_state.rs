use strata_canvas2d::map_tools::MapToolKind;
use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;
use strata_types::theme_tokens::ThemeTokens;

use crate::dialogs_account::AccountDialogState;
use crate::dialogs_modal::ModalDialogController;
use crate::left_layer_panel::LeftLayerPanel;
use crate::ribbon::{KaBeginnerRibbon, RibbonCategory};
use crate::right_inspector::RightInspectorState;
use crate::tab_drawing::DrawingTabState;
use crate::tabs_home_map::HomeMapTabState;
use crate::tabs_paleo_3d::PaleoTerrain3dTabState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Home,
    Map,
    Paleo,
    Drawing,
    Terrain3d,
}

impl MainTab {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Home,
            Self::Map,
            Self::Paleo,
            Self::Drawing,
            Self::Terrain3d,
        ]
    }

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Home => "홈",
            Self::Map => "지도",
            Self::Paleo => "고지형분석",
            Self::Drawing => "도면",
            Self::Terrain3d => "입체지형",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrataAppState {
    pub active_tab: MainTab,
    pub work_crs: WorkCrs,
    pub theme: ThemeTokens,
    pub paper_look: bool,
    pub project_title: String,
    pub is_dirty: bool,
    pub active_tool: MapToolKind,
    pub ribbon: KaBeginnerRibbon,
    pub left_layers: LeftLayerPanel,
    pub right_inspector: RightInspectorState,
    pub home_map: HomeMapTabState,
    pub paleo_3d: PaleoTerrain3dTabState,
    pub drawing: DrawingTabState,
    pub accounts: AccountDialogState,
    pub modals: ModalDialogController,
}

impl StrataAppState {
    #[must_use]
    pub fn new_startup() -> Self {
        let default_cfg = std::env::temp_dir().join("ka-hgis");
        Self {
            active_tab: MainTab::Home,
            work_crs: WorkCrs::Epsg5187,
            theme: ThemeTokens::paper_tokens(false),
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
            accounts: AccountDialogState::probe_local_accounts(&default_cfg),
            modals: ModalDialogController::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_opens_home_tab_and_drawing_ribbon_tool_switches_to_map_tab() {
        let mut app = StrataAppState::new_startup();
        assert_eq!(app.active_tab, MainTab::Home);
        assert_eq!(app.work_crs, WorkCrs::Epsg5187);
        assert!(!app.window_title().ends_with(" *"));
        assert!(app.left_layers.entries.is_empty());

        app.active_tab = MainTab::Drawing;
        app.trigger_map_ribbon_tool(MapToolKind::TrenchPlace);
        assert_eq!(app.active_tab, MainTab::Map);
        assert_eq!(app.active_tool, MapToolKind::TrenchPlace);

        app.is_dirty = true;
        assert!(app.window_title().ends_with("[EPSG:5187]"));
        assert!(app.window_title().contains(" * "));
    }
}
