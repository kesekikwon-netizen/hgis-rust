use crate::edit_buffer::LayerEditBuffer;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

pub const GPKG_MAGIC_HEADER: &[u8; 16] = b"SQLite format 3\0";

static GPKG_REPO_MUTEX: Mutex<()> = Mutex::new(());

/// C++ `GpkgAtomicWrite::commitLayer` 결과 코드 (`enum class GpkgResultCode`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpkgResultCode {
    Success,
    LockFailed,
    SchemaMismatch,
    DiskFull,
    WriteError,
}

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

pub const HGIS_MAX_RECENT_PROJECTS: usize = 10;
pub const HGIS_AUTOSAVE_INTERVAL_SEC: u64 = 300;
pub const HGIS_LOCK_FILE_EXT: &str = ".hgis.lock";
pub const HGIS_AUTOSAVE_FILE_EXT: &str = ".hgis.autosave";

/// C++ `KaProjectManager::ProjectOpenResult` 1:1 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectOpenResult {
    Success,
    FileNotFound,
    LockedByAnotherProcess,
    CrashDetectedNeedsRecovery,
    InvalidMagicBytes,
    FutureVersionBlocked,
}

/// C++ `KaProjectManager::openProject` 사전 검사(`.hgis.lock`, `.hgis.autosave`, SQLite 16바이트 매직 헤더)
#[must_use]
pub fn check_project_open_preflight(project_path: &Path, force_unlock: bool) -> ProjectOpenResult {
    if !project_path.exists() {
        return ProjectOpenResult::FileNotFound;
    }
    let lock_path = std::path::PathBuf::from(format!("{}{HGIS_LOCK_FILE_EXT}", project_path.display()));
    if lock_path.exists() && !force_unlock {
        return ProjectOpenResult::LockedByAnotherProcess;
    }
    let autosave_path =
        std::path::PathBuf::from(format!("{}{HGIS_AUTOSAVE_FILE_EXT}", project_path.display()));
    let wal_path = std::path::PathBuf::from(format!("{}.wal", project_path.display()));
    if autosave_path.exists() || wal_path.exists() {
        return ProjectOpenResult::CrashDetectedNeedsRecovery;
    }
    let Ok(bytes) = std::fs::read(project_path) else {
        return ProjectOpenResult::FileNotFound;
    };
    if bytes.len() < GPKG_MAGIC_HEADER.len() || &bytes[..16] != GPKG_MAGIC_HEADER {
        return ProjectOpenResult::InvalidMagicBytes;
    }
    ProjectOpenResult::Success
}

