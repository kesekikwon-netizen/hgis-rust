//! C++ 앱과 함께 쓰는 사용자 설정 파일 위치와 Windows 알려진 폴더(바탕 화면·문서·내려받기).
//!
//! C++ `RecentSurveys::userSettings()` 는 `QSettings(IniFormat, UserScope, "ka-hgis", "ka-hgis")`
//! 이고, `QgsApplication::initQgis` 가 QSettings 위치를 QGIS 프로필 폴더로 옮긴다(Qt 6.11.1·
//! qgis-dev 로 확인). 그래서 파일은 `%APPDATA%\ka-hgis\ka-hgis\profiles\default\ka-hgis\ka-hgis.ini`
//! 이다. `KA_HGIS_SETTINGS_DIR`(C++ `KaPortableRuntime::isolateUserState`, 시험 실행용)가 있으면
//! `QGIS_CUSTOM_CONFIG_PATH = <폴더>\qgis-profile` 이 되어 `<폴더>\qgis-profile\profiles\default\
//! ka-hgis\ka-hgis.ini` 를 쓴다. 포터블 묶음(실행 파일 옆 config)은 아직 없다.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const PROFILE_TAIL: [&str; 4] = ["profiles", "default", "ka-hgis", "ka-hgis.ini"];
pub(crate) const RUST_SETTINGS_FILE_NAME: &str = "strata-rust.ini";

/// 설정 파일 경로 — 위치를 알 수 없으면 None(최근 조사를 읽지도 쓰지도 않는다).
#[must_use]
pub(crate) fn user_settings_ini() -> Option<PathBuf> {
    let qa = std::env::var("KA_HGIS_SETTINGS_DIR").map(|s| s.trim().to_owned()).unwrap_or_default();
    let root = if qa.is_empty() {
        PathBuf::from(std::env::var_os("APPDATA")?).join("ka-hgis").join("ka-hgis")
    } else {
        PathBuf::from(qa).join("qgis-profile")
    };
    Some(PROFILE_TAIL.iter().fold(root, |p, s| p.join(s)))
}

/// Rust-only settings file (`strata-rust.ini`) in the same folder as `shared_ini` (`ka-hgis.ini`).
#[must_use]
pub(crate) fn rust_settings_ini_from_shared(shared_ini: &Path) -> Option<PathBuf> {
    Some(shared_ini.parent()?.join(RUST_SETTINGS_FILE_NAME))
}

fn clean(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

/// `QDir::homePath()`.
#[must_use]
pub(crate) fn home_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

/// C++ `MainWindow::resolvedDesktopPath`(`MainWindowFiles.cpp:216-235`) — 알려진 폴더 Desktop
/// (OneDrive 로 옮겨진 바탕 화면도 따른다), 없으면 흔한 자리들, 그래도 없으면 홈 폴더.
#[must_use]
pub(crate) fn resolved_desktop_path() -> PathBuf {
    static CACHED: OnceLock<PathBuf> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            let home = home_path();
            let candidates = [
                dirs::desktop_dir(),
                Some(home.join("Desktop")),
                Some(home.join("OneDrive").join("Desktop")),
                Some(home.join("OneDrive").join("바탕 화면")),
            ];
            candidates.into_iter().flatten().find(|c| c.is_dir()).map_or_else(|| home.clone(), |c| clean(&c))
        })
        .clone()
}

/// `QStandardPaths::DocumentsLocation`.
#[must_use]
pub(crate) fn documents_path() -> Option<PathBuf> {
    dirs::document_dir()
}

/// C++ 파일함 「내려받기」(`KaFileBrowserPanel.cpp:107-113`) — 없으면 홈\Downloads.
#[must_use]
pub(crate) fn downloads_path() -> PathBuf {
    dirs::download_dir().filter(|d| d.is_dir()).unwrap_or_else(|| home_path().join("Downloads"))
}
