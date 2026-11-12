//! `gpkg` 코어 시험 — 파일 생성·원자적 쓰기·스키마 마이그레이션.
//!
//! `gpkg` 에서 분리했다.

use std::collections::BTreeMap;

use rusqlite::Connection;

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::gpkg::*;

#[test]
fn created_gpkg_passes_sqlite_pragma_integrity_check_and_has_gpkg_contents() -> StrataResult<()>
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("real_sqlite_test.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187)?;

    let conn =
        rusqlite::Connection::open(&path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(integrity, "ok");

    // C++(GDAL) 새 조사 파일은 작업 좌표계 하나만 담는다(5179 는 제출 때만 쓴다).
    let srs_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM gpkg_spatial_ref_sys WHERE srs_id IN (5187, 5186, 5179)",
            [],
            |r| r.get(0),
        )
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(srs_count, 1);

    let schema_path = dir.path().join("survey_schema_5_layers.gpkg");
    assert!(GpkgRepository::create_survey_schema_5_layers(
        &schema_path,
        WorkCrs::Epsg5186
    ));
    let schema_conn = rusqlite::Connection::open(&schema_path)
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    let geom_cols_count: i64 = schema_conn
        .query_row(
            "SELECT COUNT(*) FROM gpkg_geometry_columns WHERE geometry_type_name IN ('MULTIPOLYGON', 'MULTILINESTRING', 'POINT')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(geom_cols_count, 5);
    Ok(())
}

/// 조사 저장(하나로 모은 길)이 새 도형 1000개를 쓰고 무결성·양수 fid 를 지킨다.
#[test]
fn test_survey_save_commits_1000_features_and_verifies_pragma_integrity_ok(
) -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("atomic_1000_features.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187)?;
    let mut tables = crate::survey_gpkg_read::read_survey_tables(&path)?;
    let poly_idx = tables
        .iter()
        .position(|t| t.layer.key == Some(DomainLayerKey::FeaturePoly))
        .ok_or_else(|| StrataError::Storage("feature_poly".into()))?;
    let buf = &mut tables[poly_idx].layer;
    for i in 0..1000 {
        let x = 219_800.0 + f64::from(i % 50) * 5.0;
        let y = 361_500.0 + f64::from(i / 50) * 5.0;
        let wkt = format!(
            "POLYGON(({x:.1} {y:.1}, {:.1} {y:.1}, {:.1} {:.1}, {x:.1} {:.1}, {x:.1} {y:.1}))",
            x + 3.0,
            x + 3.0,
            y + 3.0,
            y + 3.0
        );
        let mut attrs = BTreeMap::new();
        attrs.insert("feature_no".to_owned(), format!("유구-{}", i + 1));
        buf.add_unsaved_feature(wkt, attrs);
    }

    crate::survey_gpkg_save::save_survey(&path, &tables)
        .map_err(|e| StrataError::Storage(format!("{e:?}")))?;

    let reopened = crate::survey_gpkg_read::read_survey_tables(&path)?;
    let committed_layer = &reopened
        .iter()
        .find(|t| t.layer.key == Some(DomainLayerKey::FeaturePoly))
        .ok_or_else(|| StrataError::Storage("feature_poly".into()))?
        .layer;
    assert_eq!(committed_layer.features.len(), 1000);
    assert!(committed_layer.features.iter().all(|f| f.fid > 0));
    let conn = Connection::open(&path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(integrity, "ok");
    Ok(())
}

#[test]
fn test_schema_migrator_upgrade_v1_to_v5_and_block_future_version_999() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let v1_path = dir.path().join("legacy_v1.gpkg");
    {
        let conn =
            Connection::open(&v1_path).map_err(|e| StrataError::Storage(e.to_string()))?;
        conn.execute_batch("PRAGMA user_version = 1;")
            .map_err(|e| StrataError::Storage(e.to_string()))?;
    }

    // 1. Upgrade v1 -> v5 (HGIS_SCHEMA_VERSION)
    let status = validate_and_migrate_schema(&v1_path, HGIS_SCHEMA_VERSION);
    assert_eq!(status, MigrationStatus::MigratedSuccess);

    // Verify PRAGMA user_version is now 5 and tables exist
    let conn = Connection::open(&v1_path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let ver: i32 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(ver, HGIS_SCHEMA_VERSION);

    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='hgis_meta'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(table_exists, 1);

    // Calling again on v5 returns UpToDate
    assert_eq!(
        validate_and_migrate_schema(&v1_path, HGIS_SCHEMA_VERSION),
        MigrationStatus::UpToDate
    );

    // 2. Block future version 999
    let future_path = dir.path().join("future_v999.gpkg");
    {
        let fconn =
            Connection::open(&future_path).map_err(|e| StrataError::Storage(e.to_string()))?;
        fconn
            .execute_batch("PRAGMA user_version = 999;")
            .map_err(|e| StrataError::Storage(e.to_string()))?;
    }
    assert_eq!(
        validate_and_migrate_schema(&future_path, HGIS_SCHEMA_VERSION),
        MigrationStatus::FutureVersion
    );
    let msg = format_future_schema_version_error(999, HGIS_SCHEMA_VERSION);
    assert!(msg.contains("스키마 v999") && msg.contains("스키마 v5"));
    Ok(())
}

/// C++ `SurveyProjectFactory.cpp:44-49` 의 거절 문구 그대로.
const SAME_NAME_SURVEY_MSG: &str = "같은 이름의 조사 파일이 이미 있습니다. 기존 파일은 그대로 두었습니다. 다른 조사명이나 폴더를 선택해 주세요.";

/// 새 조사: 같은 이름 .gpkg 가 있으면 만들지 않고 기존 파일을 한 바이트도 바꾸지 않는다.
#[test]
fn create_new_survey_refuses_existing_gpkg_and_keeps_its_bytes() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("병산동.gpkg");
    std::fs::write(&path, b"existing survey bytes")?;

    match GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187) {
        Err(StrataError::InvalidInput(msg)) => assert_eq!(msg, SAME_NAME_SURVEY_MSG),
        other => panic!("an existing .gpkg must be refused, got {other:?}"),
    }
    assert_eq!(std::fs::read(&path)?, b"existing survey bytes");
    Ok(())
}