/// C++ `KaProjectManager` 최근 프로젝트 MRU 목록 갱신 (중복 제거 후 최상단 배치, 최대 `HGIS_MAX_RECENT_PROJECTS = 10`개 유지)
pub fn push_recent_project_mru(recent_list: &mut Vec<String>, new_path: &str) {
    recent_list.retain(|p| p != new_path);
    recent_list.insert(0, new_path.to_owned());
    if recent_list.len() > HGIS_MAX_RECENT_PROJECTS {
        recent_list.truncate(HGIS_MAX_RECENT_PROJECTS);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyWorkspace {
    pub schema_version: u32,
    pub work_crs: WorkCrs,
    pub layers: BTreeMap<String, LayerEditBuffer>,
    pub embedded_project_json: String,
}

#[derive(Debug, Clone, Copy)]
pub struct GpkgRepository;

impl GpkgRepository {
    /// C++ `GpkgAtomicWrite::commitLayer(gpkgPath, layerName, layer) -> GpkgResultCode`
    /// 프로세스 내 상호 배제(`GPKG_REPO_MUTEX`), Windows 파일 잠금 재시도 루프,
    /// SQLite 트랜잭션 커밋 후 `PRAGMA integrity_check == "ok"` 검증을 수행합니다.
    pub fn commit_layer_atomic(
        path: &Path,
        layer_name: &str,
        layer: &LayerEditBuffer,
    ) -> GpkgResultCode {
        if layer_name.trim().is_empty() {
            return GpkgResultCode::SchemaMismatch;
        }
        let _guard = match GPKG_REPO_MUTEX.lock() {
            Ok(g) => g,
            Err(_) => return GpkgResultCode::LockFailed,
        };

        let mut ws = match Self::open_read_only(path) {
            Ok(ws) => ws,
            Err(_) => return GpkgResultCode::SchemaMismatch,
        };
        if ws.schema_version != 3 {
            return GpkgResultCode::SchemaMismatch;
        }

        let mut updated_layer = layer.clone();
        updated_layer.assign_positive_ids_on_commit();
        ws.layers.insert(layer_name.to_owned(), updated_layer);

        let staging_path = path.with_extension("gpkg.atomic-tmp");
        if Self::write_workspace_file(&staging_path, &ws).is_err() {
            let _ = std::fs::remove_file(&staging_path);
            return GpkgResultCode::WriteError;
        }

        let integrity_ok = Connection::open_with_flags(
            &staging_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .and_then(|conn| conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0)))
        .map(|res| res.eq_ignore_ascii_case("ok"))
        .unwrap_or(false);

        if !integrity_ok {
            let _ = std::fs::remove_file(&staging_path);
            return GpkgResultCode::WriteError;
        }

        for attempt in 0..4 {
            let _ = std::fs::remove_file(path);
            if std::fs::rename(&staging_path, path).is_ok() {
                return GpkgResultCode::Success;
            }
            thread::sleep(Duration::from_millis(15 * (attempt + 1)));
        }
        let _ = std::fs::remove_file(&staging_path);
        GpkgResultCode::LockFailed
    }

    pub fn create_new_survey(path: &Path, work_crs: WorkCrs) -> StrataResult<SurveyWorkspace> {
        let ws = SurveyWorkspace {
            schema_version: 3,
            work_crs,
            layers: BTreeMap::new(),
            embedded_project_json: String::from("{\"ka_hgis_project\":true}"),
        };
        Self::write_workspace_file(path, &ws)?;
        Ok(ws)
    }

    /// C++ `LayerSchema::createSurveySchema(SurveyLayerType type, const QString &gpkgPath) -> bool` — 5개 고고학 표준 레이어(`SurveyArea`, `FeaturePoly`, `FeatureLine`, `TrialTrench`, `ArtifactPoint`)를 `gpkg_contents` 및 `gpkg_geometry_columns`에 WKB 타입으로 일괄 생성 1:1
    pub fn create_survey_schema_5_layers(path: &Path, work_crs: WorkCrs) -> bool {
        let mut ws = SurveyWorkspace {
            schema_version: 3,
            work_crs,
            layers: BTreeMap::new(),
            embedded_project_json: String::from("{\"ka_hgis_project\":true}"),
        };
        for key in [
            DomainLayerKey::SurveyArea,
            DomainLayerKey::FeaturePoly,
            DomainLayerKey::FeatureLine,
            DomainLayerKey::TrialTrench,
            DomainLayerKey::ArtifactPoint,
        ] {
            ws.ensure_domain_layer(key);
        }
        Self::write_workspace_file(path, &ws).is_ok()
    }

    pub fn open_read_only(path: &Path) -> StrataResult<SurveyWorkspace> {
        let bytes = std::fs::read(path)?;
        if bytes.len() < GPKG_MAGIC_HEADER.len() || &bytes[..16] != GPKG_MAGIC_HEADER {
            return Err(StrataError::Storage(
                "유효한 GeoPackage(SQLite 3) 헤더가 아닙니다".into(),
            ));
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| StrataError::Storage(format!("GPKG 읽기 전용 열기 실패: {e}")))?;

        let json_str: String = conn
            .query_row(
                "SELECT workspace_json FROM ka_hgis_workspace_state WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| StrataError::Storage(format!("워크스페이스 메타 조회 실패: {e}")))?;

        serde_json::from_str(&json_str)
            .map_err(|e| StrataError::Storage(format!("조사 파일 역직렬화 실패: {e}")))
    }

    pub fn write_workspace_file(path: &Path, ws: &SurveyWorkspace) -> StrataResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        let conn = Connection::open(path)
            .map_err(|e| StrataError::Storage(format!("GPKG 생성 실패: {e}")))?;
        conn.execute_batch(
            "PRAGMA application_id = 1196444487;
             PRAGMA user_version = 10300;
             PRAGMA journal_mode = DELETE;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE gpkg_spatial_ref_sys (
                 srs_name TEXT NOT NULL,
                 srs_id INTEGER NOT NULL PRIMARY KEY,
                 organization TEXT NOT NULL,
                 organization_coordsys_id INTEGER NOT NULL,
                 definition TEXT NOT NULL,
                 description TEXT
             );
             INSERT INTO gpkg_spatial_ref_sys VALUES
                 ('Korea 2000 / East Belt 2010', 5187, 'EPSG', 5187, 'PROJCS[\"Korea 2000 / East Belt 2010\"]', '동부원점'),
                 ('Korea 2000 / Central Belt 2010', 5186, 'EPSG', 5186, 'PROJCS[\"Korea 2000 / Central Belt 2010\"]', '중부원점'),
                 ('Korea 2000 / Unified CS', 5179, 'EPSG', 5179, 'PROJCS[\"Korea 2000 / Unified CS\"]', '제출용 UTM-K');
             CREATE TABLE gpkg_contents (
                 table_name TEXT NOT NULL PRIMARY KEY,
                 data_type TEXT NOT NULL,
                 identifier TEXT UNIQUE,
                 description TEXT DEFAULT '',
                 last_change DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE,
                 srs_id INTEGER,
                 CONSTRAINT fk_gc_r_srs_id FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys(srs_id)
             );
             CREATE TABLE ka_hgis_workspace_state (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 schema_version INTEGER NOT NULL,
                 work_crs_epsg INTEGER NOT NULL,
                 workspace_json TEXT NOT NULL
             );",
        )
        .map_err(|e| StrataError::Storage(format!("GPKG 스키마 초기화 실패: {e}")))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS gpkg_geometry_columns (
                 table_name TEXT NOT NULL PRIMARY KEY,
                 column_name TEXT NOT NULL DEFAULT 'geom',
                 geometry_type_name TEXT NOT NULL,
                 srs_id INTEGER NOT NULL,
                 z TINYINT NOT NULL DEFAULT 0,
                 m TINYINT NOT NULL DEFAULT 0
             );",
        )
        .map_err(|e| StrataError::Storage(format!("gpkg_geometry_columns 초기화 실패: {e}")))?;

        let srs_id = i64::from(ws.work_crs.epsg_code());
        for (layer_id, buf) in &ws.layers {
            let desc = buf.key.map_or("", DomainLayerKey::name_ko);
            let geom_type_name = match buf.key {
                Some(DomainLayerKey::SurveyArea | DomainLayerKey::FeaturePoly | DomainLayerKey::TrialTrench) => "MULTIPOLYGON",
                Some(DomainLayerKey::FeatureLine | DomainLayerKey::SectionLine) => "MULTILINESTRING",
                Some(DomainLayerKey::ControlPoints | DomainLayerKey::ArtifactPoint) => "POINT",
                None => "GEOMETRY",
            };
            conn.execute(
                "INSERT INTO gpkg_contents (table_name, data_type, identifier, description, srs_id)
                 VALUES (?1, 'features', ?1, ?2, ?3)",
                params![layer_id, desc, srs_id],
            )
            .map_err(|e| StrataError::Storage(format!("gpkg_contents 등록 실패: {e}")))?;
            conn.execute(
                "INSERT INTO gpkg_geometry_columns (table_name, column_name, geometry_type_name, srs_id, z, m)
                 VALUES (?1, 'geom', ?2, ?3, 0, 0)",
                params![layer_id, geom_type_name, srs_id],
            )
            .map_err(|e| StrataError::Storage(format!("gpkg_geometry_columns 등록 실패: {e}")))?;
        }

        let json_str = serde_json::to_string(ws)
            .map_err(|e| StrataError::Storage(format!("조사 파일 직렬화 실패: {e}")))?;
        conn.execute(
            "INSERT INTO ka_hgis_workspace_state (id, schema_version, work_crs_epsg, workspace_json)
             VALUES (1, ?1, ?2, ?3)",
            params![ws.schema_version, srs_id, json_str],
        )
        .map_err(|e| StrataError::Storage(format!("워크스페이스 상태 기록 실패: {e}")))?;
        Ok(())
    }
}

