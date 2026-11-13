//! 새 프로젝트 만들기·열기·저장(조사 GPKG·비상 자동 저장).
//!
//! `app_state.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 메서드는 그대로다.
//! `write_survey_to` 는 `app_state` 시험이 직접 부르므로 `pub(crate)` 다 — 크레이트 밖으로는
//! 나가지 않으므로 공개 API 는 그대로다.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use super::app_state_types::{MainTab, ProjectTemplateConfig};

/// C++ `MainWindowSession.cpp:1457-1459` 의 다른 이름 저장 거절 문구 그대로.
const SAVE_AS_SAME_NAME_KO: &str = "같은 이름의 조사 파일 또는 작업공간 사본이 있습니다. 기존 자료를 보존하기 위해 덮어쓰지 않았습니다. 사용하지 않은 이름으로 저장하세요.";

/// C++ `MainWindowSession.cpp:1438-1462` — 고른 경로에서 실제로 쓸 .gpkg 를 정한다.
/// .qgz 면 같은 폴더 같은 이름 .gpkg, .gpkg 로 끝나지 않으면 뒤에 .gpkg 를 붙인다.
/// 열린 조사와 같은 파일이 아닌데 그 .gpkg 나 같은 이름 .qgz 가 있으면 거절한다.
pub(crate) fn resolve_save_as_target(
    picked: &Path,
    current: Option<&Path>,
) -> Result<PathBuf, &'static str> {
    let has_ext = |p: &Path, ext: &str| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    };
    let mut target = if has_ext(picked, "qgz") {
        picked.with_extension("gpkg")
    } else {
        picked.to_path_buf()
    };
    if !has_ext(&target, "gpkg") {
        let mut name = target.file_name().map(OsString::from).unwrap_or_default();
        name.push(".gpkg");
        target.set_file_name(name);
    }
    let same_file = current.is_some_and(|cur| same_path_ignoring_case(cur, &target));
    if !same_file && (target.exists() || target.with_extension("qgz").exists()) {
        return Err(SAVE_AS_SAME_NAME_KO);
    }
    Ok(target)
}

/// C++ 처럼 절대 경로를 대소문자 무시로 비교한다(대상 파일이 아직 없어도 된다).
fn same_path_ignoring_case(a: &Path, b: &Path) -> bool {
    let key = |p: &Path| {
        std::path::absolute(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    key(a) == key(b)
}

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
        if !path.exists() {
            // C++ SurveyStorage.cpp:868-878 은 이때 복구 사본으로 다시 만든다(지시 2E). 그 전까지는
            // 없어진 자리에 아무 파일도 만들지 않고 실패한다.
            self.report_save_failure("원래 조사 파일이 없어 새 조사 파일을 만들지 못했습니다.");
            return false;
        }
        self.write_survey_to(&path)
    }

    /// C++ `MainWindowSession.cpp:1420` `saveProjectAs` — 다른 이름으로 저장.
    /// .qgz 를 고르면 같은 이름의 .gpkg 로 쓴다. 취소하거나 기존 파일이라 거절하면
    /// 아무 것도 바꾸지 않는다.
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
        // C++ :1425-1431 — 조사가 있으면 그 폴더, 없으면 preferredSurveyDir.
        if let Some(dir) = self.survey_path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        } else {
            dialog = dialog.set_directory(self.preferred_survey_dir());
        }
        let Some(picked) = dialog.save_file() else {
            return false;
        };
        let target = match resolve_save_as_target(&picked, self.survey_path.as_deref()) {
            Ok(target) => target,
            Err(msg) => {
                self.home_map.draw_guide_message =
                    Some(format!("다른 이름을 선택해 주세요. {msg}"));
                return false;
            }
        };
        // C++ :1519-1545 — 지금 조사 파일이 있으면 그 사본에, 없으면 새 조사 파일에 편집을 쓴다.
        let current = self.survey_path.clone().filter(|p| p.exists());
        match &current {
            Some(cur) if same_path_ignoring_case(cur, &target) => return self.write_survey_to(cur),
            Some(cur) => {
                if let Err(e) = strata_storage::survey_gpkg_save::copy_survey_snapshot(cur, &target) {
                    self.report_save_failure(&format!("다른 이름으로 저장할 조사 파일을 만들지 못했습니다. {e}"));
                    return false;
                }
            }
            None => {}
        }
        let saved = self.write_survey_to(&target);
        if !saved && current.is_some() {
            // 방금 만든 사본만 지운다 — 지금 조사는 그대로다.
            let _ = fs::remove_file(&target);
        }
        saved
    }

    /// 조사 표의 편집을 `path` 에 저장한다 — C++ persistSurveyWork(`MainWindowSession.cpp:874-1000`).
    /// 파일이 없으면 C++ 처럼 새 조사 파일을 먼저 만든다(:1530-1545). 성공하면 저장된 상태를 다시
    /// 읽고 「*」를 지운다. 실패하면 편집·「*」·원본을 그대로 두고 알린다.
    pub(crate) fn write_survey_to(&mut self, path: &std::path::Path) -> bool {
        use strata_storage::survey_gpkg_save::{save_survey, SaveFailure};

        // 카드에 적던 글도 함께 쓴다(C++ 은 「저장」 단추를 누르면 칸을 떠나며 먼저 쓴다 — Ctrl+S 때도 잃지 않게).
        self.commit_card_typing();
        if !path.exists() {
            if let Err(e) = strata_storage::gpkg::GpkgRepository::create_new_survey(path, self.work_crs) {
                let detail = crate::dlg_new_survey::new_survey_error_text(&e);
                self.report_save_failure(&format!("새 조사 파일을 만들지 못했습니다. {detail}"));
                return false;
            }
        }
        let written = match save_survey(path, &self.survey_tables) {
            Ok(written) => written,
            Err(SaveFailure::Layer { table, .. }) => {
                let title = crate::gui_tab_home::saved_gpkg_layer_title(&table);
                self.report_save_failure(&format!("{title}의 편집을 다음 세대 파일에 쓰지 못했습니다."));
                return false;
            }
            Err(SaveFailure::Message(m)) => {
                self.report_save_failure(&m);
                return false;
            }
        };
        // C++ :963-985 — 저장된 파일(잠겨 있었으면 옆 「-저장.gpkg」)로 맞춘다.
        if let Ok(tables) = strata_storage::survey_gpkg_read::read_survey_tables(&written) {
            self.survey_tables = tables;
        }
        // 저장 안 한 도형은 새 번호를 받는다 — 그 도형을 보이던 카드는 비운다(KaFeatureCard.cpp:249-251).
        self.drop_card_if_gone();
        let name = written.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(stem) = written.file_stem() {
            self.project_title = stem.to_string_lossy().into_owned();
        }
        // C++ :963-964 rememberSurvey(파일 이름) · rememberSurveyDir.
        self.remember_survey(&written, &strata_storage::recent_surveys::complete_base_name(&written.to_string_lossy()));
        self.remember_survey_dir(&written);
        self.survey_path = Some(written);
        self.is_dirty = false;
        self.home_map.draw_guide_message = Some(format!("조사 데이터와 작업 구성을 저장했습니다: {name}"));
        true
    }

    /// C++ `MainWindowSession.cpp:1006-1019` reportSaveFailure — 알림 창은 아직 없어 상태줄에 쓴다.
    fn report_save_failure(&mut self, reason: &str) {
        self.home_map.draw_guide_message = Some(format!(
            "저장 실패 — {reason} 미저장 편집은 그대로 있습니다. 다시 저장하세요."
        ));
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
