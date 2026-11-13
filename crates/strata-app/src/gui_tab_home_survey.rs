//! 「홈」 탭에서 여는 조사 — 새 조사(빈 프로젝트)·파일 열기·GPKG 읽기.
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 같은 `impl` 블록 안에 있던 메서드들을 그대로 옮겼고, 동작·문구는 바꾸지 않았다.

use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg::GpkgRepository;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};

impl StrataAppState {
    /// C++ `MainWindow::newSurvey`(`MainWindowSession.cpp:669‑751`) — 새 조사는
    /// 이전 조사에서 아무 것도 물려받지 않는 **빈 프로젝트**에서 시작한다.
    /// C++ 은 `kaSafeClearQgisProject` 로 QGIS 프로젝트를 비우고 `loadSurveyLayers(path)`
    /// 로 방금 만든 빈 GPKG 만 읽는다(`removeSurveyDomainLayers`). Rust 판은 같은 자리에서
    /// 지도 장면·레이어 목록·조사 경계·시굴·지적·유산·선택·편집 세션을 모두 비우고,
    /// 방금 만든 파일이 있으면 그 파일의(비어 있는) 레이어만 되살린다.
    pub fn start_new_survey_project_with_path(
        &mut self,
        name: &str,
        path: &std::path::Path,
        work_crs: strata_types::crs::WorkCrs,
    ) {
        self.reset_survey_workspace(name);
        self.work_crs = work_crs;

        // C++ `loadSurveyLayers` → `addNonEmptyDomainLayers`: 빈 GPKG 이므로
        // 도메인 레이어는 0개다. 도형이 있는 표만 되살리고, 표는 저장할 수 있게 메모리에 둔다.
        let mut restored = 0usize;
        if let Ok(tables) = strata_storage::survey_gpkg_read::read_survey_tables(path) {
            for t in tables.iter().filter(|t| !t.layer.features.is_empty()) {
                if let Some(key) = DomainLayerKey::from_id(&t.table) {
                    self.left_layers.ensure_domain_layer(key);
                    restored += 1;
                }
            }
            self.survey_tables = tables;
        }

        // C++ `applyStartupMap` → `ensureDefaultBasemaps`: 위성 + 지적 딱 두 개.
        // (사용자 지적 2026-11-09: 최초 화면에 이 둘만 떠야 한다)
        self.left_layers.apply_default_basemaps();
        let total = self.left_layers.entries.len();

        self.survey_path = Some(path.to_path_buf());
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!(
            "새 조사 '{}' ({}) 프로젝트를 생성했습니다. 배경지도 2개(위성·지적)를 켰습니다{}.",
            self.project_title,
            self.work_crs.auth_id(),
            if restored == 0 {
                String::new()
            } else {
                format!(", 조사 레이어 {restored}개 복원")
            }
        ));
        debug_assert!(
            total >= 2,
            "새 조사에는 위성·지적 배경지도가 최소 2개 있어야 한다"
        );
        // C++ `MainWindowSession.cpp:741-742` — rememberSurvey(path, 대화상자의 이름) · rememberSurveyDir.
        self.remember_survey(path, name);
        self.remember_survey_dir(path);
    }

    /// `start_new_survey_project` 의 예전 시험·시연용 입구(Ctrl+N). 부를 때마다 임시 폴더 안
    /// 새 폴더에 만들고, 만들지 못하면 멈춘다 — 기존 파일로 계속하지 않는다.
    pub fn start_new_survey_project(&mut self, name: &str) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir()
            .join(format!("strata-new-survey-{}-{stamp}", std::process::id()));
        let path = dir.join(format!("{name}.gpkg"));
        let crs = self.modals.new_survey_crs;
        if let Err(e) = GpkgRepository::create_new_survey(&path, crs) {
            let msg = crate::dlg_new_survey::new_survey_error_text(&e);
            eprintln!("[NEW SURVEY] {}: {msg}", path.display());
            self.home_map.draw_guide_message = Some(msg);
            return;
        }
        self.start_new_survey_project_with_path(name, &path, crs);
    }

    /// 이전 조사의 지도·레이어·편집 상태를 모두 비운다(C++ `kaSafeClearQgisProject` 자리).
    /// 조사 파일 자체는 건드리지 않는다 — 저장하지 않은 편집은 호출자가 먼저 확인한다.
    ///
    /// C++ 은 `removeSurveyDomainLayers` 로 **조사 데이터 레이어만** 지우고 배경지도는
    /// `ensureDefaultBasemaps` 가 다시 맞춘다. 여기서도 패널을 통째로 비우지 않고
    /// 조사 데이터 그룹만 지운다(사용자 지적 2026-11-09: 레이어가 남아 있었다).
    pub(super) fn reset_survey_workspace(&mut self, name: &str) {
        self.project_title = name.to_owned();
        self.survey_path = None;
        self.survey_tables.clear();
        // C++ newSurvey·openSurveyGpkg 의 m_editLayer = nullptr — 이전 조사의 그리기 대상을 남기지 않는다.
        self.edit_layer_key = None;
        // 그리던 점(stopCaptureTool)과 이전 조사 레이어의 되돌리기 차례도 남기지 않는다.
        self.draw_sketch = crate::draw_sketch::DrawSketch::default();
        self.undo_tables.clear();
        self.redo_tables.clear();
        self.is_dirty = false;
        self.active_tool = MapToolKind::Pan;
        // 조사 데이터 레이어만 제거. 배경지도·지적은 호출부가 다시 보장한다.
        self.left_layers.remove_survey_domain_layers();
        self.right_inspector = crate::right_inspector::RightInspectorState::new_default();
        self.home_map.scene = strata_canvas2d::painter_scene::MapSceneModel::default();
        self.home_map.selected_feature_idx = None;
        // 이전 조사의 고른 도형·방금 그린 도형·겹친 도형 돌기도 남기지 않는다.
        self.home_map.selected_shape = None;
        self.home_map.last_drawn_shape = None;
        self.home_map.pick_cycle = None;
        self.home_map.cad = strata_canvas2d::cad_session::CadSessionController::default();
        self.home_map.real_survey =
            crate::real_survey::RealSurveyData::new_empty_survey();
        self.align = crate::gui_ribbon_bar::AlignSessionState::default();
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = crate::ribbon::SubToolsMode::None;
        self.is_dirty = false;
    }


    pub fn import_demo_dxf_and_csv(&mut self) {
        self.left_layers
            .ensure_domain_layer(DomainLayerKey::ControlPoints);
        self.active_tab = MainTab::Map;
    }
}