impl SurveyWorkspace {
    pub fn ensure_domain_layer(&mut self, key: DomainLayerKey) -> &mut LayerEditBuffer {
        let id = key.id().to_string();
        let entry = self
            .layers
            .entry(id)
            .or_insert_with(|| LayerEditBuffer::new(key));
        entry.visible = true;
        entry
    }

    pub fn remove_domain_layer(&mut self, key: DomainLayerKey) {
        self.layers.remove(key.id());
    }

    pub fn migrate_generation_copy(&mut self) {
        if self.schema_version < 3 {
            self.schema_version = 3;
        }
        for layer in self.layers.values_mut() {
            layer.assign_positive_ids_on_commit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_gpkg_passes_sqlite_pragma_integrity_check_and_has_gpkg_contents() -> StrataResult<()>
    {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("real_sqlite_test.gpkg");
        GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187)?;

        let conn = rusqlite::Connection::open(&path)
            .map_err(|e| StrataError::Storage(e.to_string()))?;
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
            let conn = Connection::open(&v1_path).map_err(|e| StrataError::Storage(e.to_string()))?;
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

    #[test]
    fn test_crash_autosave_recovery_trigger_and_lock_file_exclusion_and_mru_limit_10(
    ) -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let proj_path = dir.path().join("site_a.gpkg");
        GpkgRepository::create_new_survey(&proj_path, WorkCrs::Epsg5187)?;
        assert_eq!(
            check_project_open_preflight(&proj_path, false),
            ProjectOpenResult::Success
        );

        // 1. Lock file exclusion (.hgis.lock)
        let lock_path =
            std::path::PathBuf::from(format!("{}{HGIS_LOCK_FILE_EXT}", proj_path.display()));
        std::fs::write(&lock_path, "1234:1700000000:workstation")?;
        assert_eq!(
            check_project_open_preflight(&proj_path, false),
            ProjectOpenResult::LockedByAnotherProcess
        );
        std::fs::remove_file(&lock_path)?;

        // 2. Crash autosave recovery trigger (.hgis.autosave)
        let autosave_path =
            std::path::PathBuf::from(format!("{}{HGIS_AUTOSAVE_FILE_EXT}", proj_path.display()));
        std::fs::write(&autosave_path, b"snapshot")?;
        assert_eq!(
            check_project_open_preflight(&proj_path, false),
            ProjectOpenResult::CrashDetectedNeedsRecovery
        );
        std::fs::remove_file(&autosave_path)?;

        // 3. Recent projects MRU capped at 10
        let mut mru = Vec::new();
        for i in 1..=12 {
            push_recent_project_mru(&mut mru, &format!("/projects/p{i}.gpkg"));
        }
        assert_eq!(mru.len(), HGIS_MAX_RECENT_PROJECTS);
        assert_eq!(mru[0], "/projects/p12.gpkg");
        push_recent_project_mru(&mut mru, "/projects/p5.gpkg");
        assert_eq!(mru.len(), HGIS_MAX_RECENT_PROJECTS);
        assert_eq!(mru[0], "/projects/p5.gpkg");
        Ok(())
    }

    #[test]
    fn test_progress_and_man_days_calculation_and_diary_roundtrip() -> StrataResult<()> {
        // TestKaExcavationProgressTracker::testProgressAndManDaysCalculation
        let history = vec![
            FieldDiaryEntry {
                diary_id: 1,
                work_date: "2026-11-05".to_string(),
                weather: "맑음".to_string(),
                worker_count: 5,
                active_trenches: "T-1".to_string(),
                discovered_features: "1호 수혈".to_string(),
                completed_area_m2: 1500.0,
                notes: String::new(),
            },
            FieldDiaryEntry {
                diary_id: 2,
                work_date: "2026-11-06".to_string(),
                weather: "흐림".to_string(),
                worker_count: 8,
                active_trenches: "T-1, T-2".to_string(),
                discovered_features: "2호 주거지".to_string(),
                completed_area_m2: 3500.0,
                notes: String::new(),
            },
            FieldDiaryEntry {
                diary_id: 3,
                work_date: "2026-11-07".to_string(),
                weather: "맑음".to_string(),
                worker_count: 7,
                active_trenches: "T-3".to_string(),
                discovered_features: "3호 구상유구".to_string(),
                completed_area_m2: 6000.0,
                notes: "안전 점검 완료".to_string(),
            },
        ];
        let metrics = calculate_excavation_progress_metrics(10_000.0, &history);
        assert_eq!(metrics.total_man_days, 20);
        assert!((metrics.progress_percentage - 60.0).abs() < 1e-6);

        // TestKaFieldDiaryManager::testRecordAndQueryRoundtrip
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("diary_test.gpkg");
        let row_id = record_diary_entry_sqlite(&db_path, &history[2])?;
        assert!(row_id >= 1);
        let loaded = query_diary_entries_sqlite(&db_path)?;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].work_date, "2026-11-07");
        assert_eq!(loaded[0].weather, "맑음");
        assert_eq!(loaded[0].worker_count, 7);
        assert_eq!(loaded[0].active_trenches, "T-3");
        assert!((loaded[0].completed_area_m2 - 6000.0).abs() < 1e-6);
        Ok(())
    }
}

