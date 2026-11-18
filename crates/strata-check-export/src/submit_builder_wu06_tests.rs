//! WU-06 — checklist blocking and a composed sheet, before any folder is written.
//! `ExportService.cpp:88-90` and the R2 refusal for a sheet PDF this unit does not write
//! (`ExportService.cpp:198-200`, docs/RUST_PLAN.md R2).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_engine::SubmitEncoding;

use super::{export_submission_package, SubmitPackageInputs, BLOCKED, COMPOSED_SHEET};

fn bare(block: bool, composed: bool) -> SubmitPackageInputs<'static> {
    SubmitPackageInputs {
        sources: &[],
        encoding: SubmitEncoding::Utf8,
        survey_name: "demo",
        survey_path: None,
        unsaved_edits_included: false,
        checklist_summary: "- [error] 기준점이 2개 미만입니다\n",
        checklist_errors_remain: true,
        reference_lines: &[],
        app_line: "Strata 2.0.0 (b036c0905f6a)",
        block_on_checklist_errors: block,
        composed_user_sheet: composed,
    }
}

fn refused(inp: &SubmitPackageInputs<'_>) -> (String, bool) {
    let dir = tempfile::tempdir().expect("temp");
    let out = dir.path().join("pkg");
    let err = export_submission_package(&out, inp, &mut |_, _, _| true).expect_err("refused");
    let left = out.exists();
    let staging = std::fs::read_dir(dir.path()).expect("list").any(|e| {
        e.expect("entry")
            .file_name()
            .to_string_lossy()
            .starts_with(".ka-hgis-export-")
    });
    assert!(!staging, "staging left behind");
    (err, left)
}

/// `ExportService.cpp:88-90` — blocking errors refuse before a folder exists.
#[test]
fn blocking_checklist_errors_refuse_before_a_folder_is_made() {
    let (err, left) = refused(&bare(true, false));
    assert_eq!(err, BLOCKED);
    assert!(!left);
}

/// A composed `user_sheet` is not turned into a package that omits the PDF C++ would write.
#[test]
fn composed_user_sheet_refuses_and_leaves_no_folder() {
    let (err, left) = refused(&bare(false, true));
    assert_eq!(err, COMPOSED_SHEET);
    assert!(!left);
}

/// `ExportService.cpp:96-105` stays ahead of the composed-sheet refusal.
#[test]
fn bad_destination_is_reported_before_the_composed_sheet_refusal() {
    let dir = tempfile::tempdir().expect("temp");
    let out = dir.path().join("pkg");
    std::fs::write(&out, b"x").expect("file");
    let err = export_submission_package(&out, &bare(false, true), &mut |_, _, _| true)
        .expect_err("folder");
    assert!(
        err.contains("제출 폴더가 비어 있지 않거나 사용할 수 없습니다"),
        "{err}"
    );
    assert!(!err.contains("user_sheet"));
    assert_eq!(std::fs::read(&out).expect("bytes"), b"x");
}
