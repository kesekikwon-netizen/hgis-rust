//! 스내핑 로케이터 — C++ `QgsSnappingUtils` + `KaVertexEditTool::snapMapPoint` 자리.
//!
//! ## C++ 대응
//!
//! | Rust | C++ |
//! |---|---|
//! | [`locate`] | `KaVertexEditTool.cpp:216` `snapMapPoint` → `snappingUtils()->snapToMap(pos)` |
//! | [`crate::snap_locator::SnapSettings`] | `KaSnapSettingsWidget.cpp:16-39` (자석 켜기·허용치) |
//! | `exclude` 인자 | `KaVertexEditTool.cpp:225-241` `snapMapPointExcludingTarget` |
//!
//! ## 왜 `exclude` 가 꼭 있어야 하는가
//!
//! 정점을 끄는 동안 스냅이 **자기 자신**의 정점·선분을 후보로 보면, 끌어 놓은
//! 좌표가 제자리로 되돌아가 정점이 움직이지 않는다. C++ 은 `ExcludeFeatureFilter`
//! 로 편집 중 피처를 통째로 뺀다. 여기서는 `(레이어 순서 인덱스, 피처 인덱스)` 로
//! 같은 일을 한다 — [`LayerStage::all`] 의 순서가 곧 그 인덱스다.
//!
//! ## 허용치 — 문서 기준 10px (1~100)
//!
//! 설계 문서 Phase 6 은 "커서 주변 허용오차(기본 10픽셀, 1~100픽셀 설정 가능)"를
//! 말한다. 그 값을 여기서 쓴다.
//!
//! **주의 — 그리기 도구와 값이 다르다.** `crates/strata-canvas2d/src/cad_session.rs:33`
//! 은 `KaSnapSettingsWidget.cpp` 를 따라 기본 12px · 범위 4~64px 를 그대로 쓴다.
//! 두 값은 대상이 다르다:
//! - `cad_session`: 그리기 도구(폴리곤·선 그리기)의 커서 스냅
//! - 이 모듈: 정점 편집 핸들의 스냅(문서 Phase 6 기준)
//!
//! 기존 시험이 12px/4~64 를 기댓값으로 잡고 있으므로 그 값을 바꾸지 않는다.
//!
//! ## 공간 질의
//!
//! 직접 만들지 않는다. GeoRust 의 `rstar` 를 쓰는
//! [`strata_geo::snap_index::SnapIndex`] 가 정점·선분 질의를 맡는다.

use strata_geo::snap_index::{SnapIndex, SnapSource};

use crate::layer_engine::{GeometryKind, LayerStage};

/// 기본 허용치(px). 설계 문서 Phase 6.
pub const SNAP_DEFAULT_TOLERANCE_PX: f64 = 10.0;
/// 허용치 하한(px).
pub const SNAP_MIN_TOLERANCE_PX: f64 = 1.0;
/// 허용치 상한(px).
pub const SNAP_MAX_TOLERANCE_PX: f64 = 100.0;

/// 스냅 설정. C++ `KaSnapSettingsWidget` 의 「자석」 묶음 + 문서 Phase 6 의 종류별 스위치.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapSettings {
    /// 스냅을 쓸지. 끄면 [`locate`] 는 늘 `None`.
    pub enabled: bool,
    /// 허용치(px). 쓰기 전에 1~100 으로 clamp 한다([`Self::clamped_tolerance_px`]).
    pub tolerance_px: f64,
    /// 정점(중간 꼭짓점)에 붙을지.
    pub vertex: bool,
    /// 끝점(선의 양 끝·점 피처)에 붙을지.
    pub endpoint: bool,
    /// 선분(변)에 붙을지.
    pub edge: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            tolerance_px: SNAP_DEFAULT_TOLERANCE_PX,
            vertex: true,
            endpoint: true,
            edge: true,
        }
    }
}

