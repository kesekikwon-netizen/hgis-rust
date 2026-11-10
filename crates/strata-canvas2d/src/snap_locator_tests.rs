//! `snap_locator` 시험 — 설계 문서 Phase 6 스내핑 로케이터.
//!
//! 지도 좌표는 미터, `meters_per_px = 1.0` 으로 두어 px = m 가 되게 한다
//! (허용치 10px = 10m 로 읽으면 계산이 그대로 보인다).

use super::*;
use crate::layer_engine::{Feature, GeometryKind, LayerStage, MapLayer};

const MPP: f64 = 1.0;

/// 사각형 면 (0,0)-(100,100).
fn square_ring() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]
}

/// 레이어 하나짜리 창고.
fn stage1(key: &str, kind: GeometryKind, pts: Vec<[f64; 2]>) -> LayerStage {
    let mut l = MapLayer::new(key, "시험", kind, 0);
    l.features = vec![Feature {
        pts,
        label: String::new(),
    }];
    let mut st = LayerStage::new();
    st.upsert(l);
    st
}

fn square_stage() -> LayerStage {
    stage1("survey_area", GeometryKind::Polygon, square_ring())
}

fn line_stage(pts: Vec<[f64; 2]>) -> LayerStage {
    stage1("section_line", GeometryKind::Line, pts)
}

/// 정점 스냅이 선분 스냅보다 우선한다.
///
/// 커서 `(4,4)` 는 정점 (0,0) 에서 5.66m, 아래쪽 변까지는 4m 로 **선분이 더
/// 가깝다**. 그래도 정점이 이겨야 한다 — 문서 Phase 6 의 "정점·끝점에 선분보다
/// 높은 우선순위".
#[test]
fn vertex_snap_wins_over_nearer_segment() {
    let stage = square_stage();
    let settings = SnapSettings::default();

    let hit = locate([4.0, 4.0], MPP, &stage, &settings, None).expect("스냅 대상");
    assert_eq!(hit.kind, SnapKind::Vertex, "정점이 선분보다 우선해야 한다");
    assert_eq!(hit.vertex_index, Some(0));
    assert_eq!(hit.world, [0.0, 0.0], "변 위 점이 아니라 정점이어야 한다");
    assert!((hit.distance_px - 5.656_854).abs() < 1e-3);
    assert_eq!(hit.layer_key, "survey_area");
}

/// 정확히 같은 거리일 때도 정점이 이긴다.
#[test]
fn vertex_wins_when_distance_ties_with_edge() {
    let stage = line_stage(vec![[0.0, 0.0], [100.0, 0.0]]);
    let settings = SnapSettings::default();

    // 커서가 정점 (0,0) 위 — 정점 거리 0, 선분 거리 0 의 동률.
    let hit = locate([0.0, 0.0], MPP, &stage, &settings, None).expect("정점에 붙어야 한다");
    assert_eq!(hit.kind, SnapKind::Endpoint, "선의 첫 점은 끝점이다");
    assert_eq!(hit.world, [0.0, 0.0]);

    // 가운데 정점이 없는 선: 정점이 허용치 밖이면 선분이 잡힌다.
    let hit = locate([50.0, 3.0], MPP, &stage, &settings, None).expect("선분에 붙어야 한다");
    assert_eq!(hit.kind, SnapKind::Edge);
    assert_eq!(hit.world, [50.0, 0.0]);
    assert!(hit.vertex_index.is_none());
}

/// 허용치 밖은 안 잡힌다 — 10px 경계. 10px 는 잡히고 10.1px 는 안 잡힌다.
#[test]
fn tolerance_boundary_at_ten_pixels() {
    // 점 피처 하나 — 선분이 지나가지 않아 경계가 흐려지지 않는다.
    let stage = stage1("control_points", GeometryKind::Point, vec![[0.0, 0.0]]);
    let settings = SnapSettings::default();
    assert!((settings.clamped_tolerance_px() - 10.0).abs() < 1e-9);

    assert!(
        locate([0.0, 10.0], MPP, &stage, &settings, None).is_some(),
        "정확히 10px 는 잡혀야 한다"
    );
    assert!(
        locate([0.0, 10.1], MPP, &stage, &settings, None).is_none(),
        "10px 를 넘으면 안 잡혀야 한다"
    );

    // 허용치를 20px 로 올리면 같은 자리가 잡힌다.
    let wide = SnapSettings {
        tolerance_px: 20.0,
        ..SnapSettings::default()
    };
    assert!(locate([0.0, 15.0], MPP, &stage, &wide, None).is_some());
    assert!(locate([0.0, 15.0], MPP, &stage, &settings, None).is_none());

    // 선분도 같은 경계를 쓴다 — 끝점은 51m 떨어져 있고 변까지 정확히 10m.
    let seg = line_stage(vec![[-50.0, 0.0], [50.0, 0.0]]);
    let hit = locate([0.0, 10.0], MPP, &seg, &settings, None).expect("선분 경계값");
    assert_eq!(hit.kind, SnapKind::Edge);
    assert!(locate([0.0, 10.2], MPP, &seg, &settings, None).is_none());
}

