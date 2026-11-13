//! R2-2 — the submit package equals the cpp package (docs/senior/ref-files/cpp-submit-package-demo): the 35 SHP
//! files and encoding.txt byte for byte (DBF bytes 1..3 masked), README_submit.txt line by line except
//! `created:`, MANIFEST.sha256 lines of the equal files; destination checks, cancel, nothing to submit.
//! Ported from target/cpp-ref/tests/test_submit_package.cpp (cancelLeavesNoPackageAndNoStaging,
//! blockedAndManifestMessagesAreKorean, packageWithChecklistErrorsSaysSoInReadme and the README part of
//! mergeKeepsFieldsOfEveryLayerAndReadmeRecordsProvenance). Tempdir copies only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use strata_engine::{SubmitEncoding, SubmitSource, SUBMIT_KEYS};

use super::{export_submission_package, SubmitPackageInputs, CANCELED};
use crate::submit_readme::{build_readme, native, system_time_zone_id, write_sha256_manifest, ReadmeInputs};

const APP_LINE: &str = "Strata 2.0.0 (b036c0905f6a)";
/// sha256 of the tracked samples/demo_survey/demo.gpkg = the cpp README value.
const DEMO_SHA256: &str = "082674951f6233c5800dedcc4e954faca6fba6d5bece101a69b6b26178a0b74f";

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn demo_copy(dir: &Path) -> PathBuf {
    let gpkg = dir.join("demo.gpkg");
    std::fs::copy(manifest("../../samples/demo_survey/demo.gpkg"), &gpkg).expect("copy demo");
    gpkg
}

fn demo_sources(gpkg: &Path) -> Vec<(&'static str, Vec<SubmitSource>)> {
    SUBMIT_KEYS
        .iter()
        .map(|k| (*k, vec![SubmitSource { gpkg: gpkg.to_path_buf(), table: (*k).to_owned(), title: (*k).to_owned() }]))
        .collect()
}

fn inputs<'a>(sources: &'a [(&'static str, Vec<SubmitSource>)], survey: &'a Path) -> SubmitPackageInputs<'a> {
    SubmitPackageInputs {
        sources,
        encoding: SubmitEncoding::Utf8,
        survey_name: "demo",
        survey_path: Some(survey),
        unsaved_edits_included: false,
        checklist_summary: "",
        checklist_errors_remain: false,
        reference_lines: &[],
        app_line: APP_LINE,
    }
}

fn staging_dirs(parent: &Path) -> Vec<String> {
    std::fs::read_dir(parent)
        .expect("list")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".ka-hgis-export-"))
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Lines of a text-mode file; fails when a line end is not `\r\n` (Windows text mode).
fn crlf_lines(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8(bytes.to_vec()).expect("utf-8");
    let body = text.strip_suffix("\r\n").expect("ends with CRLF");
    assert!(!body.replace("\r\n", "").contains('\n'), "a bare LF");
    body.split("\r\n").map(str::to_owned).collect()
}