impl SnapSettings {
    /// 기본 설정(켜짐, 10px, 세 종류 모두).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 1~100 으로 clamp 한 허용치. `NaN` 이면 기본값.
    #[must_use]
    pub fn clamped_tolerance_px(&self) -> f64 {
        if self.tolerance_px.is_finite() {
            self.tolerance_px
                .clamp(SNAP_MIN_TOLERANCE_PX, SNAP_MAX_TOLERANCE_PX)
        } else {
            SNAP_DEFAULT_TOLERANCE_PX
        }
    }

    /// 붙을 종류가 하나라도 켜져 있는지.
    #[must_use]
    pub fn any_target(&self) -> bool {
        self.vertex || self.endpoint || self.edge
    }
}

/// 스냅 종류. 문서 Phase 6 의 정점(Vertex)·끝점(End)·선분(Edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// 중간 꼭짓점.
    Vertex,
    /// 선의 양 끝 또는 점 피처.
    Endpoint,
    /// 선분(변) 위의 한 점.
    Edge,
}

/// 스냅 결과 한 건.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapHit {
    /// 스냅 종류.
    pub kind: SnapKind,
    /// 붙을 지도 좌표.
    pub world: [f64; 2],
    /// 화면 좌표 — **커서 기준 오프셋**(px, y 는 아래로 증가).
    ///
    /// [`locate`] 에는 화면 변환이 없어 절대 화면 좌표를 알 수 없다. 커서의 화면
    /// 좌표를 아는 호출부는 [`locate_on_screen`] 을 쓰면 절대 좌표를 받는다.
    pub screen: [f32; 2],
    /// 커서에서 스냅 지점까지의 화면 거리(px).
    pub distance_px: f64,
    /// 레이어 키(`survey_area` 등). 레이어 패널·상태 표시에 쓴다.
    pub layer_key: String,
    /// 레이어 안의 피처 순서 인덱스.
    pub feature_index: usize,
    /// 정점·끝점이면 링 안의 정점 번호. 선분이면 `None`.
    pub vertex_index: Option<usize>,
}

/// 커서 주변에서 가장 가까운 스냅 대상을 찾는다.
///
/// 우선순위: **정점/끝점 > 선분**. 같은 등급이면 가까운 것.
/// `exclude` 는 `(레이어 순서 인덱스, 피처 순서 인덱스)` — 보통 편집 중인 자기 자신.
///
/// `meters_per_px` 가 0 이하이거나 유한하지 않으면 `None`(px↔m 환산이 불가능하다).
#[must_use]
pub fn locate(
    cursor_world: [f64; 2],
    meters_per_px: f64,
    stage: &LayerStage,
    settings: &SnapSettings,
    exclude: Option<(usize, usize)>,
) -> Option<SnapHit> {
    if !settings.enabled || !settings.any_target() {
        return None;
    }
    if !meters_per_px.is_finite() || meters_per_px <= 0.0 {
        return None;
    }
    let index = build_index(stage, settings, exclude);
    if index.is_empty() {
        return None;
    }
    let tolerance_world = settings.clamped_tolerance_px() * meters_per_px;
    let hit = index.query(cursor_world, tolerance_world)?;
    let layer = stage.all().get(hit.source.layer_index)?;
    let feature = layer.features.get(hit.source.feature_index)?;
    let (kind, vertex_index) = match hit.kind {
        strata_geo::snap_index::SnapKind::Segment => (SnapKind::Edge, None),
        strata_geo::snap_index::SnapKind::Vertex => (
            classify_vertex(layer.geometry, feature.pts.len(), hit.source.index),
            Some(hit.source.index),
        ),
    };
    Some(SnapHit {
        kind,
        world: hit.point,
        screen: screen_offset(cursor_world, hit.point, meters_per_px),
        distance_px: hit.distance / meters_per_px,
        layer_key: layer.key.clone(),
        feature_index: hit.source.feature_index,
        vertex_index,
    })
}

/// 커서의 **화면 좌표**를 아는 호출부용. [`SnapHit::screen`] 이 절대 화면 좌표가 된다.
#[must_use]
pub fn locate_on_screen(
    cursor_screen: [f32; 2],
    cursor_world: [f64; 2],
    meters_per_px: f64,
    stage: &LayerStage,
    settings: &SnapSettings,
    exclude: Option<(usize, usize)>,
) -> Option<SnapHit> {
    let mut hit = locate(cursor_world, meters_per_px, stage, settings, exclude)?;
    hit.screen = [
        cursor_screen[0] + hit.screen[0],
        cursor_screen[1] + hit.screen[1],
    ];
    Some(hit)
}

