//! The submit package ≡ cpp `ExportService::exportSubmissionPackage` (ExportService.cpp:73-241) without the
//! sheet PDF (R3): checks, a sibling staging folder, the shapefiles, encoding.txt, README_submit.txt,
//! MANIFEST.sha256, then one rename into the chosen folder. Checklist errors never block (2026-10-10).

use std::io::Write;
use std::path::{Path, PathBuf};

use strata_engine::{write_submit_shapefiles, SubmitEncoding, SubmitSource, SUBMIT_KEYS};

use crate::submit_readme::{build_readme, native, sha256_of_file, text_mode, write_sha256_manifest, ReadmeInputs};

/// What the package is made from (cpp `SubmitPackageInfo` + `exportSubmissionPackage` arguments).
#[derive(Debug, Clone, Copy)]
pub struct SubmitPackageInputs<'a> {
    pub sources: &'a [(&'static str, Vec<SubmitSource>)],
    pub encoding: SubmitEncoding,
    pub survey_name: &'a str,
    pub survey_path: Option<&'a Path>,
    pub unsaved_edits_included: bool,
    pub checklist_summary: &'a str,
    pub checklist_errors_remain: bool,
    pub reference_lines: &'a [String],
    /// `Strata <version> (<commit>)`.
    pub app_line: &'a str,
}

const CANCELED: &str = "제출 꾸러미 만들기를 취소했습니다. 이전 결과는 그대로입니다.";

