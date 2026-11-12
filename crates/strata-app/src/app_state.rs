//! `app_state` 는 실행 상태(`StrataAppState`)와 그 조작의 재수출 창구다.
//!
//! 원래 682줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::app_state::<이름>` 경로는 그대로 동작한다.
//!
//! `StrataAppState` **구조체 정의는 이 파일에 그대로 남긴다** — 여러 파일이
//! `crate::app_state::StrataAppState` 를 가져다 쓴다. `impl` 블록만 주제별 파일로 나눴다
//! (Rust 는 같은 타입에 여러 파일의 `impl` 을 허용한다).

use strata_canvas2d::map_tools::MapToolKind;
use strata_types::crs::WorkCrs;
use strata_types::theme_tokens::{DisplayOptions, ThemeTokens};

use crate::dialogs_account::AccountDialogState;
use crate::dialogs_modal::ModalDialogController;
use crate::left_layer_panel::LeftLayerPanel;
use crate::ribbon::KaBeginnerRibbon;
use crate::right_inspector::RightInspectorState;
use crate::tab_drawing::DrawingTabState;
use crate::tabs_home_map::HomeMapTabState;
use crate::tabs_paleo_3d::PaleoTerrain3dTabState;

#[path = "app_state_lifecycle.rs"]
mod app_state_lifecycle;
#[path = "app_state_projects.rs"]
mod app_state_projects;
#[path = "app_state_status_bar.rs"]
mod app_state_status_bar;
#[path = "app_state_types.rs"]
mod app_state_types;

pub use app_state_types::*;

#[derive(Debug, Clone)]
pub struct StrataAppState {
    pub active_tab: MainTab,
    pub open_studio_tabs: Vec<MainTab>,
    pub scale_menu_open: bool,
    // D12 KaStatusBar.cpp:121-136,77-98 — 지도갱신 토글과 축척 직접 입력칸
    pub map_rendering_enabled: bool,
    pub scale_input_buf: String,
    // MainWindowContextMenus.cpp:112-161 — 지도 캔버스 우클릭 메뉴
    pub map_context_menu_open: bool,
    pub map_context_menu_pos: [f32; 2],
    /// 메뉴를 연 지도 좌표 (N, E)
    pub map_context_menu_world: [f64; 2],
    // MainWindowContextMenus.cpp:176-448 — 왼쪽 레이어 트리 우클릭 메뉴
    pub layer_context_menu_open: bool,
    pub layer_context_menu_pos: [f32; 2],
    pub layer_context_menu_key: Option<String>,
    /// 라벨 하위 메뉴 펼침 상태 (라벨·명칭 속성 / 투명도 / 깨진 한글)
    pub layer_menu_sub: u8,
    // MainWindowFiles.cpp:259-289 — 파일 끌어 놓기
    pub last_drop_message: Option<String>,
    /// 종료 확인 (KaClosePrompt.cpp)
    pub close_prompt_open: bool,
    /// KaCadImportNotice.cpp — 끌어 놓은 CAD 도면 경로
    pub cad_import_path: String,
    /// `KaStatusBar::showMessage` 대응 — 맞추기 같은 긴 작업의 결과를 상태줄에 남긴다
    pub status_bar_hint: String,
    /// `MainWindowAlign.cpp` — 정합(맞추기) 세션. `m_alignTool` + `m_alignApplied` 1:1
    pub align: crate::gui_ribbon_bar::AlignSessionState,
    pub theme_menu_open: bool,
    pub large_font_mode: bool,
    pub display_options: DisplayOptions,
    pub work_crs: WorkCrs,
    pub theme: ThemeTokens,
    /// C++ `KaThemeOptions.cpp` — 테마·고대비가 바뀌면 다음 프레임에 스타일을 다시 적용한다
    pub theme_dirty: bool,
    /// C++ `MainWindow::m_surveyPath` — 지금 열려 있는 조사 파일(.gpkg) 경로
    pub survey_path: Option<std::path::PathBuf>,
    pub paper_look: bool,
    pub project_title: String,
    pub is_dirty: bool,
    /// 첫 프레임에 창 크기를 모니터 작업영역 안으로 맞췄는지.
    /// 200% 배율 화면에서 eframe 이 inner_size 를 물리 픽셀로 잡아 창이 화면을
    /// 넘는 문제를 막는다(`gui_shell::update` 참조).
    pub startup_window_fitted: bool,
    /// --snap 검증 모드 상태(없으면 None). 실행 중에는 항상 None.
    pub snap: Option<crate::snap_shot::SnapRequest>,
    /// 배경지도 타일 텍스처 캐시(종류·줌·x·y).
    pub basemap_textures: crate::basemap_paint::TileTextureCache,
    /// 마지막 프레임에 칠한 타일 수(진단·시험용).
    pub basemap_tiles_painted: usize,
    /// 연속지적도 z/x/y 타일별 텍스처(위성 타일과 같은 격자).
    pub cadastral_cells: crate::basemap_paint::CadastralCellCache,
    /// 연속지적도 타일을 뒤에서 받는 작업(화면을 멈추지 않는다).
    pub cadastral_fetch: crate::basemap_cadastral_jobs::CadastralFetcher,
    pub active_tool: MapToolKind,
    pub ribbon: KaBeginnerRibbon,
    pub left_layers: LeftLayerPanel,
    pub right_inspector: RightInspectorState,
    pub home_map: HomeMapTabState,
    pub paleo_3d: PaleoTerrain3dTabState,
    pub drawing: DrawingTabState,
    pub accounts: AccountDialogState,
    pub modals: ModalDialogController,
    /// C++ `KaBoreholeDialog` 창 상태 (`d7afcc8`). 자료 받기의 「시추」 단추가 연다.
    pub borehole_dialog: crate::dialogs_borehole::BoreholeDialogController,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_types::schema::DomainLayerKey;

    /// R3 — 저장이 실제 .gpkg 파일을 만들고, 다시 읽으면 좌표계·제목이 살아 있는지.
    #[test]
    fn test_r3_save_project_writes_real_gpkg_and_reopens() {
        let mut app = StrataAppState::new_startup();
        app.project_title = "병산동".to_owned();
        app.work_crs = WorkCrs::Epsg5186;
        app.left_layers
            .ensure_domain_layer(DomainLayerKey::SurveyArea);
        app.is_dirty = true;

        // 시험마다 다른 폴더를 쓴다. 고정 이름은 병렬 실행이나 동시 빌드에서
        // 다른 프로세스와 부딪혀 간헐적으로 실패한다(실제로 한 번 그렇게 실패했다).
        let dir = std::env::temp_dir().join(format!(
            "strata_r3_save_test_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("병산동.gpkg");
        let _ = std::fs::remove_file(&path);

        assert!(app.write_survey_to(&path), "save should succeed");
        assert!(path.exists(), "the .gpkg must exist on disk");
        assert!(!app.is_dirty, "a successful save clears the dirty flag");

        // 다시 읽어 좌표계와 레이어가 살아 있는지
        let ws = strata_storage::gpkg::GpkgRepository::open_read_only(&path)
            .expect("the file must be a readable GeoPackage");
        assert_eq!(ws.work_crs, WorkCrs::Epsg5186);
        assert!(ws.layers.contains_key("survey_area"));

        let _ = std::fs::remove_file(&path);
    }

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
        let tmp_path =
            std::env::temp_dir().join(format!("strata_test_project_{}.qgs", std::process::id()));
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
        let scale =
            StrataAppState::compute_representative_scale(mpp, StrataAppState::HGIS_STANDARD_DPI)
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