/// 창고의 보이는 레이어에서 정점·선분을 모아 색인을 만든다.
///
/// **[`locate`] 는 부를 때마다 이걸 새로 만든다.** 마우스 이동마다 부르는
/// 호출부는 색인을 한 번 만들어 두고 [`strata_geo::snap_index::SnapIndex::query`]
/// 를 직접 부르는 편이 낫다(정점이 바뀌었을 때만 다시 만든다).
#[must_use]
pub fn build_index(
    stage: &LayerStage,
    settings: &SnapSettings,
    exclude: Option<(usize, usize)>,
) -> SnapIndex {
    let mut vertices: Vec<([f64; 2], SnapSource)> = Vec::new();
    let mut segments: Vec<([[f64; 2]; 2], SnapSource)> = Vec::new();
    for (layer_index, layer) in stage.all().iter().enumerate() {
        // 숨긴 레이어에는 붙지 않는다(C++ `snappingUtils` 도 보이는 레이어만 본다).
        if !layer.visible {
            continue;
        }
        for (feature_index, feature) in layer.features.iter().enumerate() {
            if exclude == Some((layer_index, feature_index)) {
                continue;
            }
            let pts = &feature.pts;
            if pts.is_empty() {
                continue;
            }
            // 폐합 링이면 끝에 복제된 첫 점을 정점으로 세지 않는다(같은 자리 두 번).
            let closed = layer.geometry == GeometryKind::Polygon
                && pts.len() >= 2
                && pts.first() == pts.last();
            let n = if closed { pts.len() - 1 } else { pts.len() };
            if n == 0 {
                continue;
            }
            for i in 0..n {
                let kind = classify_vertex(layer.geometry, pts.len(), i);
                let wanted = match kind {
                    SnapKind::Vertex => settings.vertex,
                    SnapKind::Endpoint => settings.endpoint,
                    SnapKind::Edge => false,
                };
                if wanted {
                    let src = SnapSource {
                        layer_index,
                        feature_index,
                        index: i,
                    };
                    vertices.push((pts[i], src));
                }
            }
            if !settings.edge || n < 2 {
                continue;
            }
            let seg_count = if closed { n } else { n - 1 };
            for i in 0..seg_count {
                let a = pts[i];
                let b = pts[(i + 1) % n];
                segments.push((
                    [a, b],
                    SnapSource {
                        layer_index,
                        feature_index,
                        index: i,
                    },
                ));
            }
        }
    }
    let mut index = SnapIndex::new();
    index.rebuild(vertices, segments);
    index
}

/// 정점 번호가 정점인지 끝점인지.
///
/// - 선: 양 끝만 끝점, 가운데는 정점 (C++ `QgsPointLocator::Endpoint`/`Vertex`)
/// - 면: 모두 정점 (링은 닫혀 있어 "끝"이 없다)
/// - 점: 점 하나가 곧 끝점
fn classify_vertex(geometry: GeometryKind, point_count: usize, index: usize) -> SnapKind {
    match geometry {
        GeometryKind::Line => {
            if index == 0 || index + 1 == point_count {
                SnapKind::Endpoint
            } else {
                SnapKind::Vertex
            }
        }
        GeometryKind::Polygon => SnapKind::Vertex,
        GeometryKind::Point => SnapKind::Endpoint,
    }
}

/// 스냅 지점의 커서 기준 화면 오프셋(px). 화면 y 는 아래로 증가하므로 부호를 뒤집는다.
fn screen_offset(cursor_world: [f64; 2], target_world: [f64; 2], meters_per_px: f64) -> [f32; 2] {
    [
        ((target_world[0] - cursor_world[0]) / meters_per_px) as f32,
        -((target_world[1] - cursor_world[1]) / meters_per_px) as f32,
    ]
}

#[cfg(test)]
#[path = "snap_locator_tests.rs"]
mod snap_locator_tests;