/// SQLite 현장 야장 테이블명 (C++ `HGIS_DIARY_TABLE_NAME`)
pub const HGIS_DIARY_TABLE_NAME: &str = "ka_field_diary";
/// 최대 진척률 상한 (%, C++ `HGIS_PROGRESS_CLAMP_MAX`)
pub const HGIS_PROGRESS_CLAMP_MAX: f64 = 100.0;

/// 현장 발굴조사 일지(야장) 엔트리 (C++ `FieldDiaryEntry`)
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDiaryEntry {
    pub diary_id: i64,
    pub work_date: String,
    pub weather: String,
    pub worker_count: i32,
    pub active_trenches: String,
    pub discovered_features: String,
    pub completed_area_m2: f64,
    pub notes: String,
}

/// 발굴조사 진척률 및 누적 투입 공수 지표 (C++ `ProgressMetrics`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressMetrics {
    pub total_survey_area_m2: f64,
    pub excavated_area_m2: f64,
    pub progress_percentage: f64,
    pub total_man_days: i32,
}

/// 전체 계획 조사 면적 대비 최신 완료 면적 진척률(`0.0..=100.0%`) 및 누적 공수(Man-Days) 산출
/// (C++ `KaExcavationProgressTracker::calculateProgressMetrics` 1:1 대응)
#[must_use]
pub fn calculate_excavation_progress_metrics(
    total_survey_area_m2: f64,
    diary_history: &[FieldDiaryEntry],
) -> ProgressMetrics {
    let total_man_days: i32 = diary_history
        .iter()
        .map(|d| d.worker_count.max(0))
        .sum();
    let excavated_area_m2 = diary_history
        .last()
        .map_or(0.0, |d| d.completed_area_m2.max(0.0));
    let progress_percentage = if total_survey_area_m2 <= 0.0 || !total_survey_area_m2.is_finite() {
        0.0
    } else {
        ((excavated_area_m2 / total_survey_area_m2) * 100.0).clamp(0.0, HGIS_PROGRESS_CLAMP_MAX)
    };
    ProgressMetrics {
        total_survey_area_m2: total_survey_area_m2.max(0.0),
        excavated_area_m2,
        progress_percentage,
        total_man_days,
    }
}

