use std::fs;
use std::path::Path;

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

/// C++ `ShortcutManager::registerShortcut` 단축키 중복 충돌 방어 레지스트리 1:1
#[derive(Debug, Clone, Default)]
pub struct ShortcutManager {
    pub bindings: std::collections::BTreeMap<String, String>,
}

impl ShortcutManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// C++ `ShortcutManager::registerShortcut(actionId, seq) -> bool` — 이미 등록된 키 시퀀스(`seq`)와 충돌 시 `false` 반환 1:1
    pub fn register_shortcut(&mut self, action_id: &str, key_seq: &str) -> bool {
        let normalized_seq = key_seq.trim().to_ascii_uppercase();
        if normalized_seq.is_empty() || action_id.trim().is_empty() {
            return false;
        }
        if self.bindings.values().any(|existing| existing == &normalized_seq) {
            return false;
        }
        self.bindings.insert(action_id.trim().to_owned(), normalized_seq);
        true
    }
}

/// C++ `KaHomeScreen::ProjectTemplateConfig` 신규 프로젝트 생성 템플릿 설정 1:1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTemplateConfig {
    pub survey_title: String,
    pub work_crs: WorkCrs,
    pub investigator: String,
    pub default_layers: Vec<DomainLayerKey>,
}

impl Default for ProjectTemplateConfig {
    fn default() -> Self {
        Self {
            survey_title: "새 고고학 지표·시굴조사".to_owned(),
            work_crs: WorkCrs::Epsg5186,
            investigator: "국가유산청 지표·시굴조사단".to_owned(),
            default_layers: vec![
                DomainLayerKey::SurveyArea,
                DomainLayerKey::FeaturePoly,
                DomainLayerKey::FeatureLine,
                DomainLayerKey::TrialTrench,
            ],
        }
    }
}

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
    pub open_studio_tabs: Vec<MainTab>,
    pub scale_menu_open: bool,
    pub theme_menu_open: bool,
    pub large_font_mode: bool,
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
        let accounts = AccountDialogState::probe_user_accounts();
        Self {
            active_tab: MainTab::Home,
            open_studio_tabs: Vec::new(),
            scale_menu_open: false,
            theme_menu_open: false,
            large_font_mode: false,
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
            accounts,
            modals: ModalDialogController::default(),
        }
    }

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

    pub const HGIS_STANDARD_DPI: f64 = 96.0;
    pub const HGIS_METERS_PER_INCH: f64 = 0.0254;
    pub const HGIS_FALLBACK_SCALE_DENOM: f64 = 1_000.0;
    pub const HGIS_PRESET_SCALES: [u32; 6] = [100, 500, 1_000, 2_500, 5_000, 10_000];
    pub const HGIS_PRESET_SCALES_7: [u32; 7] = [500, 1_000, 2_500, 5_000, 10_000, 25_000, 50_000];

    /// C++ `KaStatusBar::updateCoordinates(const QgsPointXY &point)` 1:1 대응:
    /// 천단위 콤마 및 소수점 3자리(밀리미터 해상도) 고정폭 포맷팅 (`"X 219,897.093  Y 361,622.354"`).
    /// 부동소수점 오염(`NaN`/`Infinity`) 시 `"X ---.---  Y ---.---"` 반환.
    #[must_use]
    pub fn format_status_bar_coordinates(x: f64, y: f64) -> String {
        if !x.is_finite() || !y.is_finite() {
            return "X ---.---  Y ---.---".to_owned();
        }
        fn fmt_coord(val: f64) -> String {
            let sign = if val < 0.0 { "-" } else { "" };
            let abs_v = val.abs();
            let rounded_millis = (abs_v * 1000.0).round() as u64;
            let int_part = rounded_millis / 1000;
            let frac_part = rounded_millis % 1000;
            let digits = int_part.to_string();
            let mut rev_commas = String::with_capacity(digits.len() + digits.len() / 3);
            for (idx, ch) in digits.chars().rev().enumerate() {
                if idx > 0 && idx % 3 == 0 {
                    rev_commas.push(',');
                }
                rev_commas.push(ch);
            }
            let int_str: String = rev_commas.chars().rev().collect();
            format!("{sign}{int_str}.{frac_part:03}")
        }
        format!("X {:>11}  Y {:>11}", fmt_coord(x), fmt_coord(y))
    }

    /// C++ `KaStatusBar::computeRepresentativeScale` 폴백 버전 — 비정상 입력 시 `1000.0` (`1:1,000`) 반환
    #[must_use]
    pub fn compute_representative_scale_or_fallback(
        meters_per_pixel: f64,
        screen_dpi: f64,
    ) -> f64 {
        Self::compute_representative_scale(meters_per_pixel, screen_dpi)
            .unwrap_or(Self::HGIS_FALLBACK_SCALE_DENOM)
    }

    /// C++ `KaStatusBar::computeRepresentativeScale(metersPerPixel, screenDpi)` — 0 나누기/NaN 방어 및 1:N 축척 분모 산출
    #[must_use]
    pub fn compute_representative_scale(meters_per_pixel: f64, screen_dpi: f64) -> Option<f64> {
        if !meters_per_pixel.is_finite()
            || meters_per_pixel <= 1e-9
            || !screen_dpi.is_finite()
            || screen_dpi <= 0.0
        {
            return None;
        }
        let pixels_per_meter = screen_dpi / Self::HGIS_METERS_PER_INCH;
        Some(meters_per_pixel * pixels_per_meter)
    }

    /// C++ `KaStatusBar` 축척 라벨 포맷 — `"1:1,000"` 또는 비정상 입력 시 `"1: ---"`
    #[must_use]
    pub fn format_representative_scale(meters_per_pixel: f64, screen_dpi: f64) -> String {
        let Some(scale) = Self::compute_representative_scale(meters_per_pixel, screen_dpi) else {
            return "1: ---".to_owned();
        };
        let n = scale.round() as u64;
        let digits = n.to_string();
        let mut out = String::with_capacity(digits.len() + digits.len() / 3);
        for (idx, ch) in digits.chars().rev().enumerate() {
            if idx > 0 && idx % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        let formatted: String = out.chars().rev().collect();
        format!("1:{formatted}")
    }

    /// C++ `KaStatusBar` 더티 플래그 문구 전환
    #[must_use]
    pub const fn dirty_status_label_ko(is_dirty: bool) -> &'static str {
        if is_dirty {
            "● 저장 필요"
        } else {
            "✓ 저장됨"
        }
    }

    /// C++ `KaHomeScreen::createNewProject(const ProjectTemplateConfig &config) -> std::optional<QString>` 1:1
    pub fn create_new_project(
        &mut self,
        project_file_path: &Path,
        config: &ProjectTemplateConfig,
    ) -> Option<String> {
        let parent = project_file_path.parent()?;
        if !parent.exists() && fs::create_dir_all(parent).is_err() {
            return None;
        }
        let layer_codes: Vec<&str> = config
            .default_layers
            .iter()
            .map(|l| l.id())
            .collect();
        let payload = format!(
            "STRATA_PROJECT_V1\ntitle={}\ncrs={}\ninvestigator={}\nlayers={}\n",
            config.survey_title,
            config.work_crs.auth_id(),
            config.investigator,
            layer_codes.join(",")
        );
        if fs::write(project_file_path, payload).is_err() {
            return None;
        }
        self.project_title.clone_from(&config.survey_title);
        self.work_crs = config.work_crs;
        self.left_layers.entries.clear();
        for &layer_key in &config.default_layers {
            self.left_layers.ensure_domain_layer(layer_key);
        }
        self.is_dirty = false;
        Some(project_file_path.to_string_lossy().into_owned())
    }

    /// C++ `MainWindow::openProject(const QString &projectPath) -> bool` 1:1
    pub fn open_project(&mut self, project_file_path: &Path) -> bool {
        let Ok(raw) = fs::read_to_string(project_file_path) else {
            return false;
        };
        if !raw.starts_with("STRATA_PROJECT_V1\n") {
            return false;
        }
        let mut parsed_title = None;
        let mut parsed_crs = None;
        let mut parsed_layers = Vec::new();
        for line in raw.lines().skip(1) {
            if let Some(rest) = line.strip_prefix("title=") {
                parsed_title = Some(rest.to_owned());
            } else if let Some(rest) = line.strip_prefix("crs=") {
                parsed_crs = match rest.trim() {
                    "EPSG:5186" => Some(WorkCrs::Epsg5186),
                    "EPSG:5187" => Some(WorkCrs::Epsg5187),
                    _ => None,
                };
            } else if let Some(rest) = line.strip_prefix("layers=") {
                for code in rest.split(',').filter(|s| !s.is_empty()) {
                    if let Some(k) = DomainLayerKey::from_id(code.trim()) {
                        parsed_layers.push(k);
                    }
                }
            }
        }
        let (Some(title), Some(crs)) = (parsed_title, parsed_crs) else {
            return false;
        };
        self.project_title = title;
        self.work_crs = crs;
        self.left_layers.entries.clear();
        for layer_key in parsed_layers {
            self.left_layers.ensure_domain_layer(layer_key);
        }
        self.active_tab = MainTab::Map;
        self.is_dirty = false;
        true
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

    /// C++ `CrashHandler::handleCrash` / `emergencyAutoSave` — 비정상 종료 시 현재 열려 있는 조사 버퍼를 `.crash_recovery.gpkg`로 즉시 직렬화 백업 1:1
    pub fn emergency_auto_save(&self, crash_recovery_path: &Path) -> bool {
        use strata_storage::gpkg::{GpkgRepository, SurveyWorkspace};
        use std::collections::BTreeMap;

        let mut ws = SurveyWorkspace {
            schema_version: 3,
            work_crs: self.work_crs,
            layers: BTreeMap::new(),
            embedded_project_json: format!(
                "{{\"ka_hgis_crash_recovery\":true,\"title\":{:?}}}",
                self.project_title
            ),
        };
        for entry in &self.left_layers.entries {
            if let Some(key) = DomainLayerKey::from_id(&entry.layer_key) {
                ws.ensure_domain_layer(key);
            }
        }
        GpkgRepository::write_workspace_file(crash_recovery_path, &ws).is_ok()
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

        app.open_studio_tab(MainTab::Drawing);
        assert_eq!(app.active_tab, MainTab::Drawing);
        assert!(app.open_studio_tabs.contains(&MainTab::Drawing));

        app.active_tab = MainTab::Map;
        assert!(app.open_studio_tabs.contains(&MainTab::Drawing));

        app.open_studio_tab(MainTab::Drawing);
        app.close_studio_tab(MainTab::Drawing);
        assert_eq!(app.active_tab, MainTab::Map);
        assert!(!app.open_studio_tabs.contains(&MainTab::Drawing));
    }

    #[test]
    fn test_project_lifecycle_creates_and_reopens_4_default_layers_and_epsg_5186() {
        let tmp_path = std::env::temp_dir().join(format!(
            "strata_test_project_{}.qgs",
            std::process::id()
        ));
        let mut creator = StrataAppState::new_startup();
        let config = ProjectTemplateConfig::default();
        let created = creator.create_new_project(&tmp_path, &config);
        assert!(created.is_some());

        let mut reader = StrataAppState::new_startup();
        assert_eq!(reader.active_tab, MainTab::Home);
        assert!(reader.open_project(&tmp_path));
        assert_eq!(reader.active_tab, MainTab::Map);
        assert_eq!(reader.work_crs.auth_id(), "EPSG:5186");
        assert_eq!(reader.left_layers.entries.len(), 4);

        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_status_bar_scale_1000_from_0264583333m_and_dirty_label_and_studio_tab_close() {
        let mpp = 0.264_583_333_333_333_3_f64;
        let scale = StrataAppState::compute_representative_scale(mpp, StrataAppState::HGIS_STANDARD_DPI)
            .expect("valid scale");
        assert!((scale - 1000.0).abs() < 1e-4);
        assert_eq!(
            StrataAppState::format_representative_scale(mpp, StrataAppState::HGIS_STANDARD_DPI),
            "1:1,000"
        );
        assert_eq!(
            StrataAppState::format_representative_scale(-1.0, StrataAppState::HGIS_STANDARD_DPI),
            "1: ---"
        );
        assert_eq!(StrataAppState::dirty_status_label_ko(true), "● 저장 필요");
        assert_eq!(StrataAppState::dirty_status_label_ko(false), "✓ 저장됨");

        let mut app = StrataAppState::new_startup();
        app.open_studio_tab(MainTab::Drawing);
        app.open_studio_tab(MainTab::Paleo);
        app.active_tab = MainTab::Drawing;
        assert_eq!(app.open_studio_tabs.len(), 2);
        app.close_studio_tab(MainTab::Drawing);
        assert_eq!(app.open_studio_tabs.len(), 1);
        assert_eq!(app.active_tab, MainTab::Map);
    }
}
