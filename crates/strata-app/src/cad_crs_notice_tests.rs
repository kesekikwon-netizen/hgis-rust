//! `tests/test_cad_hints.cpp` stated cases, and the notice rows from `KaCadImportNotice.cpp:53-59`.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use super::{notice_choice_status, notice_for, CadNoticeKind};
use crate::app_state::StrataAppState;
use crate::cad_crs_stated::{crs_label, stated_auth_ids};
use crate::dialogs_modal::ActiveModalDialog;

fn ids(path: &str, texts: &[&str]) -> Vec<String> {
    stated_auth_ids(
        Path::new(path),
        &texts
            .iter()
            .map(|text| (*text).to_owned())
            .collect::<Vec<_>>(),
    )
}

fn expect(path: &str, texts: &[&str], want: &[&str]) {
    assert_eq!(
        ids(path, texts),
        want.iter()
            .map(|text| (*text).to_owned())
            .collect::<Vec<_>>()
    );
}

/// `test_cad_hints.cpp:22-30` and `:44-54`.
#[test]
fn stated_reads_the_file_name_and_the_header_words() {
    expect("C:/a/(고경면 가수리)면적산출_5187.dxf", &[], &["EPSG:5187"]);
    expect("C:/a/EPSG5174 도면.dxf", &[], &["EPSG:5174"]);
    expect("C:/a/test1.dwg", &[], &[]);
    expect("C:/a/도면51860.dxf", &[], &[]);
    expect("C:/a/산5174.dxf", &[], &[]);
    expect("C:/a/5186-1번지 도면.dxf", &[], &[]);
    let drawing = "C:/a/도면.dxf";
    expect(drawing, &["좌표계: EPSG:5186"], &["EPSG:5186"]);
    expect(
        drawing,
        &["측량 기준 GRS80 동부원점"],
        &["EPSG:5187", "EPSG:5183"],
    );
    expect(
        drawing,
        &["세계측지계 중부 원점"],
        &["EPSG:5186", "EPSG:5181"],
    );
    expect(drawing, &["Bessel 중부원점"], &["EPSG:5174"]);
    expect(
        drawing,
        &["중부원점"],
        &["EPSG:5186", "EPSG:5181", "EPSG:5174"],
    );
    expect(drawing, &["UTM-K"], &["EPSG:5179"]);
    expect(drawing, &["산5174", "374-1답"], &[]);
}

#[test]
fn a_bessel_central_header_offers_5174_and_more_than_the_work_crs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("도면.dxf");
    std::fs::write(&path, "베셀 중부원점").unwrap();
    assert_eq!(crs_label("EPSG:5186"), "GRS80 중부원점(2010)");
    let notice = notice_for(&path, "EPSG:5187", &[]);
    let crs: Vec<_> = notice
        .menu
        .iter()
        .filter(|line| matches!(line.kind, CadNoticeKind::OtherCrs { .. }))
        .collect();
    assert!(crs.len() > 1, "the work CRS is not the only line");
    assert!(!crs.iter().any(|line| line.label.contains("EPSG:5187")));
    assert_eq!(crs[0].label, "베셀 중부원점 보정 (EPSG:5174)");
    assert!(crs
        .iter()
        .any(|line| line.label.contains("GRS80 중부원점(2010)")));
    assert_eq!(
        notice.menu.last().map(|line| line.label.as_str()),
        Some("좌표 없는 도면으로 보기")
    );
    assert_eq!(notice.align_label, "직접 맞추기");

    let mut app = StrataAppState::new_startup();
    app.drop_paths(std::slice::from_ref(&path));
    assert_eq!(
        app.status_left_message(),
        "CAD 도면(DXF·DWG)은 아직 지도에 올리지 못합니다: 도면.dxf"
    );
    assert!(matches!(
        app.modals.active,
        Some(ActiveModalDialog::CadImportNotice)
    ));
    let offered = CadNoticeKind::OtherCrs {
        auth_id: "EPSG:5174".to_owned(),
    };
    let chosen = notice_choice_status("도면.dxf", &offered);
    let align = notice_choice_status("도면.dxf", &CadNoticeKind::Align);
    assert_eq!(
        chosen,
        "CAD 도면(DXF·DWG)은 아직 지도에 올리지 못합니다: 도면.dxf"
    );
    assert_eq!(
        align,
        "이 도면이 지도에 없습니다. 도면을 다시 불러와 주세요."
    );
    assert!(!chosen.contains("올렸습니다") && !align.contains("올렸습니다"));
    assert!(!chosen.contains("추가됨") && !align.contains("추가됨"));
    app.apply_cad_notice(&offered);
    assert!(app.modals.active.is_none());
    assert_eq!(app.status_left_message(), chosen);
}

#[test]
fn a_recent_crs_leads_when_the_header_states_nothing() {
    let notice = notice_for(
        Path::new("C:/a/도면.dxf"),
        "EPSG:5187",
        &["EPSG:2097".to_owned()],
    );
    let first = notice
        .menu
        .iter()
        .find(|line| matches!(line.kind, CadNoticeKind::OtherCrs { .. }));
    assert_eq!(
        first.map(|line| line.label.as_str()),
        Some("베셀 중부원점 (EPSG:2097)")
    );
}
