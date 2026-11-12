//! `GpkgRepository` 의 쓰기 쪽 — 새 조사 생성과 5 레이어 스키마.
//!
//! `impl GpkgRepository` 가 211줄이라 여기로 나눴다. 조사 저장은 `survey_gpkg_save` 하나로 한다
//! (예전 commit_layer_atomic 은 원본을 먼저 지우고 이름을 바꿨다 — 지시 2C 에서 없앴다).

use std::collections::BTreeMap;
use std::path::Path;

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::gpkg_repository::{GpkgRepository, SurveyWorkspace};

/// C++ `SurveyProjectFactory.cpp:46-48` 의 거절 문구 그대로.
const SAME_NAME_SURVEY_EXISTS_KO: &str = "같은 이름의 조사 파일이 이미 있습니다. 기존 파일은 그대로 두었습니다. 다른 조사명이나 폴더를 선택해 주세요.";

impl GpkgRepository {

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
