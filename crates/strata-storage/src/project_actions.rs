use crate::cad_csv_io::{extract_dxf_elevation_points, parse_control_points_csv, ControlPointRow};
use crate::gpkg::{GpkgRepository, SurveyWorkspace};
use crate::session_save::persist_workspace;
use std::io::Read;
use std::path::{Path, PathBuf};
use strata_types::error::StrataResult;

pub const HGIS_MAX_RECENT_PROJECTS: usize = 10;
pub const HGIS_SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

#[derive(Debug, Clone, Default)]
pub struct RecentProjectManager {
    pub recent_paths: Vec<PathBuf>,
}

impl RecentProjectManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recent_paths: Vec::new(),
        }
    }

    /// C++ `KaProjectManager::addRecentProject` — 중복 제거 후 최상단(0번 인덱스) 삽입 및 최대 10개 상한 유지
    pub fn add_recent_project(&mut self, project_path: PathBuf) {
        self.recent_paths.retain(|p| p != &project_path);
        self.recent_paths.insert(0, project_path);
        if self.recent_paths.len() > HGIS_MAX_RECENT_PROJECTS {
            self.recent_paths.truncate(HGIS_MAX_RECENT_PROJECTS);
        }
    }

    /// C++ `KaProjectManager::pruneMissingRecentProjects` — 삭제·이동된 경로 자동 제거
    pub fn prune_missing_recent_projects(&mut self) -> Vec<PathBuf> {
        self.recent_paths.retain(|p| p.exists());
        self.recent_paths.clone()
    }

    /// C++ `KaProjectManager::verifyGpkgMagicBytes` — SQLite3 16바이트 헤더 매직(`"SQLite format 3\0"`) 사전 검사
    #[must_use]
    pub fn verify_gpkg_magic_bytes(file_path: &Path) -> bool {
        let Ok(mut file) = std::fs::File::open(file_path) else {
            return false;
        };
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).is_err() {
            return false;
        }
        &header == HGIS_SQLITE_MAGIC
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceFileActions;

impl WorkspaceFileActions {
    pub fn save_and_reopen_gpkg(
        path: &Path,
        ws: &mut SurveyWorkspace,
    ) -> StrataResult<SurveyWorkspace> {
        persist_workspace(ws, path)?;
        GpkgRepository::open_read_only(path)
    }

    pub fn import_dxf_contour_points(dxf_path: &Path) -> StrataResult<Vec<[f64; 3]>> {
        extract_dxf_elevation_points(dxf_path)
    }

    pub fn import_control_points_csv(csv_path: &Path) -> StrataResult<Vec<ControlPointRow>> {
        let bytes = std::fs::read(csv_path)?;
        parse_control_points_csv(&bytes)
    }
}

/// 외장 USB 드라이브 문자가 변경되어 마운트된 경우(예: `"D:/surveys/site.gpkg"` vs `"E:/surveys/site.gpkg"`) 드라이브 문자를 제외한 나머지 경로가 동일한지 판정
/// (C++ `RecentSurveys::sameRestOnOtherDrive` 1:1 대응)
#[must_use]
pub fn same_rest_on_other_drive(path_a: &str, path_b: &str) -> bool {
    let clean = |s: &str| s.trim().replace('\\', "/");
    let x = clean(path_a);
    let y = clean(path_b);
    let has_drive = |s: &str| {
        let b = s.as_bytes();
        b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/'
    };
    if !has_drive(&x) || !has_drive(&y) {
        return false;
    }
    if x.as_bytes()[0].eq_ignore_ascii_case(&y.as_bytes()[0]) {
        return false;
    }
    x[2..].eq_ignore_ascii_case(&y[2..])
}

/// 고고학 조사 7단계 워크플로 단계 (C++ `WorkflowStep`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStep {
    Survey,
    Background,
    SurveyArea,
    Features,
    ControlPoints,
    Review,
    Submission,
}

/// 7단계 워크플로 판정 입력 상태 (C++ `WorkflowGuide::evaluate` 입력)
#[derive(Debug, Clone, Default)]
pub struct WorkflowProjectInput {
    pub survey_name: String,
    pub layer_count: usize,
    pub survey_area_count: usize,
    pub feature_poly_count: usize,
    pub feature_line_count: usize,
    pub control_points_count: usize,
    pub has_datum: bool,
    pub has_ellipsoid: bool,
    pub has_projection: bool,
    pub has_visible_reference_layer: bool,
    pub checklist_error_count: usize,
    pub package_created: bool,
}