#[test]
fn demo_package_equals_the_cpp_package() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let sources = demo_sources(&gpkg);
    let out = dir.path().join("pkg");
    let mut calls: Vec<(usize, usize, String)> = Vec::new();
    let mut progress = |done: usize, total: usize, label: &str| {
        calls.push((done, total, label.to_owned()));
        true
    };
    let got = export_submission_package(&out, &inputs(&sources, &gpkg), &mut progress).expect("package");
    assert_eq!(got, out);
    // ExportService.cpp:125-131, :240 — the steps and their labels.
    let mut want: Vec<String> = SUBMIT_KEYS.iter().map(|k| format!("{k}.shp")).collect();
    want.extend(["encoding.txt", "조사도면.pdf", "조사 파일 지문", "MANIFEST.sha256", "확정"].map(str::to_owned));
    let mut want: Vec<(usize, usize, String)> = want.into_iter().enumerate().map(|(i, l)| (i, 12, l)).collect();
    want.push((12, 12, "완료".to_owned()));
    assert_eq!(calls, want);
    assert!(staging_dirs(dir.path()).is_empty(), "staging left behind");

    let shp = manifest("../strata-engine/tests/fixtures/cpp_submit_demo");
    let pkg = manifest("tests/fixtures/cpp_submit_demo_package");
    let mut written: Vec<String> =
        std::fs::read_dir(&out).expect("list").map(|e| e.expect("entry").file_name().to_string_lossy().into_owned()).collect();
    written.sort();
    assert_eq!(written.len(), 38, "{written:?}");
    let mut diffs = Vec::new();
    for key in SUBMIT_KEYS {
        for ext in ["shp", "shx", "dbf", "prj", "cpg"] {
            let file = format!("{key}.{ext}");
            let mut a = std::fs::read(out.join(&file)).expect(&file);
            let mut b = std::fs::read(shp.join(&file)).expect(&file);
            if ext == "dbf" {
                a[1..4].fill(0);
                b[1..4].fill(0);
            }
            if a != b {
                diffs.push(file);
            }
        }
    }
    assert!(diffs.is_empty(), "differ from the cpp package: {diffs:?}");
    let enc = std::fs::read(out.join("encoding.txt")).expect("encoding.txt");
    assert_eq!(enc, std::fs::read(pkg.join("encoding.txt")).expect("cpp encoding.txt"), "LF, no CRLF");

    // README: every line equal except created (local time); timezone is this machine's IANA id.
    let ours = crlf_lines(&std::fs::read(out.join("README_submit.txt")).expect("README"));
    let cpp = crlf_lines(&std::fs::read(pkg.join("README_submit.txt")).expect("cpp README"));
    assert_eq!(ours.len(), cpp.len(), "{ours:#?}");
    for (a, b) in ours.iter().zip(&cpp) {
        if b.starts_with("created: ") {
            assert!(a.starts_with("created: "), "{a}");
        } else if b.starts_with("timezone: ") && system_time_zone_id() != "Asia/Seoul" {
            assert_eq!(*a, format!("timezone: {}", system_time_zone_id()));
        } else {
            assert_eq!(a, b);
        }
    }
    assert!(ours.contains(&format!("survey_sha256: {DEMO_SHA256}")));
    assert_eq!(sha256_hex(&std::fs::read(&gpkg).expect("survey")), DEMO_SHA256, "the writer left the survey unchanged");

    // MANIFEST: same names in the same order; the hash of every file whose bytes equal the cpp file is the cpp
    // hash (README differs in created:, DBF in its date); every hash is the hash of our file.
    let ours = crlf_lines(&std::fs::read(out.join("MANIFEST.sha256")).expect("MANIFEST"));
    let cpp = crlf_lines(&std::fs::read(pkg.join("MANIFEST.sha256")).expect("cpp MANIFEST"));
    assert_eq!(ours.len(), 37);
    for (a, b) in ours.iter().zip(&cpp) {
        let (hex, name) = a.split_once("  ").expect("hex  name");
        assert_eq!(name, b.split_once("  ").expect("cpp line").1);
        assert_eq!(hex, sha256_hex(&std::fs::read(out.join(name)).expect(name)), "{name}");
        if name != "README_submit.txt" && !name.ends_with(".dbf") {
            assert_eq!(a, b);
        }
    }
}

/// ExportService.cpp:97-104 — a destination that is not empty or not a folder: the cpp text, nothing written.
#[test]
fn destination_not_empty_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let sources = demo_sources(&gpkg);
    let out = dir.path().join("pkg");
    std::fs::create_dir(&out).expect("pkg");
    std::fs::write(out.join("old.txt"), b"old").expect("old file");
    let file_target = dir.path().join("file.txt");
    std::fs::write(&file_target, b"x").expect("file");
    for target in [&out, &file_target] {
        let err = export_submission_package(target, &inputs(&sources, &gpkg), &mut |_, _, _| true).expect_err("refused");
        assert_eq!(err, format!("제출 폴더가 비어 있지 않거나 사용할 수 없습니다. 새 폴더를 선택하세요: {}", native(target)));
    }
    let left: Vec<_> = std::fs::read_dir(&out).expect("list").map(|e| e.expect("entry").file_name()).collect();
    assert_eq!(left, vec![std::ffi::OsString::from("old.txt")]);
    assert!(staging_dirs(dir.path()).is_empty());
}

/// An existing empty folder is replaced by the package (ExportService.cpp:231).
#[test]
fn empty_destination_folder_takes_the_package() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let sources = demo_sources(&gpkg);
    let out = dir.path().join("pkg");
    std::fs::create_dir(&out).expect("pkg");
    export_submission_package(&out, &inputs(&sources, &gpkg), &mut |_, _, _| true).expect("package");
    assert!(out.join("MANIFEST.sha256").is_file());
    assert!(staging_dirs(dir.path()).is_empty());
}

