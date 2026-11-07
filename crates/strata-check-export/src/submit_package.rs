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

pub const HASH_CHUNK_SIZE: usize = 65536;

/// C++ `ManifestBuilder::generateSha256Manifest` — 64KB 청크 단위 스트리밍 SHA-256 체크섬 계산
pub fn stream_file_sha256_hex(path: &Path) -> StrataResult<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; HASH_CHUNK_SIZE];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// C++ `HGIS_NEARBY_HERITAGE_BUFFER_M` — 주변 유적 기본 조사 반경 (`500.0 m`)
pub const HGIS_NEARBY_HERITAGE_BUFFER_M: f64 = 500.0;
/// C++ `HGIS_MIN_VALID_FEATURE_AREA_M2` — 최소 허용 유구 면적 슬리버 임계치 (`0.05 m²`)
pub const HGIS_MIN_VALID_FEATURE_AREA_M2: f64 = 0.05;
/// C++ `HGIS_DUPLICATE_VERTEX_TOL_M` — 중복 정점 허용 한계 (`0.001 m` = `1 mm`)
pub const HGIS_DUPLICATE_VERTEX_TOL_M: f64 = 0.001;
/// C++ `HGIS_DELIVERABLE_ZIP_COMPRESS` — 성과품 압축률 (`9`)
pub const HGIS_DELIVERABLE_ZIP_COMPRESS: u8 = 9;

/// C++ `HeritageSiteRecord` (`NearbyHeritageExport`) — 반경 500m 주변 유적 레코드
#[derive(Debug, Clone, PartialEq)]
pub struct HeritageSiteRecord {
    pub site_name: String,
    pub era: String,
    pub designation: String,
    pub distance_m: f64,
    pub centroid_coord: [f64; 2],
}