/// 7단계 워크플로 개별 단계 판정 결과 (C++ `WorkflowStepState`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStepState {
    pub step: WorkflowStep,
    pub title: &'static str,
    pub completed: bool,
}

/// 프로젝트의 레이어·피처·기준점·검수 현황을 감사하여 7단계 완료 여부를 판정
/// (C++ `WorkflowGuide::evaluate` 1:1 대응)
#[must_use]
pub fn evaluate_workflow_guide(input: &WorkflowProjectInput) -> [WorkflowStepState; 7] {
    let survey_done = !input.survey_name.trim().is_empty()
        || input.layer_count > 0
        || input.survey_area_count > 0;
    let survey_area_done = input.survey_area_count > 0;
    let features_done = input.feature_poly_count > 0 || input.feature_line_count > 0;
    let cp_done = input.control_points_count >= 2
        && input.has_datum
        && input.has_ellipsoid
        && input.has_projection;
    let review_done = input.checklist_error_count == 0;

    [
        WorkflowStepState {
            step: WorkflowStep::Survey,
            title: "1. 새 조사",
            completed: survey_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Background,
            title: "2. 배경·지적도",
            completed: input.has_visible_reference_layer,
        },
        WorkflowStepState {
            step: WorkflowStep::SurveyArea,
            title: "3. 조사구역",
            completed: survey_area_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Features,
            title: "4. 유구·단면선",
            completed: features_done,
        },
        WorkflowStepState {
            step: WorkflowStep::ControlPoints,
            title: "5. GPS 기준점",
            completed: cp_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Review,
            title: "6. 도면 검수",
            completed: review_done,
        },
        WorkflowStepState {
            step: WorkflowStep::Submission,
            title: "7. 제출 패키지",
            completed: input.package_created,
        },
    ]
}

/// 고고학 10대 레이어 역할 및 Z-순서 우선순위 (C++ `LayerRole` & `LayerTreePolicy`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerRole {
    SurveyBoundary,
    ControlPoints,
    ArtifactPoints,
    FeatureBoundaries,
    TrenchGrids,
    TopographicContours,
    CadastralParcels,
    OrthoMosaicDrone,
    HistoricalTiles,
    BaseMapSatellite,
}

impl LayerRole {
    /// 상단(900)에서 최하단(100)까지의 표준 Z-순서 우선순위 반환 (C++ `LayerTreePolicy::getTargetZOrderIndex`)
    #[must_use]
    pub const fn target_z_order(self) -> i32 {
        match self {
            Self::SurveyBoundary => 900,
            Self::ControlPoints => 850,
            Self::ArtifactPoints => 800,
            Self::FeatureBoundaries => 700,
            Self::TrenchGrids => 600,
            Self::TopographicContours => 500,
            Self::CadastralParcels => 400,
            Self::OrthoMosaicDrone => 300,
            Self::HistoricalTiles => 200,
            Self::BaseMapSatellite => 100,
        }
    }
}

/// 레이어 목록을 고고학 표준 Z-순서(조사구역 최상단 ~ 배경지도 최하단)에 따라 정렬
/// (C++ `LayerTreePolicy::enforceStandardHierarchy` 1:1 대응)
pub fn enforce_standard_layer_z_order(layers: &mut [(String, LayerRole)]) {
    layers.sort_by_key(|(_, role)| std::cmp::Reverse(role.target_z_order()));
}

/// 레이어의 고정 그리기 순서 안내 문구 반환 (C++ `LayerTreePolicy::forcedOrderNote` 1:1 대응)
#[must_use]
pub fn forced_layer_order_note_ko(
    layer_name: &str,
    is_basemap: bool,
    is_cadastral: bool,
    drawn_above_labels: bool,
) -> &'static str {
    if layer_name.contains("위성") {
        "그리기 순서 고정: 위성 사진은 목록 순서와 관계없이 맨 아래에 그립니다."
    } else if is_basemap {
        "그리기 순서 고정: 배경 지도는 목록 순서와 관계없이 조사·참조 자료 아래에 그립니다."
    } else if is_cadastral {
        "그리기 순서 고정: 지적 선과 지번은 본 지도에 그리고, 조사 도형은 그 위에 한 번 더 그립니다."
    } else if drawn_above_labels {
        "그리기 순서 고정: 글자를 가리지 않도록 지도 위에 한 번 더 그립니다. 목록에서 아래로 내려도 그림(래스터)보다 위에 보입니다."
    } else {
        ""
    }
}

