//! SHP·PDF·MANIFEST.sha256 를 묶는 제출 꾸러미 생성.
//!
//! `submit_package` 에서 분리했다.

use std::path::Path;

use strata_layout::pdf_geotiff::render_sheet_pdf_bytes;
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_storage::shp_io::{write_shapefile_utf8, ShpFeatureRow};
use strata_types::crs::SubmitCrs;
use strata_types::error::StrataResult;

use crate::checklist_engine::{ChecklistFinding, ChecklistSeverity};
use crate::submit_manifest::{stream_file_sha256_hex, ExportResult, SubmissionManifest};

/// C++ `SubmitPackageExporter::exportSubmissionZip` — `Severity::Error` 존재 시 `ExportResult::HasBlockingErrors` 반환 지원
pub fn export_submission_package_checked(
    out_dir: &Path,
    findings: &[ChecklistFinding],
    block_on_error: bool,
) -> (ExportResult, Option<SubmissionManifest>) {
    if block_on_error
        && findings
            .iter()
            .any(|f| !f.passed && f.severity == ChecklistSeverity::Error)
    {
        return (ExportResult::HasBlockingErrors, None);
    }
    match export_submission_package(out_dir, findings) {
        Ok(manifest) => (ExportResult::Success, Some(manifest)),
        Err(_) => (ExportResult::IoError, None),
    }
}

pub fn export_submission_package(
    out_dir: &Path,
    findings: &[ChecklistFinding],
) -> StrataResult<SubmissionManifest> {
    std::fs::create_dir_all(out_dir)?;
    let crs = SubmitCrs::Epsg5179;

    // 1. EPSG:5179 Shapefile 세트
    let shp_path = out_dir.join("site_boundary_5179.shp");
    let rows = vec![ShpFeatureRow {
        wkt: "POLYGON ((0 0, 100 0, 100 100, 0 100, 0 0))".to_owned(),
        fields: vec![("site_name".to_owned(), "현장경계".to_owned())],
    }];
    write_shapefile_utf8(&shp_path, crs.epsg_code(), &rows)?;

    // 2. 조사도면.pdf
    let pdf_path = out_dir.join("조사도면.pdf");
    let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    let pdf_bytes = render_sheet_pdf_bytes(&sheet, "국가유산청 제출용 조사도면")?;
    std::fs::write(&pdf_path, &pdf_bytes)?;

    // 3. README_submit.txt (점검 결과 안내 기록, 내보내기 차단 없음)
    let readme_path = out_dir.join("README_submit.txt");
    let mut readme = format!(
        "Strata 국가유산청 제출 패키지\n좌표계: {} (UTM-K / GRS80)\n\n[도면 점검 항목 기록]\n",
        crs.auth_id()
    );
    for f in findings {
        let status = if f.passed {
            "통과"
        } else if f.severity == ChecklistSeverity::Error {
            "error"
        } else {
            "warn"
        };
        readme.push_str(&format!("- [{}] {}: {}\n", status, f.rule_id, f.message_ko));
    }
    std::fs::write(&readme_path, readme.as_bytes())?;

    // 4. MANIFEST.sha256 (64KB 스트리밍 청크 SHA-256 해시)
    let tracked = [
        "site_boundary_5179.shp",
        "조사도면.pdf",
        "README_submit.txt",
    ];
    let mut files_sha256 = Vec::with_capacity(tracked.len());
    let mut manifest_txt = String::new();
    for file_name in tracked {
        let digest_hex = stream_file_sha256_hex(&out_dir.join(file_name))?;
        manifest_txt.push_str(&format!("{digest_hex}  {file_name}\n"));
        files_sha256.push((file_name.to_owned(), digest_hex));
    }
    std::fs::write(out_dir.join("MANIFEST.sha256"), manifest_txt.as_bytes())?;

    Ok(SubmissionManifest {
        crs,
        files_sha256,
        readme_path,
    })
}
