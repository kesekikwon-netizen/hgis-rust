use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use strata_layout::pdf_geotiff::render_sheet_pdf_bytes;
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_storage::shp_io::{write_shapefile_utf8, ShpFeatureRow};
use strata_types::crs::SubmitCrs;
use strata_types::error::StrataResult;

use crate::checklist_engine::{ChecklistFinding, ChecklistSeverity};

/// C++ `SubmitPackageExporter::exportSubmissionZip` 반환 상태 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportResult {
    Success,
    HasBlockingErrors,
    IoError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmissionManifest {
    pub crs: SubmitCrs,
    pub files_sha256: Vec<(String, String)>,
    pub readme_path: PathBuf,
}

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
        let status = if f.passed { "통과" } else { "안내(비차단)" };
        readme.push_str(&format!("- [{}] {}: {}\n", status, f.rule_id, f.message_ko));
    }
    std::fs::write(&readme_path, readme.as_bytes())?;

    // 4. MANIFEST.sha256
    let tracked = ["site_boundary_5179.shp", "조사도면.pdf", "README_submit.txt"];
    let mut files_sha256 = Vec::with_capacity(tracked.len());
    let mut manifest_txt = String::new();
    for file_name in tracked {
        let bytes = std::fs::read(out_dir.join(file_name))?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest_hex = format!("{:x}", hasher.finalize());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submission_package_writes_5179_manifest_and_non_blocking_readme() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let findings = vec![ChecklistFinding {
            rule_id: "R-NORTH-ARROW".to_owned(),
            category_ko: "조판·좌표계".to_owned(),
            message_ko: "방위표 위치 재확인 권장".to_owned(),
            severity: crate::checklist_engine::ChecklistSeverity::Warning,
            passed: false,
            target_world_xy: None,
        }];
        let manifest = export_submission_package(dir.path(), &findings)?;
        assert_eq!(manifest.crs, SubmitCrs::Epsg5179);
        assert!(dir.path().join("MANIFEST.sha256").exists());
        let readme = std::fs::read_to_string(&manifest.readme_path)?;
        assert!(readme.contains("EPSG:5179"));
        assert!(readme.contains("R-NORTH-ARROW"));
        Ok(())
    }

    #[test]
    fn test_submit_package_exporter_blocks_on_missing_feat_no_and_succeeds_after_fix(
    ) -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let blocking_findings = vec![ChecklistFinding {
            rule_id: "R15-REQ-ATTR".to_owned(),
            category_ko: "속성·스키마".to_owned(),
            message_ko: "15. 필수 속성(조사명·유구번호) 기입 누락(FEAT_NO)".to_owned(),
            severity: ChecklistSeverity::Error,
            passed: false,
            target_world_xy: None,
        }];
        let (res, manifest_opt) =
            export_submission_package_checked(dir.path(), &blocking_findings, true);
        assert_eq!(res, ExportResult::HasBlockingErrors);
        assert!(manifest_opt.is_none());
        assert!(!dir.path().join("MANIFEST.sha256").exists());

        let resolved_findings = vec![ChecklistFinding {
            rule_id: "R15-REQ-ATTR".to_owned(),
            category_ko: "속성·스키마".to_owned(),
            message_ko: "15. 필수 속성(조사명·유구번호) 기입 완료".to_owned(),
            severity: ChecklistSeverity::Pass,
            passed: true,
            target_world_xy: None,
        }];
        let (res_ok, manifest_ok) =
            export_submission_package_checked(dir.path(), &resolved_findings, true);
        assert_eq!(res_ok, ExportResult::Success);
        assert_eq!(manifest_ok.expect("manifest").files_sha256.len(), 3);
        Ok(())
    }
}
