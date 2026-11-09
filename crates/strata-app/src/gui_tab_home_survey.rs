//! 「홈」 탭에서 여는 조사 — 새 조사(빈 프로젝트)·견본·파일 열기·GPKG 읽기.
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
        // 도메인 레이어는 0개다. 파일에 실제로 들어 있는 레이어만 되살린다.
        let mut restored = 0usize;
        if let Ok(ws) = strata_storage::gpkg::GpkgRepository::open_read_only(path) {
            for key_name in ws.layers.keys() {
                if let Some(key) = DomainLayerKey::from_id(key_name.as_str()) {
                    self.left_layers.ensure_domain_layer(key);
                    restored += 1;
                }
            }
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
    }

    /// `start_new_survey_project` 의 예전 시험·시연용 입구(임시 폴더에 만든다).
    pub fn start_new_survey_project(&mut self, name: &str) {
        let path = std::env::temp_dir().join(format!("{name}.gpkg"));
        let crs = self.modals.new_survey_crs;
        let _ = GpkgRepository::create_new_survey(&path, crs);
        self.start_new_survey_project_with_path(name, &path, crs);
    }

    /// 이전 조사의 지도·레이어·편집 상태를 모두 비운다(C++ `kaSafeClearQgisProject` 자리).
    /// 조사 파일 자체는 건드리지 않는다 — 저장하지 않은 편집은 호출자가 먼저 확인한다.
    ///
    /// C++ 은 `removeSurveyDomainLayers` 로 **조사 데이터 레이어만** 지우고 배경지도는
    /// `ensureDefaultBasemaps` 가 다시 맞춘다. 여기서도 패널을 통째로 비우지 않고
    /// 조사 데이터 그룹만 지운다(사용자 지적 2026-11-09: 레이어가 남아 있었다).
    fn reset_survey_workspace(&mut self, name: &str) {
        self.project_title = name.to_owned();
        self.survey_path = None;
        self.is_dirty = false;
        self.active_tool = MapToolKind::Pan;
        // 조사 데이터 레이어만 제거. 배경지도·지적은 호출부가 다시 보장한다.
        self.left_layers.remove_survey_domain_layers();
        self.right_inspector = crate::right_inspector::RightInspectorState::new_default();
        self.home_map.scene = strata_canvas2d::painter_scene::MapSceneModel::default();
        self.home_map.selected_feature_idx = None;
        self.home_map.cad = strata_canvas2d::cad_session::CadSessionController::default();
        self.home_map.real_survey =
            crate::real_survey::RealSurveyData::new_empty_survey(&std::env::temp_dir());
        self.align = crate::gui_ribbon_bar::AlignSessionState::default();
        self.ribbon.sub_toolbar_visible = false;
        self.ribbon.sub_mode = crate::ribbon::SubToolsMode::None;
        self.is_dirty = false;
    }

    /// C++ `MainWindowSession.cpp:1634-1680` `openProject` — 「열기」.
    /// 파일 대화상자로 조사 파일(.gpkg)을 고르면 실제로 읽어 워크스페이스를 되살린다.
    /// 취소하면 아무 것도 바꾸지 않는다. 견본(idx) 경로는 테스트·시연용으로 남긴다.
    pub fn open_sample_survey_project(&mut self, idx: usize) {
        const SAMPLE_IDX: usize = usize::MAX;
        if idx != SAMPLE_IDX {
            // 견본 열기 (기존 동작 유지)
            self.project_title = "경주시".to_owned();
            self.left_layers
                .ensure_domain_layer(DomainLayerKey::SurveyArea);
            self.left_layers.apply_default_basemaps();
            self.home_map.fit_to_survey();
            self.active_tab = MainTab::Map;
            self.home_map.draw_guide_message =
                Some("다 그렸으면 리본의 「도면」으로 종이에 옮기세요.".to_owned());
            return;
        }
        self.open_survey_file_dialog();
    }

    /// C++ `openProject` 의 파일 선택 + `openSurveyGpkg` 에 해당한다.
    pub fn open_survey_file_dialog(&mut self) {
        // C++ QFileDialog 필터: 조사 (*.gpkg *.qgz *.qgs);; GeoPackage (*.gpkg)
        let picked = ::rfd::FileDialog::new()
            .set_title("열기")
            .add_filter("조사", &["gpkg", "qgz", "qgs"])
            .add_filter("GeoPackage", &["gpkg"])
            .pick_file();
        let Some(path) = picked else {
            // 사용자가 취소했다 — C++ 은 아무 것도 하지 않는다.
            return;
        };
        self.open_survey_gpkg(&path);
    }

    /// C++ `openSurveyGpkg` — 파일을 읽어 조사 제목·좌표계·레이어를 되살린다.
    pub fn open_survey_gpkg(&mut self, path: &std::path::Path) {
        use strata_storage::gpkg::GpkgRepository;

        // C++ 은 .qgz/.qgs 를 고르면 같은 이름의 .gpkg 동반 파일을 먼저 찾는다.
        let mut target = path.to_path_buf();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "gpkg" {
            let companion = path.with_extension("gpkg");
            if companion.exists() {
                target = companion;
            } else {
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」은(는) GeoPackage 조사 파일이 아니고 같은 이름의 .gpkg 도 없습니다.",
                    path.display()
                ));
                return;
            }
        }

        match GpkgRepository::open_read_only(&target) {
            Ok(ws) => {
                // C++ `MainWindowSession.cpp:389` — 열기 전에 프로젝트를 비우고
                // `loadSurveyLayers` 가 `removeSurveyDomainLayers` 로 이전 조사 레이어를
                // 지운다. 같은 자리에서 지도·레이어·조사 상태를 먼저 비운다.
                let recents = self.home_map.recent_files.clone();
                let title = target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("조사")
                    .to_owned();
                self.reset_survey_workspace(&title);
                self.home_map.recent_files = recents;
                self.work_crs = ws.work_crs;

                // 레이어 종류별로 도메인 레이어를 되살린다.
                for key_name in ws.layers.keys() {
                    if let Some(key) = DomainLayerKey::from_id(key_name.as_str()) {
                        self.left_layers.ensure_domain_layer(key);
                    }
                }
                // C++ `ensureDefaultBasemaps` — 열기 직후에도 위성·지적 두 개.
                self.left_layers.apply_default_basemaps();
                self.survey_path = Some(target.clone());
                self.home_map.fit_to_survey();
                self.active_tab = MainTab::Map;
                self.is_dirty = false;
                let recent = target.to_string_lossy().to_string();
                if !self.home_map.recent_files.contains(&recent) {
                    self.home_map.recent_files.insert(0, recent);
                    self.home_map.recent_files.truncate(12);
                }
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」 조사를 열었습니다 ({} · 레이어 {}개).",
                    title,
                    self.work_crs.auth_id(),
                    ws.layers.len()
                ));
            }
            Err(e) => {
                // C++ KaUserError::warn — 오류를 알리고 아무 것도 바꾸지 않는다.
                self.home_map.draw_guide_message =
                    Some(format!("조사 파일을 열 수 없습니다: {e}"));
            }
        }
    }

    pub fn import_demo_dxf_and_csv(&mut self) {
        self.left_layers
            .ensure_domain_layer(DomainLayerKey::ControlPoints);
        self.active_tab = MainTab::Map;
    }
}