/// 허용치는 1~100 으로 clamp 한다(문서 Phase 6). 범위 밖 값은 경계로 당긴다.
#[test]
fn tolerance_is_clamped_to_one_through_hundred() {
    let at = |px: f64| {
        SnapSettings {
            tolerance_px: px,
            ..SnapSettings::default()
        }
        .clamped_tolerance_px()
    };
    assert!((at(0.2) - 1.0).abs() < 1e-9);
    assert!((at(500.0) - 100.0).abs() < 1e-9);
    assert!((at(f64::NAN) - SNAP_DEFAULT_TOLERANCE_PX).abs() < 1e-9);
}

/// **가장 중요한 시험** — `exclude` 로 자기 자신은 안 잡힌다.
///
/// 정점을 끌 때 스냅이 자기 피처를 보면 방금 놓은 자리로 되돌아가 움직이지 않는다
/// (C++ `KaVertexEditTool.cpp:225-241` `snapMapPointExcludingTarget`).
#[test]
fn exclude_keeps_dragged_vertex_from_snapping_to_itself() {
    // 레이어 0 = 편집 중(자기 자신), 레이어 1 = 이웃.
    let mut stage = LayerStage::new();
    let mut self_layer = MapLayer::new("survey_area", "조사구역", GeometryKind::Polygon, 0);
    self_layer.features = vec![Feature::polygon(square_ring())];
    stage.upsert(self_layer);
    let mut neighbour = MapLayer::new("feature_poly", "유구 면", GeometryKind::Polygon, 1);
    neighbour.features = vec![Feature::polygon(vec![
        [300.0, 300.0],
        [400.0, 300.0],
        [400.0, 400.0],
        [300.0, 400.0],
    ])];
    stage.upsert(neighbour);
    let settings = SnapSettings::default();

    // 편집 중 피처의 정점 (0,0) 바로 위 — 제외하지 않으면 자기 자신에 붙는다.
    let cursor = [1.0, 1.0];
    let grabbed = locate(cursor, MPP, &stage, &settings, None).expect("자기 자신에 붙는다");
    assert_eq!(grabbed.kind, SnapKind::Vertex);
    assert_eq!(grabbed.layer_key, "survey_area");
    assert_eq!(grabbed.world, [0.0, 0.0], "제외하면 제자리로 돌아간다");

    // 제외하면 자기 자신은 후보에서 빠지고, 허용치 안에 다른 대상이 없어 None.
    assert!(
        locate(cursor, MPP, &stage, &settings, Some((0, 0))).is_none(),
        "자기 자신은 잡히면 안 된다"
    );

    // 이웃 피처는 그대로 잡힌다 — 제외는 그 피처 하나에만 걸린다.
    let hit = locate([300.5, 300.5], MPP, &stage, &settings, Some((0, 0)))
        .expect("이웃 피처에는 붙어야 한다");
    assert_eq!(hit.layer_key, "feature_poly");
    assert_eq!(hit.world, [300.0, 300.0]);

    // 제외는 (레이어, 피처) **쌍**이다 — 같은 레이어의 다른 피처는 살아 있다.
    let mut two = MapLayer::new("survey_area", "조사구역", GeometryKind::Polygon, 0);
    two.features = vec![
        Feature::polygon(square_ring()),
        Feature::polygon(vec![[200.0, 200.0], [300.0, 200.0], [300.0, 300.0], [200.0, 300.0]]),
    ];
    let mut stage2 = LayerStage::new();
    stage2.upsert(two);
    let hit = locate([200.5, 200.5], MPP, &stage2, &settings, Some((0, 0)))
        .expect("같은 레이어의 다른 피처는 잡힌다");
    assert_eq!((hit.feature_index, hit.world), (1, [200.0, 200.0]));
}

/// 숨긴 레이어에는 붙지 않는다.
#[test]
fn hidden_layer_is_not_a_snap_target() {
    let mut stage = square_stage();
    let settings = SnapSettings::default();

    assert!(locate([1.0, 1.0], MPP, &stage, &settings, None).is_some());
    stage.set_visible("survey_area", false);
    assert!(
        locate([1.0, 1.0], MPP, &stage, &settings, None).is_none(),
        "숨긴 레이어는 안 잡혀야 한다"
    );
}

