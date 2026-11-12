//! `GpkgRepository` 의 쓰기 쪽 — 원자적 레이어 커밋과 새 조사 생성.
//!
//! `impl GpkgRepository` 가 211줄이라 여기로 나눴다.

use std::collections::BTreeMap;
use std::path::Path;
use std::thread;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::edit_buffer::LayerEditBuffer;
use crate::gpkg_repository::{GpkgRepository, SurveyWorkspace};
use crate::gpkg_schema::{GpkgResultCode, GPKG_REPO_MUTEX};

/// C++ `SurveyProjectFactory.cpp:46-48` 의 거절 문구 그대로.
const SAME_NAME_SURVEY_EXISTS_KO: &str = "같은 이름의 조사 파일이 이미 있습니다. 기존 파일은 그대로 두었습니다. 다른 조사명이나 폴더를 선택해 주세요.";

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

    /// C++ `SurveyProjectFactory.cpp:44-49` — 같은 이름 .gpkg 나 .qgz 가 있으면 아무 파일도
    /// 건드리지 않고 거절한다. 파일은 C++(GDAL)과 같은 형식으로 만든다(:61-170) — Rust
    /// 작업공간 표(`ka_hgis_workspace_state`)는 쓰지 않는다. 돌려주는 값은 빈 메모리 작업공간이다.
    pub fn create_new_survey(path: &Path, work_crs: WorkCrs) -> StrataResult<SurveyWorkspace> {
        if path.exists() || path.with_extension("qgz").exists() {
            return Err(StrataError::InvalidInput(SAME_NAME_SURVEY_EXISTS_KO.to_owned()));
        }
        let ws = SurveyWorkspace {
            schema_version: 3,
            work_crs,
            layers: BTreeMap::new(),
            embedded_project_json: String::from("{\"ka_hgis_project\":true}"),
        };
        crate::survey_gpkg_create::create_cpp_survey_file(path, work_crs)?;
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
}