/// 새 조사: 같은 이름 .qgz(C++ 작업공간 사본)만 있어도 거절하고 .gpkg 를 만들지 않는다.
#[test]
fn create_new_survey_refuses_when_same_name_qgz_exists() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let gpkg = dir.path().join("병산동.gpkg");
    let qgz = dir.path().join("병산동.qgz");
    std::fs::write(&qgz, b"cpp workspace copy")?;

    match GpkgRepository::create_new_survey(&gpkg, WorkCrs::Epsg5187) {
        Err(StrataError::InvalidInput(msg)) => assert_eq!(msg, SAME_NAME_SURVEY_MSG),
        other => panic!("a same-name .qgz must be refused, got {other:?}"),
    }
    assert!(!gpkg.exists(), "no .gpkg may be created next to the .qgz");
    assert_eq!(std::fs::read(&qgz)?, b"cpp workspace copy");
    Ok(())
}

/// 자기 작업공간 파일은 임시 파일에 다 쓴 뒤 바꿔 넣는다. 남은 임시 파일이 있어도 쓰고,
/// 끝나면 조사 파일 하나만 남는다.
#[test]
fn write_workspace_file_replaces_own_workspace_and_leaves_no_temp_file() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("새조사.gpkg");
    // 바탕은 Rust 작업공간 형식(새 조사는 C++ 형식이라 이 시험의 대상이 아니다).
    let mut ws = SurveyWorkspace {
        schema_version: 3,
        work_crs: WorkCrs::Epsg5187,
        layers: BTreeMap::new(),
        embedded_project_json: String::from("{}"),
    };
    GpkgRepository::write_workspace_file(&path, &ws)?;
    std::fs::write(GpkgRepository::workspace_temp_path(&path), b"stale temp")?;

    ws.work_crs = WorkCrs::Epsg5186;
    ws.ensure_domain_layer(DomainLayerKey::SurveyArea);
    GpkgRepository::write_workspace_file(&path, &ws)?;

    let reopened = GpkgRepository::open_read_only(&path)?;
    assert_eq!(reopened.work_crs, WorkCrs::Epsg5186);
    assert!(reopened.layers.contains_key("survey_area"));
    let names: Vec<_> = std::fs::read_dir(dir.path())?
        .filter_map(Result::ok)
        .map(|e| e.file_name())
        .collect();
    assert_eq!(names.len(), 1, "only the survey file may remain: {names:?}");
    Ok(())
}