/// 종류 스위치가 각각 먹는다 — 선분이 꺼지면 정점만, 정점이 꺼지면 선분만.
#[test]
fn snap_kind_switches_are_respected() {
    let stage = line_stage(vec![[0.0, 0.0], [100.0, 0.0]]);
    let only = |vertex, endpoint, edge| SnapSettings {
        vertex,
        endpoint,
        edge,
        ..SnapSettings::default()
    };

    let hit = locate([10.0, 1.0], MPP, &stage, &only(false, false, true), None).expect("선분만");
    assert_eq!(hit.kind, SnapKind::Edge);

    let vertex_only = only(true, true, false);
    assert!(
        locate([10.0, 1.0], MPP, &stage, &vertex_only, None).is_none(),
        "선분이 꺼졌고 근처에 끝점이 없다"
    );
    let hit = locate([0.5, 0.5], MPP, &stage, &vertex_only, None).expect("끝점");
    assert_eq!(hit.kind, SnapKind::Endpoint);

    assert!(locate([0.5, 0.5], MPP, &stage, &only(false, false, false), None).is_none());
    let disabled = SnapSettings {
        enabled: false,
        ..SnapSettings::default()
    };
    assert!(locate([0.5, 0.5], MPP, &stage, &disabled, None).is_none());
}

/// 선의 가운데 꼭짓점은 정점, 양 끝은 끝점. 면은 모두 정점.
#[test]
fn line_middle_vertex_is_vertex_and_ends_are_endpoints() {
    let stage = line_stage(vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]]);
    let settings = SnapSettings::default();

    let start = locate([0.5, 0.0], MPP, &stage, &settings, None).expect("시작점");
    assert_eq!((start.kind, start.vertex_index), (SnapKind::Endpoint, Some(0)));

    let middle = locate([50.5, 0.0], MPP, &stage, &settings, None).expect("가운데 점");
    assert_eq!((middle.kind, middle.vertex_index), (SnapKind::Vertex, Some(1)));

    let end = locate([99.5, 0.0], MPP, &stage, &settings, None).expect("끝점");
    assert_eq!((end.kind, end.vertex_index), (SnapKind::Endpoint, Some(2)));

    // 면의 꼭짓점은 끝점이 아니라 정점이다(링은 닫혀 있어 "끝"이 없다).
    let square = square_stage();
    let hit = locate([0.5, 0.5], MPP, &square, &settings, None).expect("면 정점");
    assert_eq!(hit.kind, SnapKind::Vertex);
}

/// 닫는 점(첫 점 복제)이 있는 폴리곤 링은 같은 자리를 두 번 세지 않는다.
#[test]
fn closed_ring_does_not_duplicate_first_vertex() {
    let mut ring = square_ring();
    ring.push([0.0, 0.0]); // 닫는 점
    let stage = stage1("survey_area", GeometryKind::Polygon, ring);
    let settings = SnapSettings::default();

    let hit = locate([0.1, 0.1], MPP, &stage, &settings, None).expect("정점");
    assert_eq!(hit.kind, SnapKind::Vertex);
    assert_eq!(
        hit.vertex_index,
        Some(0),
        "닫는 점이 아니라 첫 점이어야 한다"
    );
}

/// 화면 좌표 — 커서 기준 오프셋과 절대 좌표 두 가지.
#[test]
fn screen_coordinates_are_offset_and_absolute_as_documented() {
    let stage = square_stage();
    let settings = SnapSettings::default();

    // meters_per_px = 2.0 이면 6m 위쪽 = 화면 3px 아래.
    let hit = locate([0.0, 6.0], 2.0, &stage, &settings, None).expect("정점");
    assert_eq!(hit.world, [0.0, 0.0]);
    assert!((hit.screen[0] - 0.0).abs() < 1e-6);
    assert!((hit.screen[1] - 3.0).abs() < 1e-6, "화면 y: {}", hit.screen[1]);
    assert!((hit.distance_px - 3.0).abs() < 1e-6);

    let absolute =
        locate_on_screen([400.0, 300.0], [0.0, 6.0], 2.0, &stage, &settings, None).expect("정점");
    assert!((absolute.screen[0] - 400.0).abs() < 1e-6);
    assert!((absolute.screen[1] - 303.0).abs() < 1e-6);
}

/// 빈 창고·빈 피처·잘못된 입력에서도 패닉하지 않고 `None`.
#[test]
fn empty_and_degenerate_inputs_return_none_without_panic() {
    let settings = SnapSettings::default();
    assert!(locate([0.0, 0.0], MPP, &LayerStage::new(), &settings, None).is_none());

    let empty_feature = stage1("survey_area", GeometryKind::Polygon, Vec::new());
    assert!(locate([0.0, 0.0], MPP, &empty_feature, &settings, None).is_none());

    let one_point = stage1("control_points", GeometryKind::Point, vec![[5.0, 5.0]]);
    let hit = locate([5.5, 5.5], MPP, &one_point, &settings, None).expect("점 피처는 끝점 스냅");
    assert_eq!(hit.kind, SnapKind::Endpoint);

    // mpp 가 0 이나 NaN 이면 px↔m 환산이 불가능하다.
    let stage = square_stage();
    assert!(locate([5.0, 5.0], 0.0, &stage, &settings, None).is_none());
    assert!(locate([5.0, 5.0], f64::NAN, &stage, &settings, None).is_none());
    assert!(locate([f64::NAN, 0.0], MPP, &stage, &settings, None).is_none());
}