/// 깨진 레이어 재탐색 지수 백오프 초(`15s -> 30s -> 60s -> 120s -> 240s -> 최대 300s`) 산출
/// (C++ `LayerTreeRecovery::backoffSeconds` 1:1 대응)
#[must_use]
pub fn layer_recovery_backoff_seconds(failures: i32) -> i32 {
    if failures <= 0 {
        return 0;
    }
    let shift = (failures - 1).min(5);
    (15 << shift).min(300)
}

/// 깨진 레이어 소스 복구 결과 (C++ `LayerRecoveryResult`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRecoveryResult {
    pub is_recovered: bool,
    pub old_source_path: PathBuf,
    pub new_resolved_path: PathBuf,
    pub recovery_method: &'static str,
}

/// 이동·드라이브 변경으로 깨진 레이어 파일 경로를 프로젝트 폴더(`project_dir`), 하위 `data/`·`layers/`, `.hgis/cache/` 순으로 탐색하여 자동 복구
/// (C++ `LayerTreeRecovery::repairBrokenSource` 1:1 대응)
#[must_use]
pub fn repair_broken_layer_source(
    old_source_path: &Path,
    project_dir: &Path,
) -> LayerRecoveryResult {
    if old_source_path.exists() {
        return LayerRecoveryResult {
            is_recovered: true,
            old_source_path: old_source_path.to_path_buf(),
            new_resolved_path: old_source_path.to_path_buf(),
            recovery_method: "AlreadyValid",
        };
    }
    let Some(file_name) = old_source_path.file_name() else {
        return LayerRecoveryResult {
            is_recovered: false,
            old_source_path: old_source_path.to_path_buf(),
            new_resolved_path: PathBuf::new(),
            recovery_method: "None",
        };
    };
    let candidates: [(&'static str, PathBuf); 4] = [
        ("SameDirectoryFallback", project_dir.join(file_name)),
        (
            "RelativeSubdirSearch",
            project_dir.join("data").join(file_name),
        ),
        (
            "RelativeSubdirSearch",
            project_dir.join("layers").join(file_name),
        ),
        (
            "CacheDirMatch",
            project_dir.join(".hgis").join("cache").join(file_name),
        ),
    ];
    for (method, cand) in candidates {
        if cand.exists() {
            return LayerRecoveryResult {
                is_recovered: true,
                old_source_path: old_source_path.to_path_buf(),
                new_resolved_path: cand,
                recovery_method: method,
            };
        }
    }
    LayerRecoveryResult {
        is_recovered: false,
        old_source_path: old_source_path.to_path_buf(),
        new_resolved_path: PathBuf::new(),
        recovery_method: "None",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_types::crs::WorkCrs;
    use strata_types::schema::DomainLayerKey;

    #[test]
    fn workspace_file_actions_save_reopen_and_import_dxf_csv_roundtrip() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let gpkg_path = dir.path().join("action_test.gpkg");
        let mut ws = GpkgRepository::create_new_survey(&gpkg_path, WorkCrs::Epsg5187)?;
        let buf = ws.ensure_domain_layer(DomainLayerKey::SurveyArea);
        buf.add_unsaved_feature(
            String::from("POLYGON((218500 359500, 218550 359500, 218550 359550, 218500 359500))"),
            std::collections::BTreeMap::new(),
        );

        let reopened = WorkspaceFileActions::save_and_reopen_gpkg(&gpkg_path, &mut ws)?;
        assert_eq!(reopened.schema_version, 3);
        assert!(reopened
            .layers
            .contains_key(DomainLayerKey::SurveyArea.id()));

        let dxf_path = dir.path().join("sample_contour.dxf");
        std::fs::write(
            &dxf_path,
            "0\nSECTION\n2\nENTITIES\n0\nPOINT\n10\n218500.0\n20\n359500.0\n30\n48.2\n0\nENDSEC\n0\nEOF\n",
        )?;
        let pts = WorkspaceFileActions::import_dxf_contour_points(&dxf_path)?;
        assert_eq!(pts.len(), 1);
        assert!((pts[0][2] - 48.2).abs() < 1e-6);

        let csv_path = dir.path().join("cp.csv");
        std::fs::write(&csv_path, "CP-01,218510.0,359510.0,45.5\n")?;
        let cps = WorkspaceFileActions::import_control_points_csv(&csv_path)?;
        assert_eq!(cps.len(), 1);
        assert_eq!(cps[0].point_id, "CP-01");
        Ok(())
    }

    #[test]
    fn test_mru_max_limit_12_to_10_prune_3_missing_to_7_and_verify_gpkg_magic_bytes(
    ) -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let mut mgr = RecentProjectManager::new();
        for i in 1..=12 {
            let p = dir.path().join(format!("proj{i}.gpkg"));
            GpkgRepository::create_new_survey(&p, WorkCrs::Epsg5186)?;
            mgr.add_recent_project(p);
        }
        assert_eq!(mgr.recent_paths.len(), 10);
        assert!(mgr.recent_paths[0].ends_with("proj12.gpkg"));

        for del_idx in [11, 9, 7] {
            std::fs::remove_file(dir.path().join(format!("proj{del_idx}.gpkg")))?;
        }
        let valid = mgr.prune_missing_recent_projects();
        assert_eq!(valid.len(), 7);

        let txt_path = dir.path().join("corrupt.gpkg");
        std::fs::write(&txt_path, b"Hello World")?;
        assert!(!RecentProjectManager::verify_gpkg_magic_bytes(&txt_path));
        assert!(RecentProjectManager::verify_gpkg_magic_bytes(&valid[0]));

        // RecentSurveys::sameRestOnOtherDrive
        assert!(same_rest_on_other_drive(
            "D:/surveys/gyeongju.gpkg",
            "E:\\surveys\\gyeongju.gpkg"
        ));
        assert!(!same_rest_on_other_drive(
            "D:/surveys/gyeongju.gpkg",
            "D:/surveys/gyeongju.gpkg"
        ));

        // TestWorkflowGuide::testStepEvaluation
        let wf = evaluate_workflow_guide(&WorkflowProjectInput {
            survey_name: "경주 성동동 시굴".to_owned(),
            layer_count: 2,
            survey_area_count: 1,
            feature_poly_count: 0,
            feature_line_count: 0,
            control_points_count: 4,
            has_datum: true,
            has_ellipsoid: true,
            has_projection: true,
            has_visible_reference_layer: true,
            checklist_error_count: 1,
            package_created: false,
        });
        assert!(wf[0].completed); // Step 1: 새 조사
        assert!(wf[1].completed); // Step 2: 배경·지적도
        assert!(wf[2].completed); // Step 3: 조사구역
        assert!(!wf[3].completed); // Step 4: 유구·단면선 미완료
        assert!(wf[4].completed); // Step 5: GPS 기준점 4개 완료

        // TestLayerTreePolicy::testZOrderEnforcement: BaseMapSatellite moved to bottom
        let mut tree = vec![
            ("배경지도".to_owned(), LayerRole::BaseMapSatellite),
            ("유구".to_owned(), LayerRole::FeatureBoundaries),
            ("조사구역".to_owned(), LayerRole::SurveyBoundary),
        ];
        enforce_standard_layer_z_order(&mut tree);
        assert_eq!(tree[0].0, "조사구역");
        assert_eq!(tree.last().map(|x| x.0.as_str()), Some("배경지도"));
        assert!(
            forced_layer_order_note_ko("브이월드 위성", true, false, false)
                .contains("맨 아래에 그립니다")
        );

        // TestLayerTreeRecovery::testSameDirectoryRepair & backoffSeconds
        assert_eq!(layer_recovery_backoff_seconds(1), 15);
        assert_eq!(layer_recovery_backoff_seconds(2), 30);
        assert_eq!(layer_recovery_backoff_seconds(6), 300);

        let moved_file = dir.path().join("features.gpkg");
        std::fs::write(&moved_file, b"SQLite format 3\0")?;
        let broken = PathBuf::from("D:/old_unplugged_usb/features.gpkg");
        let rep = repair_broken_layer_source(&broken, dir.path());
        assert!(rep.is_recovered);
        assert_eq!(rep.new_resolved_path, moved_file);
        assert_eq!(rep.recovery_method, "SameDirectoryFallback");
        Ok(())
    }
}
