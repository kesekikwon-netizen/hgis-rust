//! What the app does with the outcome of the work on received sheets: the legend row 「수치지형도」, the
//! gauge line and the status line (`target/cpp-ref/src/app/KaTopographicImportDialog.cpp:871-995`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use strata_topo::merge::MergeStop;
use strata_topo::post::Posted;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::ref_vector::data::Look;

fn app_with_job() -> (StrataAppState, crate::download_jobs::DownloadJobHandle) {
    let mut app = StrataAppState::new_startup();
    let (_, job) = app.download_jobs.add_job("수치지형도", DownloadJobState::Running { fraction: None, stage: Some("[6/6] 지도에 올리기".to_owned()) }, 0.0);
    (app, job)
}

fn state_of(app: &mut StrataAppState) -> DownloadJobState {
    app.download_jobs.tick(1.0);
    app.download_jobs.jobs()[0].state.clone()
}

fn posted(merged: Option<&str>, placed: &[&str], review: &[(&str, &str)]) -> Posted {
    Posted { merged: merged.map(PathBuf::from), lines: if merged.is_some() { 61_650 } else { 0 }, placed: placed.iter().map(|n| (*n).to_owned()).collect(), review: review.iter().map(|(n, r)| ((*n).to_owned(), (*r).to_owned())).collect(), warnings: Vec::new() }
}

/// The merged file becomes the legend row 「수치지형도」 under 참조 지도: grey lines of 0.2 mm, no labels
/// (`:936-975`); the gauge ends with the words of the plan and the status line says how many sheets.
#[test]
fn the_merged_sheets_become_the_legend_row() {
    let (mut app, job) = app_with_job();
    app.finish_topographic_post(&job, Ok(posted(Some("C:/survey/지형도/합침/수치지형도-합침-1.gpkg"), &["359012", "359021", "359014"], &[])));
    let row = app.left_layers.entries.iter().find(|e| e.layer_key == "ref_topo").expect("row");
    assert_eq!((row.label_ko.as_str(), row.group), ("수치지형도", LegendGroup::ReferenceMap));
    let spec = app.ref_vectors.spec_of("ref_topo").expect("layer");
    assert_eq!((spec.file.clone(), spec.table.as_str(), spec.work_epsg), (PathBuf::from("C:/survey/지형도/합침/수치지형도-합침-1.gpkg"), "topo", 5187));
    assert_eq!((spec.look.clone(), spec.label), (Look::Lines { stroke: [128, 128, 128, 255], width_mm: 0.2 }, None));
    assert_eq!(state_of(&mut app), DownloadJobState::Done { result: "수치지형도를 준비했습니다.".to_owned() });
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("수치지형도를 준비했습니다. 도엽 3장을 지도에 올렸습니다."));
}

/// A sheet that could not be placed is named with its reason, in the C++ form 「<도엽> · 검토 필요: …」
/// (`KaTopographicScopePanel.cpp:247-249`), beside the sheets that are on the map.
#[test]
fn a_sheet_kept_for_review_is_named_with_its_reason() {
    let (mut app, job) = app_with_job();
    app.finish_topographic_post(&job, Ok(posted(Some("C:/survey/지형도/합침/수치지형도-합침-1.gpkg"), &["359012"], &[("359023", "원본 좌표가 공식 도엽 위치와 일치하지 않습니다.")])));
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "ref_topo"));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("수치지형도를 준비했습니다. 도엽 1장을 지도에 올렸습니다. 359023 · 검토 필요: 원본 좌표가 공식 도엽 위치와 일치하지 않습니다."));

    // Nothing for the map and a sheet for review: the gauge stops with the reason and its code.
    let (mut app, job) = app_with_job();
    app.finish_topographic_post(&job, Ok(posted(None, &[], &[("359023", "원본 좌표가 공식 도엽 위치와 일치하지 않습니다.")])));
    assert!(!app.left_layers.entries.iter().any(|e| e.layer_key == "ref_topo"));
    assert_eq!(state_of(&mut app), DownloadJobState::Stopped { code: StopCode::Data1, reason: "359023 · 검토 필요: 원본 좌표가 공식 도엽 위치와 일치하지 않습니다.".to_owned() });
}

/// Sheets without a line near the survey, a stop, a failure: no row, and the gauge says which it was.
#[test]
fn without_a_map_the_gauge_says_why() {
    let (mut app, job) = app_with_job();
    app.finish_topographic_post(&job, Ok(posted(None, &["359012"], &[])));
    assert_eq!(state_of(&mut app), DownloadJobState::Done { result: "조사구역 5km 안에 지도에 올릴 수치지형도 선이 없습니다.".to_owned() });

    let (mut app, job) = app_with_job();
    app.finish_topographic_post(&job, Err(MergeStop::Cancelled));
    assert_eq!(state_of(&mut app), DownloadJobState::Cancelled);

    let (mut app, job) = app_with_job();
    app.finish_topographic_post(&job, Err(MergeStop::Failed("받은 수치지형도 레이어를 열지 못했습니다: 359012. 기존 지도와 받은 원본은 유지됩니다.".to_owned())));
    assert_eq!(state_of(&mut app), DownloadJobState::Stopped { code: StopCode::Data1, reason: "받은 수치지형도 레이어를 열지 못했습니다: 359012. 기존 지도와 받은 원본은 유지됩니다.".to_owned() });
    assert!(!app.left_layers.entries.iter().any(|e| e.layer_key == "ref_topo"));
}

/// Owner 2026-11-15: 「맵을받아 레이어에 추가되는맵이면 받은것을 같은개발일때 그다음에 넣을때 적용되게하라」; MORE0 M6-2.
/// With sheets the survey has merged already and no row on the map, 「수치지형」 puts them on the map at once,
/// without the portal, the internet or the account; nothing is ordered.
#[test]
fn a_press_puts_the_sheets_the_survey_has_on_the_map_without_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let merged = dir.path().join("지형도").join(strata_topo::post::MERGED_FOLDER);
    std::fs::create_dir_all(&merged).unwrap();
    std::fs::write(merged.join(format!("{}100.gpkg", strata_topo::post::MERGED_PREFIX)), b"old").unwrap();
    std::fs::write(merged.join(format!("{}200.gpkg", strata_topo::post::MERGED_PREFIX)), b"new").unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    app.handle_ribbon_click("수치지형");
    let spec = app.ref_vectors.spec_of("ref_topo").expect("the row of the sheets the survey has");
    assert_eq!(spec.file, merged.join(format!("{}200.gpkg", strata_topo::post::MERGED_PREFIX)), "the newest merged file");
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "ref_topo"));
    assert!(!app.topographic_download_busy(), "nothing is ordered");
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("받아 둔 수치지형도를 지도에 올렸습니다."));
}
