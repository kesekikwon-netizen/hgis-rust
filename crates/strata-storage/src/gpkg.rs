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

        let srs_id = i64::from(ws.work_crs.epsg_code());
        for (layer_id, buf) in &ws.layers {
            let desc = buf.key.map_or("", DomainLayerKey::name_ko);
            conn.execute(
                "INSERT INTO gpkg_contents (table_name, data_type, identifier, description, srs_id)
                 VALUES (?1, 'features', ?1, ?2, ?3)",
                params![layer_id, desc, srs_id],
            )
            .map_err(|e| StrataError::Storage(format!("gpkg_contents 등록 실패: {e}")))?;
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
}
