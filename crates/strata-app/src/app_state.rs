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
#[path = "app_state_draw_entry.rs"]
mod app_state_draw_entry;
#[path = "app_state_draw_commit.rs"]
mod app_state_draw_commit;
#[path = "app_state_draw_row.rs"]
mod app_state_draw_row;
#[path = "app_state_buffer.rs"]
mod app_state_buffer;
#[path = "app_state_snap_lines.rs"]
mod app_state_snap_lines;
#[path = "app_state_recent.rs"]
mod app_state_recent;
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
    /// 열린 조사 파일의 도형 표(gpkg_contents 순서)와 도형 — C++ 은 QGIS 레이어가 들고 있다.
    pub survey_tables: Vec<strata_storage::survey_gpkg_read::SurveyFileTable>,
    /// C++ `RecentSurveys::userSettings()` 의 ini 파일. `new_startup` 은 비워 두고(시험이 사용자 ini 를
    /// 읽고 쓰지 않게) 실행 파일만 `attach_user_settings` 로 붙인다.
    pub settings_ini: Option<std::path::PathBuf>,
    /// 홈 「최근 조사」 목록 — C++ `KaStartPage::reload` 의 `RecentSurveys::loadAll`.
    pub recent_surveys: Vec<strata_storage::recent_surveys::RecentSurvey>,
    /// C++ `m_editLayer` — 그리기 입구가 정한 대상 표.
    pub edit_layer_key: Option<String>,
    /// C++ `KaCaptureMapTool::m_points` — 그리던 면.
    pub(crate) draw_sketch: crate::draw_sketch::DrawSketch,
    /// 되돌리기·다시 실행 차례(표 이름, 최근 것이 끝) — C++ newestUndoLayer·newestRedoLayer.
    pub(crate) undo_tables: Vec<crate::app_state_layer_remove::UndoStep>,
    pub(crate) redo_tables: Vec<String>,
    /// 지도 우클릭 메뉴의 「조사 도형 그리기」 하위 메뉴가 펼쳐져 있는가.
    pub map_menu_sub_open: bool,
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
    /// Background satellite/base XYZ tile fetcher (`target/cpp-ref/src/app/MainWindow.cpp:526`).
    pub basemap_fetch: crate::basemap_cadastral_jobs::BasemapFetcher,
    /// Cached VWorld API key refreshed at most every 2 seconds across both tile paths.
    pub(crate) vworld_api_key: crate::basemap_cadastral_jobs::CachedApiKey,
    // Running, queued, recovering, and stopped downloads (`docs/RUST_PLAN.md` §2.8).
    pub download_jobs: crate::download_jobs::DownloadJobList,
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

#[cfg(test)]
#[path = "app_state_save_tests.rs"]
mod save_tests;

#[cfg(test)]
#[path = "app_state_tests.rs"]
mod tests;
