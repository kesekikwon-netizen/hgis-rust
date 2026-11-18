//! Field error box words — C++ `KaUserError` (`target/cpp-ref/src/app/KaUserError.cpp:34-55`,
//! `KaUserError.h:14-40`). The folder 「로그 폴더 열기」 opens is `KaSessionLog::dir`
//! (`target/cpp-ref/src/core/KaSessionLog.cpp:39-80`).

use std::path::{Path, PathBuf};

/// One field error (`KaUserError::Spec`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldError {
    pub(crate) title: String,
    pub(crate) what: String,
    pub(crate) why: String,
    pub(crate) how: String,
    /// Empty means 닫기 only (`Spec::actionLabel`).
    pub(crate) action_label: String,
    /// Folded under 「자세히」 (`Spec::details`, Qt's detailed text).
    pub(crate) details: String,
    pub(crate) critical: bool,
    pub(crate) details_open: bool,
}

impl FieldError {
    pub(crate) fn warn(title: &str, what: &str, why: &str, how: &str) -> Self {
        Self {
            title: title.to_owned(),
            what: what.to_owned(),
            why: why.to_owned(),
            how: how.to_owned(),
            action_label: String::new(),
            details: String::new(),
            critical: false,
            details_open: false,
        }
    }
}

/// `formatBody` (`KaUserError.cpp:34-41`). The window paints the same headings; tests lock this string.
#[cfg(test)]
#[must_use]
pub(crate) fn format_body(spec: &FieldError) -> String {
    let mut parts = vec!["무엇이 안 됐나".to_owned(), spec.what.trim().to_owned()];
    if !spec.why.trim().is_empty() {
        parts.push(String::new());
        parts.push("왜 그런가".to_owned());
        parts.push(spec.why.trim().to_owned());
    }
    if !spec.how.trim().is_empty() {
        parts.push(String::new());
        parts.push("어떻게 하나".to_owned());
        parts.push(spec.how.trim().to_owned());
    }
    parts.join("\n")
}

#[cfg(test)]
fn html_paragraph(text: &str) -> String {
    text.trim()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "<br>")
}

/// `formatBodyHtml` (`KaUserError.cpp:44-54`).
#[cfg(test)]
#[must_use]
pub(crate) fn format_body_html(spec: &FieldError) -> String {
    let mut html = String::new();
    let mut section = |heading: &str, text: &str, always: bool| {
        if !always && text.trim().is_empty() {
            return;
        }
        html.push_str(&format!(
            "<p style=\"margin:0 0 10px 0\"><b>{heading}</b><br>{}</p>",
            html_paragraph(text)
        ));
    };
    section("무엇이 안 됐나", &spec.what, true);
    section("왜 그런가", &spec.why, false);
    section("어떻게 하나", &spec.how, false);
    html
}

/// `QDir::fromNativeSeparators` for the override, then `QDir::toNativeSeparators` for the tooltip.
#[must_use]
pub(crate) fn native_separators(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.replace('\\', "/")
    }
}

/// Tooltip of 「로그 폴더 열기」 (`KaUserError.cpp:91-92`).
#[must_use]
pub(crate) fn log_button_tooltip(folder: &str) -> String {
    format!(
        "개발자에게 보낼 session.log가 있는 폴더를 엽니다: {}",
        native_separators(folder)
    )
}

/// `KaSessionLog::dir` with the inputs passed in, so a test does not touch the process environment.
#[must_use]
pub(crate) fn session_log_dir_from(
    overridden: &str,
    exe_dir: &Path,
    local_app_data: &str,
    temp: &Path,
) -> PathBuf {
    let overridden = overridden.trim();
    if !overridden.is_empty() {
        return PathBuf::from(overridden.replace('\\', "/"));
    }
    // `portableLogDirFor` (`KaSessionLog.cpp:65-75`): a bundle that carries `apps/qgis-dev`.
    if exe_dir.join("apps").join("qgis-dev").is_dir() {
        return exe_dir.join("config").join("logs");
    }
    // `localAppDataLogDir` (`:39-43`): `%LOCALAPPDATA%/ka-hgis/logs`, else the temp directory.
    let base = if local_app_data.trim().is_empty() {
        temp.to_path_buf()
    } else {
        PathBuf::from(local_app_data.trim())
    };
    base.join("ka-hgis").join("logs")
}

/// The folder the running app opens (`KaSessionLog::dir`, `KaUserError.cpp:87`).
#[must_use]
pub(crate) fn session_log_dir() -> PathBuf {
    let overridden = std::env::var("KA_HGIS_LOG_DIR").unwrap_or_default();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    session_log_dir_from(&overridden, &exe_dir, &local, &std::env::temp_dir())
}

/// `QDesktopServices::openUrl` of the folder (`KaUserError.cpp:19-21`), after `mkpath`.
pub(crate) fn open_log_folder(folder: &Path) -> bool {
    if std::fs::create_dir_all(folder).is_err() {
        return false;
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(folder)
            .spawn()
            .is_ok()
    }
    #[cfg(not(windows))]
    {
        // Linux is out of scope. Tests record the folder instead of opening a window.
        let _ = folder;
        false
    }
}

#[cfg(test)]
#[path = "user_error_tests.rs"]
mod tests;