/// test_submit_package.cpp:215-232 — cancelling at any step leaves no package and no staging folder.
#[test]
fn cancel_at_any_step_leaves_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let sources = demo_sources(&gpkg);
    let out = dir.path().join("pkg");
    for stop in 0..12 {
        let err = export_submission_package(&out, &inputs(&sources, &gpkg), &mut |done, _, _| done < stop)
            .expect_err("cancelled");
        assert_eq!(err, CANCELED, "step {stop}");
        assert!(!out.exists(), "step {stop}");
        assert!(staging_dirs(dir.path()).is_empty(), "step {stop}");
    }
}

/// ExportService.cpp:183-184 — no layer with features and no sheet: the cpp text and no folder.
#[test]
fn nothing_to_submit_makes_no_folder() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let out = dir.path().join("pkg");
    let err = export_submission_package(&out, &inputs(&[], &gpkg), &mut |_, _, _| true).expect_err("nothing");
    assert_eq!(err, "내보낼 조사 도형이 없습니다. 조사구역이나 유구를 그린 뒤 다시 만드세요.");
    assert!(!out.exists());
    assert!(staging_dirs(dir.path()).is_empty());
}

/// test_submit_package.cpp:244-257 — a package made with checklist errors says so; a clean README does not.
#[test]
fn checklist_errors_are_stated_in_the_readme() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let sources = demo_sources(&gpkg);
    let out = dir.path().join("pkg");
    let summary = "- [error] 기준점이 2개 미만입니다\n";
    let inp = SubmitPackageInputs { checklist_summary: summary, checklist_errors_remain: true, ..inputs(&sources, &gpkg) };
    export_submission_package(&out, &inp, &mut |_, _, _| true).expect("package");
    let readme = String::from_utf8(std::fs::read(out.join("README_submit.txt")).expect("README")).expect("utf-8");
    assert!(readme.contains("검수 오류가 남았거나 검수하지 못한 채로 만들었습니다"), "{readme}");
    assert!(readme.contains("- [error] 기준점이 2개 미만입니다"));
    assert!(!build_readme(&ReadmeInputs::default()).contains("checklist_errors_remain"));
}

/// test_submit_package.cpp:234-241 — the manifest of a missing folder fails with the cpp text.
#[test]
fn manifest_of_a_missing_folder_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("ka-no-such-folder-4ad522");
    let err = write_sha256_manifest(&missing).expect_err("missing folder");
    assert_eq!(err, format!("해시 목록을 만들 폴더가 없습니다: {}. 폴더가 지워졌거나 옮겨졌는지 확인하세요.", native(&missing)));
}

/// test_submit_package.cpp:168-213 (README part) — survey name, file and fingerprint of the given survey file,
/// unsaved edits, two layers merged under one key.
#[test]
fn readme_records_the_survey_and_merged_layers() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let survey = dir.path().join("광령리.gpkg");
    std::fs::write(&survey, b"survey bytes").expect("survey file");
    let area = |title: &str| SubmitSource { gpkg: gpkg.clone(), table: "survey_area".to_owned(), title: title.to_owned() };
    let sources = vec![("survey_area", vec![area("조사구역"), area("조사구역 2")])];
    let out = dir.path().join("pkg");
    let inp = SubmitPackageInputs { survey_name: "광령리", unsaved_edits_included: true, ..inputs(&sources, &survey) };
    export_submission_package(&out, &inp, &mut |_, _, _| true).expect("package");
    let readme = String::from_utf8(std::fs::read(out.join("README_submit.txt")).expect("README")).expect("utf-8");
    for want in [
        "app: Strata ".to_owned(),
        "survey_name: 광령리".to_owned(),
        "survey_file: 광령리.gpkg".to_owned(),
        format!("survey_sha256: {}", sha256_hex(b"survey bytes")),
        "saved_state: unsaved_edits_included".to_owned(),
        "survey_area.shp features=2 crs=EPSG:5179 verified".to_owned(),
        "reference_data".to_owned(),
    ] {
        assert!(readme.contains(&want), "{want} missing in\n{readme}");
    }
}
