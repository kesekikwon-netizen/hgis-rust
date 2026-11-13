//! 홈 「최근 조사」와 마지막 조사 폴더 — C++ `MainWindow::rememberSurvey`(`MainWindowSession.cpp:548-553`),
//! `openRecentSurvey`(:563-592), `preferredSurveyDir`·`rememberSurveyDir`(:1336-1364),
//! `updateTopographicDirectory`(`MainWindowTopographic.cpp:90-100`), `KaStartPage::reload`
//! (`KaStartPage.cpp:263-273`), 「목록에서 제거」(`MainWindow.cpp:1062-1066`).

use std::path::{Path, PathBuf};

use strata_storage::recent_surveys::{self, absolute_file_path, complete_base_name, RecentSurvey};

use crate::app_state::StrataAppState;

/// `QFileInfo(path).absolutePath()` — 앞으로 빗금, 드라이브 맨 위는 「C:/」.
fn absolute_dir(path: &Path) -> String {
    let abs = absolute_file_path(path);
    let Some(k) = abs.rfind('/') else { return String::new() };
    let dir = &abs[..k];
    if dir.ends_with(':') {
        format!("{dir}/")
    } else {
        dir.to_owned()
    }
}

/// `preferredSurveyDir` 의 차례 — 시험할 수 있게 값을 받는다.
pub(crate) fn preferred_survey_dir_from(
    last_dir: &str,
    survey: Option<&Path>,
    last_recent: &str,
    desktop: impl FnOnce() -> PathBuf,
) -> PathBuf {
    // 1) 마지막으로 조사를 저장한 폴더
    if !last_dir.is_empty() && Path::new(last_dir).is_dir() {
        return PathBuf::from(last_dir);
    }
    // 2) 지금 열려 있는 조사의 폴더
    if let Some(here) = survey.map(absolute_dir).filter(|d| Path::new(d).is_dir()) {
        return PathBuf::from(here);
    }
    // 3) 최근 조사 목록의 맨 위
    if !last_recent.is_empty() {
        let dir = absolute_dir(Path::new(last_recent));
        if Path::new(&dir).is_dir() {
            return PathBuf::from(dir);
        }
    }
    // 4) 그래도 없으면 바탕 화면
    desktop()
}

impl StrataAppState {
    /// 실행 파일이 사용자 설정 파일을 붙이고 목록을 읽는다(`main.rs`).
    pub(crate) fn attach_user_settings(&mut self, ini: Option<PathBuf>) {
        self.settings_ini = ini;
        self.reload_recent_surveys();
    }

    /// `--snap`(검증 캡처): 사용자 ini 는 읽기만 한다(지시 S0). `temp_dir` 에 사본을 만들어 붙이므로
    /// 홈 화면은 실제 목록을 보이고, 스냅의 열기·새 조사·저장이 쓰는 최근 조사·마지막 폴더는 사본에만 간다.
    /// 사용자 ini 가 없으면 빈 사본, 사본을 만들지 못하면 아무 ini 도 붙이지 않는다.
    pub(crate) fn attach_user_settings_for_snap(&mut self, ini: Option<PathBuf>, temp_dir: &Path) {
        let copy = temp_dir.join("ka-hgis.ini");
        let ready = std::fs::create_dir_all(temp_dir).is_ok()
            && match ini.filter(|src| src.is_file()) {
                Some(src) => std::fs::copy(&src, &copy).is_ok(),
                None => !copy.exists() || std::fs::remove_file(&copy).is_ok(),
            };
        self.attach_user_settings(ready.then_some(copy));
    }

    /// `KaStartPage::reload` — 없는 조사(USB 를 뺀 경우)도 회색으로 남긴다.
    pub(crate) fn reload_recent_surveys(&mut self) {
        self.recent_surveys = self.settings_ini.as_deref().map(recent_surveys::load_all).unwrap_or_default();
        self.refresh_home_facts();
    }

    /// 「이어서 작업」 — 지금 열 수 있는 첫 조사.
    pub(crate) fn continue_survey(&self) -> Option<&RecentSurvey> {
        self.recent_surveys.iter().find(|it| it.available)
    }