/// ExportService.cpp:96-97 — the folder holds any entry, hidden and system ones included.
fn has_entries(path: &Path) -> bool {
    std::fs::read_dir(path).map(|mut it| it.next().is_some()).unwrap_or(false)
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Builds the package into `out_dir` (which must not exist or be an empty folder) and returns it.
/// `progress(done, total, label)` returning false cancels; nothing is left behind on any failure.
///
/// # Errors
/// The cpp text of the failed check or step.
pub fn export_submission_package(
    out_dir: &Path,
    inp: &SubmitPackageInputs<'_>,
    progress: &mut dyn FnMut(usize, usize, &str) -> bool,
) -> Result<PathBuf, String> {
    // :93-105 the destination must be new or an empty folder.
    let final_path = std::path::absolute(out_dir).unwrap_or_else(|_| out_dir.to_path_buf());
    let existed = final_path.exists();
    if out_dir.to_string_lossy().trim().is_empty()
        || is_symlink(&final_path)
        || (existed && (!final_path.is_dir() || has_entries(&final_path)))
    {
        return Err(format!(
            "제출 폴더가 비어 있지 않거나 사용할 수 없습니다. 새 폴더를 선택하세요: {}",
            native(&final_path)
        ));
    }
    let parent = final_path.parent().map(Path::to_path_buf).unwrap_or_default();
    if !parent.exists() && std::fs::create_dir_all(&parent).is_err() {
        return Err(format!(
            "제출 위치를 만들 수 없습니다. 쓰기 권한이 있는 다른 폴더를 고르세요: {}",
            native(&parent)
        ));
    }
    // :107-111 a sibling keeps the final rename on one file system; dropping it removes only this run's folder.
    let staging = tempfile::Builder::new()
        .prefix(".ka-hgis-export-")
        .rand_bytes(6)
        .tempdir_in(&parent)
        .map_err(|e| format!("제출 임시 폴더를 만들 수 없습니다: {e}"))?;
    let dir = staging.path();

    // :117-120 EUC-KR / CP949 -> CP949, else UTF-8.
    let enc = match inp.encoding {
        SubmitEncoding::Utf8 => "UTF-8",
        SubmitEncoding::EucKr => "CP949",
    };
    // :125-131 one step per key, encoding.txt, 조사도면.pdf, 조사 파일 지문, MANIFEST.sha256, 확정.
    let total = SUBMIT_KEYS.len() + 1 + 4;
    let mut done = 0;
    let mut step = |label: &str| {
        let go_on = progress(done, total, label);
        done += 1;
        go_on
    };
    let mut readme = ReadmeInputs {
        app_line: inp.app_line.to_owned(),
        encoding: enc.to_owned(),
        survey_name: inp.survey_name.to_owned(),
        unsaved_edits_included: inp.unsaved_edits_included,
        checklist_errors_remain: inp.checklist_errors_remain,
        checklist_summary: inp.checklist_summary.to_owned(),
        ..ReadmeInputs::default()
    };

    // :140-182 the shapefiles, key by key in cpp order.
    for key in SUBMIT_KEYS {
        if !step(&format!("{key}.shp")) {
            return Err(CANCELED.to_owned());
        }
        let sources: Vec<SubmitSource> =
            inp.sources.iter().filter(|(k, _)| *k == key).flat_map(|(_, list)| list.iter().cloned()).collect();
        let (reports, notes) = write_submit_shapefiles(&[(key, sources)], dir, inp.encoding).map_err(|e| e.0)?;
        readme.field_notes.extend(notes);
        readme
            .layer_lines
            .extend(reports.iter().map(|r| format!("{}.shp features={} crs=EPSG:5179 verified", r.key, r.features)));
    }
    // :183-184 nothing to submit: no empty package (there is no sheet PDF before R3).
    if readme.layer_lines.is_empty() {
        return Err("내보낼 조사 도형이 없습니다. 조사구역이나 유구를 그린 뒤 다시 만드세요.".to_owned());
    }

    // :188-192 encoding.txt = encoding + LF, written as bytes.
    if !step("encoding.txt") {
        return Err(CANCELED.to_owned());
    }
    std::fs::write(dir.join("encoding.txt"), format!("{enc}\n")).map_err(|e| format!("encoding.txt 저장 실패: {e}"))?;

    // :194-204 the sheet PDF comes with R3; the reference lines come from the caller.
    if !step("조사도면.pdf") {
        return Err(CANCELED.to_owned());
    }
    readme.reference_lines = inp.reference_lines.to_vec();

    // :206-224 README after the PDF, so it states what is in the package.
    if !step("조사 파일 지문") {
        return Err(CANCELED.to_owned());
    }
    if let Some(survey) = inp.survey_path.filter(|p| p.is_file()) {
        readme.survey_file = survey.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        readme.survey_sha256 = sha256_of_file(survey)?;
    }
    readme.has_sheet_pdf = dir.join("조사도면.pdf").exists();
    let mut f = std::fs::File::create(dir.join("README_submit.txt"))
        .map_err(|e| format!("README_submit.txt를 쓸 수 없습니다({e}). 폴더 쓰기 권한과 남은 공간을 확인하세요."))?;
    f.write_all(&text_mode(&build_readme(&readme)))
        .and_then(|()| f.sync_all())
        .map_err(|e| format!("README_submit.txt 저장 실패: {e}"))?;
    drop(f);

    if !step("MANIFEST.sha256") {
        return Err(CANCELED.to_owned());
    }
    write_sha256_manifest(dir)?;

    // :226-239 recheck the destination, then one rename.
    if !step("확정") {
        return Err(CANCELED.to_owned());
    }
    if is_symlink(&final_path)
        || (!existed && final_path.exists())
        || (existed && (!final_path.is_dir() || has_entries(&final_path) || std::fs::remove_dir(&final_path).is_err()))
    {
        return Err("제출 중 결과 폴더가 변경되었거나 잠겨 있습니다. 새 폴더로 다시 시도하세요.".to_owned());
    }
    std::fs::rename(dir, &final_path).map_err(|_| {
        "제출 결과 폴더를 확정하지 못했습니다. 쓰기 권한과 다른 프로그램의 사용 여부를 확인하세요.".to_owned()
    })?;
    // The staging folder is now the package; nothing is left to remove.
    let _ = staging.keep();
    progress(total, total, "완료");
    Ok(out_dir.to_path_buf())
}

#[cfg(test)]
#[path = "submit_builder_tests.rs"]
mod tests;
