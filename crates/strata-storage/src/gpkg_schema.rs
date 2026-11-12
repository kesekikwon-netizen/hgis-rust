//! GeoPackage 헤더 상수와 스키마 검증·마이그레이션.
//!
//! `gpkg` 에서 분리했다.

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const GPKG_MAGIC_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// C++ `HGIS_SCHEMA_VERSION` (5) — `KaSchemaMigrator::validateAndMigrate` 목표 스키마 버전 상수
pub const HGIS_SCHEMA_VERSION: i32 = 5;

/// C++ `KaSchemaMigrator::MigrationStatus` 1:1 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    UpToDate,
    MigratedSuccess,
    FutureVersion,
    CorruptedDb,
    SqlExecutionFail,
}

/// C++ `KaSchemaMigrator::validateAndMigrate(sqlite3 *db, int appTargetVersion)` 1:1 대응:
/// SQLite `PRAGMA user_version`을 조회하여:
/// - `V_file > app_target_version`: 미래 버전 열기 차단(`MigrationStatus::FutureVersion`)
/// - `V_file == app_target_version`: `MigrationStatus::UpToDate`
/// - `V_file < app_target_version`: 단독 트랜잭션(RAII 자동 ROLLBACK 보장) 내에서 증분 DDL(`hgis_meta`, `rem_elevation`) 실행 후 `PRAGMA user_version` 갱신 및 `MigrationStatus::MigratedSuccess` 반환
pub fn validate_and_migrate_schema(path: &Path, app_target_version: i32) -> MigrationStatus {
    let Ok(bytes) = std::fs::read(path) else {
        return MigrationStatus::CorruptedDb;
    };
    if bytes.len() < GPKG_MAGIC_HEADER.len() || &bytes[..16] != GPKG_MAGIC_HEADER {
        return MigrationStatus::CorruptedDb;
    }

    let Ok(mut conn) = Connection::open(path) else {
        return MigrationStatus::CorruptedDb;
    };
    let Ok(file_version) = conn.query_row("PRAGMA user_version;", [], |row| row.get::<_, i32>(0))
    else {
        return MigrationStatus::CorruptedDb;
    };

    if file_version < 0 {
        return MigrationStatus::CorruptedDb;
    }
    if file_version > app_target_version {
        return MigrationStatus::FutureVersion;
    }
    if file_version == app_target_version {
        return MigrationStatus::UpToDate;
    }

    // RAII Transaction guard: uncommitted transaction automatically rolls back on drop
    let Ok(tx) = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive) else {
        return MigrationStatus::SqlExecutionFail;
    };

    for step_ver in (file_version + 1)..=app_target_version {
        let ddl_res = match step_ver {
            2 => tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS hgis_meta (
                     meta_key TEXT PRIMARY KEY,
                     meta_value TEXT NOT NULL
                 );",
            ),
            3 => tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS hgis_feature_ext (
                     fid INTEGER PRIMARY KEY,
                     rem_elevation DOUBLE NOT NULL DEFAULT 0.0
                 );",
            ),
            _ => tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS hgis_meta (
                     meta_key TEXT PRIMARY KEY,
                     meta_value TEXT NOT NULL
                 );",
            ),
        };
        if ddl_res.is_err() {
            return MigrationStatus::SqlExecutionFail;
        }
    }

    let pragma_sql = format!("PRAGMA user_version = {app_target_version};");
    if tx.execute_batch(&pragma_sql).is_err() {
        return MigrationStatus::SqlExecutionFail;
    }
    if tx.commit().is_err() {
        return MigrationStatus::SqlExecutionFail;
    }
    MigrationStatus::MigratedSuccess
}

/// C++ `KaSchemaMigrator` 미래 버전 안내 메시지 포맷터
#[must_use]
pub fn format_future_schema_version_error(file_version: i32, app_version: i32) -> String {
    format!(
        "이 프로젝트 파일은 더 최신 버전의 HGIS (스키마 v{file_version})에서 작성되었습니다. 현재 프로그램(스키마 v{app_version})에서는 열 수 없습니다. 최신 버전으로 업데이트해 주세요."
    )
}
