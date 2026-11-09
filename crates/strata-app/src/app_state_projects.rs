//! 새 프로젝트 만들기·열기·저장(조사 GPKG·비상 자동 저장).
//!
//! `app_state.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 메서드는 그대로다.
//! `write_survey_to` 는 `app_state` 시험이 직접 부르므로 `pub(crate)` 다 — 크레이트 밖으로는
//! 나가지 않으므로 공개 API 는 그대로다.

use std::fs;
use std::path::Path;

use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use super::app_state_types::{MainTab, ProjectTemplateConfig};

impl StrataAppState {
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
        let layer_codes: Vec<&str> = config.default_layers.iter().map(|l| l.id()).collect();
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

    /// C++ `MainWindowSession.cpp:1409` `saveProject` — 지금 조사를 파일에 쓴다.
    /// 경로가 없으면 「다른 이름으로 저장」으로 넘긴다 (C++ 과 같다).
    pub fn save_survey_project(&mut self) -> bool {
        let Some(path) = self.survey_path.clone() else {
            return self.save_survey_project_as();
        };
        self.write_survey_to(&path)
    }

    /// C++ `MainWindowSession.cpp:1420` `saveProjectAs` — 다른 이름으로 저장.
    /// .qgz 를 고르면 같은 이름의 .gpkg 로 쓴다. 취소하면 아무 것도 바꾸지 않는다.
    pub fn save_survey_project_as(&mut self) -> bool {
        let default_name = if let Some(cur) = self.survey_path.as_ref() {
            let stem = cur
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("조사");
            format!("{stem}_복사본.gpkg")
        } else {
            "새조사.gpkg".to_owned()
        };
        let mut dialog = rfd::FileDialog::new()
            .set_title("다른 이름으로 저장")
            .add_filter("고고학 조사 파일", &["gpkg", "qgz"])
            .add_filter("GeoPackage", &["gpkg"])
            .set_file_name(&default_name);
        if let Some(dir) = self.survey_path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(picked) = dialog.save_file() else {
            return false;
        };
        let mut target = picked;
        if target
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("qgz"))
            .unwrap_or(false)
        {
            target = target.with_extension("gpkg");
        } else if !target
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("gpkg"))
            .unwrap_or(false)
        {
            target.set_extension("gpkg");
        }
        if self.write_survey_to(&target) {
            self.survey_path = Some(target);
            true
        } else {
            false
        }
    }

    /// 워크스페이스를 GPKG 로 실제로 쓴다. 성공하면 제목·저장 상태를 갱신한다.
    pub(crate) fn write_survey_to(&mut self, path: &std::path::Path) -> bool {
        use std::collections::BTreeMap;
        use strata_storage::gpkg::{GpkgRepository, SurveyWorkspace};

        let mut ws = SurveyWorkspace {
            schema_version: 3,
            work_crs: self.work_crs,
            layers: BTreeMap::new(),
            embedded_project_json: format!("{{\"title\":{:?}}}", self.project_title),
        };
        for entry in &self.left_layers.entries {
            if let Some(key) = DomainLayerKey::from_id(&entry.layer_key) {
                ws.ensure_domain_layer(key);
            }
        }
        match GpkgRepository::write_workspace_file(path, &ws) {
            Ok(()) => {
                self.project_title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&self.project_title)
                    .to_owned();
                self.is_dirty = false;
                self.home_map.draw_guide_message = Some(format!(
                    "「{}」 조사를 저장했습니다: {}",
                    self.project_title,
                    path.display()
                ));
                true
            }
            Err(e) => {
                self.home_map.draw_guide_message =
                    Some(format!("조사를 저장하지 못했습니다: {e}"));
                false
            }
        }
    }

    /// C++ `CrashHandler::handleCrash` / `emergencyAutoSave` — 비정상 종료 시 현재 열려 있는 조사 버퍼를 `.crash_recovery.gpkg`로 즉시 직렬화 백업 1:1
    pub fn emergency_auto_save(&self, crash_recovery_path: &Path) -> bool {
        use std::collections::BTreeMap;
        use strata_storage::gpkg::{GpkgRepository, SurveyWorkspace};

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