    /// `rememberSurvey` — 목록 맨 앞에 넣고 홈 목록을 다시 읽는다.
    pub(crate) fn remember_survey(&mut self, path: &Path, name: &str) {
        if let Some(ini) = self.settings_ini.as_deref() {
            if let Err(e) = recent_surveys::remember(ini, path, name) {
                eprintln!("[recent] 최근 조사를 기록하지 못했습니다: {e}");
            }
        }
        self.reload_recent_surveys();
    }

    /// `rememberSurveyDir` — `Survey/LastDir` 와 조사 폴더의 「지형도」 저장 위치.
    pub(crate) fn remember_survey_dir(&mut self, path: &Path) {
        let dir = absolute_dir(path);
        if dir.is_empty() || !Path::new(&dir).is_dir() {
            return;
        }
        if let Some(ini) = self.settings_ini.as_deref() {
            if let Err(e) = recent_surveys::set_last_dir(ini, &dir) {
                eprintln!("[recent] 마지막 조사 폴더를 기록하지 못했습니다: {e}");
            }
        }
        self.update_topographic_directory(path);
    }

    /// `updateTopographicDirectory` — 조사 폴더 안 「지형도」.
    fn update_topographic_directory(&mut self, survey: &Path) {
        if !survey.exists() {
            return;
        }
        let folder = PathBuf::from(absolute_dir(survey)).join("지형도");
        if std::fs::create_dir_all(folder).is_err() {
            self.home_map.draw_guide_message = Some("조사 폴더의 지형도 저장 위치를 만들지 못했습니다.".to_owned());
        }
    }

    /// 「목록에서 제거」.
    pub(crate) fn forget_recent_survey(&mut self, path: &str) {
        if let Some(ini) = self.settings_ini.as_deref() {
            if let Err(e) = recent_surveys::forget(ini, path) {
                eprintln!("[recent] 최근 조사에서 빼지 못했습니다: {e}");
            }
        }
        self.reload_recent_surveys();
    }

    /// `preferredSurveyDir` — 새 조사 「저장 폴더」·다른 이름으로 저장이 여는 폴더.
    pub(crate) fn preferred_survey_dir(&self) -> PathBuf {
        let (last, recent) = self
            .settings_ini
            .as_deref()
            .map(|ini| (recent_surveys::last_dir(ini), recent_surveys::last_path(ini)))
            .unwrap_or_default();
        preferred_survey_dir_from(&last, self.survey_path.as_deref(), &recent, crate::user_settings::resolved_desktop_path)
    }

    /// `openRecentSurvey` — 홈 목록 줄·「이어서 열기」. 없는 조사는 알리고 목록에 그대로 둔다
    /// (USB·네트워크 드라이브가 빠졌을 수 있다). C++ 은 경고 창, 여기서는 상태줄에 같은 글을 쓴다.
    pub(crate) fn open_recent_survey(&mut self, path: &str) {
        let p = Path::new(path);
        if path.is_empty() || !p.exists() {
            self.home_map.draw_guide_message = Some(format!(
                "조사 파일을 찾을 수 없습니다 — {}. USB·네트워크 드라이브라면 연결한 뒤 다시 누르세요. \
                 목록에서 빼려면 홈의 최근 조사에서 오른쪽 클릭 → 「목록에서 제거」를 고르세요.",
                path.replace('/', "\\")
            ));
            self.reload_recent_surveys();
            return;
        }
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gpkg")) {
            if self.open_survey_gpkg(p) {
                self.remember_survey(p, &complete_base_name(path));
            }
            return;
        }
        // :584-591 — .qgz 옆에 같은 이름 .gpkg 가 있으면 그것을 열고 .qgz 를 기억한다.
        let companion = p.with_extension("gpkg");
        if companion.exists() {
            if self.open_survey_gpkg(&companion) {
                self.remember_survey(p, &complete_base_name(path));
            }
            return;
        }
        // :593- 작업 파일(.qgz)만 있는 조사는 작업공간 복원(지시 2D) 전까지 열지 못한다(열기와 같은 안내).
        self.open_survey_gpkg(p);
    }
}

#[cfg(test)]
#[path = "app_state_recent_tests.rs"]
mod tests;