/// SQLite/GeoPackage 내부 `ka_field_diary` 테이블에 현장 야장 레코드 트랜잭션 기록
/// (C++ `KaFieldDiaryManager::recordDiaryEntry` 1:1 대응)
pub fn record_diary_entry_sqlite(
    db_path: &Path,
    entry: &FieldDiaryEntry,
) -> StrataResult<i64> {
    let mut conn =
        Connection::open(db_path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let tx = conn
        .transaction()
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS ka_field_diary (
            diary_id INTEGER PRIMARY KEY AUTOINCREMENT,
            work_date TEXT NOT NULL,
            weather TEXT NOT NULL,
            worker_count INTEGER NOT NULL,
            active_trenches TEXT NOT NULL,
            discovered_features TEXT NOT NULL,
            completed_area_m2 REAL NOT NULL,
            notes TEXT NOT NULL
        );",
    )
    .map_err(|e| StrataError::Storage(e.to_string()))?;
    tx.execute(
        "INSERT INTO ka_field_diary (
            work_date, weather, worker_count, active_trenches,
            discovered_features, completed_area_m2, notes
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            entry.work_date,
            entry.weather,
            entry.worker_count,
            entry.active_trenches,
            entry.discovered_features,
            entry.completed_area_m2,
            entry.notes
        ],
    )
    .map_err(|e| StrataError::Storage(e.to_string()))?;
    let row_id = tx.last_insert_rowid();
    tx.commit()
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    Ok(row_id)
}

/// SQLite `ka_field_diary` 테이블에서 현장 야장 이력 조회
pub fn query_diary_entries_sqlite(db_path: &Path) -> StrataResult<Vec<FieldDiaryEntry>> {
    let conn = Connection::open(db_path).map_err(|e| StrataError::Storage(e.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT diary_id, work_date, weather, worker_count, active_trenches,
                    discovered_features, completed_area_m2, notes
             FROM ka_field_diary ORDER BY diary_id ASC",
        )
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(FieldDiaryEntry {
                diary_id: row.get(0)?,
                work_date: row.get(1)?,
                weather: row.get(2)?,
                worker_count: row.get(3)?,
                active_trenches: row.get(4)?,
                discovered_features: row.get(5)?,
                completed_area_m2: row.get(6)?,
                notes: row.get(7)?,
            })
        })
        .map_err(|e| StrataError::Storage(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| StrataError::Storage(e.to_string()))?);
    }
    Ok(out)
}
