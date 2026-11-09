use sha2::{Digest, Sha256};

use strata_types::crs::SubmitCrs;
use strata_types::error::StrataResult;

use crate::checklist_engine::{ChecklistFinding, ChecklistSeverity};
use crate::submit_package::*;

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
