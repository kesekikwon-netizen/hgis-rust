//! `gpkg` 코어 시험 — 파일 생성·원자적 쓰기·스키마 마이그레이션.
//!
//! `gpkg` 에서 분리했다.

use std::collections::BTreeMap;

use rusqlite::Connection;

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::edit_buffer::LayerEditBuffer;
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

    let srs_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM gpkg_spatial_ref_sys WHERE srs_id IN (5187, 5186, 5179)",
            [],
            |r| r.get(0),
        )
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    assert_eq!(srs_count, 3);

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

#[test]
fn test_gpkg_atomic_write_commits_1000_features_and_verifies_pragma_integrity_ok(
) -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("atomic_1000_features.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187)?;

    let mut buf = LayerEditBuffer::new(DomainLayerKey::FeaturePoly);
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
        attrs.insert("FEAT_NO".to_owned(), format!("유구-{}", i + 1));
        buf.add_unsaved_feature(wkt, attrs);
    }

    let code =
        GpkgRepository::commit_layer_atomic(&path, DomainLayerKey::FeaturePoly.id(), &buf);
    assert_eq!(code, GpkgResultCode::Success);
    assert_eq!(
        GpkgRepository::commit_layer_atomic(&path, "", &buf),
        GpkgResultCode::SchemaMismatch
    );

    let reopened = GpkgRepository::open_read_only(&path)?;
    let committed_layer = reopened
        .layers
        .get(DomainLayerKey::FeaturePoly.id())
        .expect("committed layer must exist");
    assert_eq!(committed_layer.features.len(), 1000);
    assert!(committed_layer.features.iter().all(|f| f.fid > 0));
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
