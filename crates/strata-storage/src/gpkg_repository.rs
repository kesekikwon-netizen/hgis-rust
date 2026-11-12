//! `GpkgRepository`·`SurveyWorkspace` — GeoPackage 열기·쓰기·레이어 관리.
//!
//! `impl GpkgRepository` 가 211줄이라 열기/쓰기 두 갈래로 나눴다.
//! 쓰기 쪽은 `gpkg_repository_write` 에 있다.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::edit_buffer::LayerEditBuffer;
use crate::gpkg_schema::GPKG_MAGIC_HEADER;

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

    /// 조사 파일을 쓸 때 먼저 다 써 두는 같은 폴더의 임시 파일(`<파일 이름>.strata-tmp`).
    pub fn workspace_temp_path(path: &Path) -> PathBuf {
        let mut name = path.file_name().map(OsString::from).unwrap_or_default();
        name.push(".strata-tmp");
        path.with_file_name(name)
    }

    /// 조사 파일을 같은 폴더 임시 파일에 다 쓴 뒤에만 바꿔 넣는다. 쓰다 실패하면 임시
    /// 파일만 지우고 원래 파일은 그대로 둔다(C++ 도 사본에 쓰고 검증한 뒤 교체한다).
    pub fn write_workspace_file(path: &Path, ws: &SurveyWorkspace) -> StrataResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // 임시 방패: Rust 작업공간 표가 없는 기존 파일(C++ 조사 파일 등)은 덮어쓰지 않는다.
        // 지시 #2 에서 파일 형식을 C++ 과 맞출 때까지 C++ 원본을 지키려는 것이다.
        if path.exists() && !Self::has_workspace_table(path) {
            return Err(StrataError::Storage(
                "Strata 작업공간 정보가 없는 기존 파일(예전 판 조사 파일 등)이라 덮어쓰지 않았습니다. \
                 원래 파일은 그대로 두었습니다. 다른 이름으로 저장해 주세요."
                    .into(),
            ));
        }
        let tmp = Self::workspace_temp_path(path);
        if tmp.exists() {
            std::fs::remove_file(&tmp)?;
        }
        let written = Self::write_workspace_tables(&tmp, ws)
            .and_then(|()| std::fs::rename(&tmp, path).map_err(StrataError::from));
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        written
    }

    /// `path` 가 Rust 작업공간 표(`ka_hgis_workspace_state`)를 가진 SQLite 파일인지.
    fn has_workspace_table(path: &Path) -> bool {
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .and_then(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master \
                 WHERE type = 'table' AND name = 'ka_hgis_workspace_state'",
                [],
                |r| r.get::<_, i64>(0),
            )
        })
        .is_ok_and(|n| n > 0)
    }

    /// 새 파일 `path` 에 GPKG 표와 작업공간 상태를 쓴다. 연결은 끝날 때 닫힌다
    /// (Windows 에서는 열린 연결이 있으면 바꿔 넣기가 실패한다).
    fn write_workspace_tables(path: &Path, ws: &SurveyWorkspace) -> StrataResult<()> {
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
                Some(
                    DomainLayerKey::SurveyArea
                    | DomainLayerKey::FeaturePoly
                    | DomainLayerKey::TrialTrench,
                ) => "MULTIPOLYGON",
                Some(DomainLayerKey::FeatureLine | DomainLayerKey::SectionLine) => {
                    "MULTILINESTRING"
                }
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
