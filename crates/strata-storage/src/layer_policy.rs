//! ?덉씠????븷쨌Z-?쒖꽌 ?뺤콉怨?源⑥쭊 ?뚯뒪 蹂듦뎄.
//!
//! `project_actions` ?먯꽌 遺꾨━?덈떎.

use std::path::{Path, PathBuf};

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
