//! `formatBody` / `formatBodyHtml` / the log folder — `target/cpp-ref/tests/test_user_error.cpp:51-119`
//! and `KaSessionLog.cpp:39-80`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::{format_body, format_body_html, session_log_dir_from, FieldError};

fn sample() -> FieldError {
    let mut spec = FieldError::warn(
        "지적도 받기",
        "지적도를 받지 못했습니다.",
        "서버가 <오류>를 돌려주었습니다.",
        "잠시 뒤 다시 받으세요.",
    );
    spec.details = "HTTP 503 Service Unavailable".to_owned();
    spec
}

#[test]
fn format_body_contains_what_why_how() {
    let spec = FieldError::warn(
        "지적도 받기",
        "지적도를 받지 못했습니다.",
        "아이디나 비밀번호가 맞지 않습니다.",
        "아이디·비밀번호를 다시 입력한 뒤 받기를 누르세요.",
    );
    let body = format_body(&spec);
    assert!(body.contains("무엇이 안 됐나"));
    assert!(body.contains(&spec.what));
    assert!(body.contains("왜 그런가"));
    assert!(body.contains(&spec.why));
    assert!(body.contains("어떻게 하나"));
    assert!(body.contains(&spec.how));
}

#[test]
fn format_body_html_bolds_headings_and_escapes_text() {
    let html = format_body_html(&sample());
    assert!(html.contains("<b>무엇이 안 됐나</b>"));
    assert!(html.contains("<b>왜 그런가</b>"));
    assert!(html.contains("<b>어떻게 하나</b>"));
    assert!(html.contains("&lt;오류&gt;"));
    assert!(!html.contains("<오류>"));
    assert!(format_body(&sample()).contains("<오류>"));
}

#[test]
fn an_empty_why_is_left_out() {
    let spec = FieldError::warn("제목", "실패했습니다.", "", "다시 하세요.");
    let body = format_body(&spec);
    assert!(!body.contains("왜 그런가"));
    assert!(body.contains("어떻게 하나"));
}

#[test]
fn the_log_folder_follows_the_cpp_order() {
    let temp = Path::new("/tmp/ka-temp");
    let exe = Path::new("/opt/strata");
    assert_eq!(
        session_log_dir_from(r"D:\logs\session", exe, r"C:\Users\me\AppData\Local", temp),
        Path::new("D:/logs/session")
    );
    assert_eq!(
        session_log_dir_from("", exe, r"C:\Users\me\AppData\Local", temp),
        Path::new(r"C:\Users\me\AppData\Local")
            .join("ka-hgis")
            .join("logs")
    );
    assert_eq!(
        session_log_dir_from("", exe, "", temp),
        temp.join("ka-hgis").join("logs")
    );
}

#[test]
fn a_portable_bundle_keeps_logs_beside_the_exe() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("apps/qgis-dev")).unwrap();
    let got = session_log_dir_from("", dir.path(), "/unused", Path::new("/tmp"));
    assert_eq!(got, dir.path().join("config/logs"));
}