/// C++ `NearbyHeritageExport::extractNearbySites` — 조사구역 경계로부터 `buffer_radius_m` (기본 `500.0 m`) 이내 유적을 추출하고 거리순 오름차순 정렬 반환
#[must_use]
pub fn extract_nearby_heritage_sites(
    candidates: &[HeritageSiteRecord],
    buffer_radius_m: f64,
) -> Vec<HeritageSiteRecord> {
    let radius = if buffer_radius_m.is_finite() && buffer_radius_m > 0.0 {
        buffer_radius_m
    } else {
        HGIS_NEARBY_HERITAGE_BUFFER_M
    };
    let mut matched: Vec<HeritageSiteRecord> = candidates
        .iter()
        .filter(|s| s.distance_m.is_finite() && s.distance_m >= 0.0 && s.distance_m <= radius)
        .cloned()
        .collect();
    matched.sort_by(|a, b| {
        a.distance_m
            .partial_cmp(&b.distance_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matched
}

/// C++ `ParcelClipboard` — 지적 편입 필지 조서 행
#[derive(Debug, Clone, PartialEq)]
pub struct ParcelClipboardRow {
    pub pnu: String,
    pub location: String,
    pub jibun: String,
    pub jimok: String,
    pub official_area_m2: f64,
    pub included_area_m2: f64,
}

/// C++ `ParcelClipboard::copyParcelTableToClipboard` — 지적 편입 필지 조서 TSV 직렬화
#[must_use]
pub fn format_parcel_table_tsv(rows: &[ParcelClipboardRow]) -> String {
    let mut out =
        String::from("연번\tPNU\t소재지\t지번\t지목\t공부면적(㎡)\t편입면적(㎡)\t편입비율(%)\n");
    for (idx, r) in rows.iter().enumerate() {
        let ratio = if r.official_area_m2 > 0.0 {
            (r.included_area_m2 / r.official_area_m2 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.1}\n",
            idx + 1,
            r.pnu,
            r.location,
            r.jibun,
            r.jimok,
            r.official_area_m2,
            r.included_area_m2,
            ratio
        ));
    }
    out
}

/// C++ `ChecklistStateFields` (`FLD-01`) — 필수 속성 필드(`FEAT_NO`, `FEAT_TYPE`, `PERIOD`) 공백/누락 검사
#[must_use]
pub fn audit_required_feature_fields_fld01(
    feat_no: &str,
    feat_type: &str,
    period: &str,
) -> Option<(&'static str, ChecklistSeverity)> {
    if feat_no.trim().is_empty() || feat_type.trim().is_empty() || period.trim().is_empty() {
        Some(("FLD-01", ChecklistSeverity::Error))
    } else {
        None
    }
}

/// C++ `ChecklistStateGeometry` (`GEO-02`) — 유구 면적 슬리버(`area_m2 < 0.05 m²`) 검사
#[must_use]
pub fn audit_sliver_polygon_geo02(area_m2: f64) -> Option<(&'static str, ChecklistSeverity)> {
    if !area_m2.is_finite() || area_m2 < HGIS_MIN_VALID_FEATURE_AREA_M2 {
        Some(("GEO-02", ChecklistSeverity::Warning))
    } else {
        None
    }
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

    #[test]
    fn test_manifest_sha256_streaming_checksum_matches_exact_digest() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let test_path = dir.path().join("test.txt");
        std::fs::write(&test_path, b"HGIS_MANIFEST_TEST_DATA")?;
        let digest = stream_file_sha256_hex(&test_path)?;
        assert_eq!(digest.len(), 64);

        let mut expected_hasher = Sha256::new();
        expected_hasher.update(b"HGIS_MANIFEST_TEST_DATA");
        let expected_hex = format!("{:x}", expected_hasher.finalize());
        assert_eq!(digest, expected_hex);
        Ok(())
    }

    #[test]
    fn test_checklist_3axis_nearby_500m_and_parcel_clipboard_tsv() {
        // 1. TestChecklistStateFields::testMissingRequiredFields ("1호", "", "삼국") -> ("FLD-01", Error)
        assert_eq!(
            audit_required_feature_fields_fld01("1호", "", "삼국"),
            Some(("FLD-01", ChecklistSeverity::Error))
        );
        assert!(audit_required_feature_fields_fld01("1호", "수혈", "삼국").is_none());

        // 2. TestChecklistStateGeometry::testSliverPolygonDetection (0.02 m² < 0.05 m²) -> ("GEO-02", Warning)
        assert_eq!(
            audit_sliver_polygon_geo02(0.02),
            Some(("GEO-02", ChecklistSeverity::Warning))
        );
        assert!(audit_sliver_polygon_geo02(1.20).is_none());

        // 3. TestNearbyHeritageExport::test500mBufferExtraction (Site A at 350m, Site B at 600m)
        let sites = vec![
            HeritageSiteRecord {
                site_name: "유적 B".to_string(),
                era: "조선".to_string(),
                designation: "비지정".to_string(),
                distance_m: 600.0,
                centroid_coord: [220_400.0, 360_100.0],
            },
            HeritageSiteRecord {
                site_name: "유적 A".to_string(),
                era: "삼국".to_string(),
                designation: "사적".to_string(),
                distance_m: 350.0,
                centroid_coord: [220_150.0, 360_100.0],
            },
        ];
        let extracted = extract_nearby_heritage_sites(&sites, HGIS_NEARBY_HERITAGE_BUFFER_M);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].site_name, "유적 A");
        assert!((extracted[0].distance_m - 350.0).abs() <= 0.1);

        // 4. TestParcelClipboard::testTsvFormatting (PNU="123", jibun="10-1", area=100.0, included=50.0)
        let tsv = format_parcel_table_tsv(&[ParcelClipboardRow {
            pnu: "123".to_string(),
            location: "경북 경주시 황남동".to_string(),
            jibun: "10-1".to_string(),
            jimok: "대".to_string(),
            official_area_m2: 100.0,
            included_area_m2: 50.0,
        }]);
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[1],
            "1\t123\t경북 경주시 황남동\t10-1\t대\t100.00\t50.00\t50.0"
        );
    }
}
