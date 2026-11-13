//! 홈의 조사 사실 기록 — C++ SurveyFacts.cpp 와 같은 키·글. 임시 ini·임시 파일만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;

/// SurveyFacts.cpp:39-45 keyFor — 대소문자를 접은 깨끗한 경로의 SHA-1 앞 16자(사용자 ini 의 실제 키와 같음).
#[test]
fn key_is_sha1_of_the_case_folded_clean_path() {
    assert_eq!(key_for("C:/Users/권을/Documents/경주시/경주시.gpkg"), "0b2aeb2b959bb5e9");
    assert_eq!(key_for(r"c:\Users\권을\Documents\경주시\.\경주시.gpkg"), "0b2aeb2b959bb5e9");
    assert_eq!(key_for("  "), "");
}

/// 글 10칸(Qt \t), 옛 9칸 기록도 읽고, 파일 크기·수정 시각이 같을 때만 안다.
#[test]
fn record_format_and_fingerprint() {
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = dir.path().join("a.gpkg");
    std::fs::write(&survey, b"0123456789").expect("file");
    let path = survey.to_string_lossy().into_owned();
    let (size, mtime) = fingerprint(&path).expect("fingerprint");
    let key = key_for(&path);
    let ini = dir.path().join("ka-hgis.ini");
    std::fs::write(&ini, format!("[RecentSurveys]\r\nfacts\\{key}={size}\\t{mtime}\\t1\\t0\\t-1\\t-1\\t0\\t17\\t0\\t\r\n")).expect("ini");
    let f = lookup(&ini, &path);
    assert!(f.known && f.has_counts() && !f.has_check());
    assert_eq!((f.areas, f.features, f.work_crs.as_str()), (1, 0, ""));
    std::fs::write(&survey, b"changed!!!!!!").expect("touch");
    assert!(!lookup(&ini, &path).known, "a changed file forgets its facts");
}

/// rememberCounts: 다른 칸은 두고 개수·시각·좌표계만, 순서 목록 맨 앞, 24개 넘으면 오래된 것을 지운다.
#[test]
fn remember_counts_writes_the_cpp_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let mut paths = Vec::new();
    for i in 0..25 {
        let p = dir.path().join(format!("s{i}.gpkg"));
        std::fs::write(&p, b"x").expect("file");
        paths.push(p.to_string_lossy().into_owned());
        remember_counts(&ini, &paths[i], i as i32, 2, "EPSG:5187").expect("write");
    }
    let file = crate::qt_ini::IniFile::read(&ini).expect("read");
    let order = crate::qt_ini::read_string_list(file.value("RecentSurveys", "factsOrder").expect("order"));
    assert_eq!(order.len(), 24);
    assert_eq!(order[0], key_for(&paths[24]));
    assert!(file.value("RecentSurveys", &format!("facts\\{}", key_for(&paths[0]))).is_none(), "the oldest is removed");
    let raw = file.value("RecentSurveys", &format!("facts\\{}", key_for(&paths[24]))).expect("line");
    let fields: Vec<&str> = raw.split("\\t").collect();
    assert_eq!(fields.len(), 10);
    assert_eq!((fields[2], fields[3], fields[4], fields[9]), ("24", "2", "-1", "EPSG:5187"));
    let f = lookup(&ini, &paths[24]);
    assert_eq!((f.areas, f.features, f.work_crs.as_str()), (24, 2, "EPSG:5187"));
}

/// 홈 칸 글(KaHomeTitleBlock::crsText·setSurvey, KaHomeRecentCard::plateCells).
#[test]
fn cell_texts() {
    assert_eq!(crs_text(""), "—");
    assert_eq!(crs_text("EPSG:5187"), "5187 동부원점");
    assert_eq!(crs_text("EPSG:5186"), "5186 중부원점");
    assert_eq!(crs_text("EPSG:5179"), "5179");
    let mut f = Facts { known: true, areas: 1, features: 0, ..Facts::default() };
    assert_eq!(check_text(&f), "검수 전");
    f.checked_at_ms = 5;
    (f.errors, f.warnings) = (0, 0);
    assert_eq!(check_text(&f), "통과");
    (f.errors, f.warnings) = (2, 1);
    assert_eq!(check_text(&f), "오류 2 · 주의 1");
    let _ = Path::new("");
}
