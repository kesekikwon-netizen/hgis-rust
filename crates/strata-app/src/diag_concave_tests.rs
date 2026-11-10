//! 오목 다각형이 **화면(egui)** 과 **검증(rasterize)** 에서 같은 결과를 내는지 확인한다.
//!
//! ## 배경
//!
//! `map_engine_painter.rs` 가 `egui::Shape::convex_polygon` 만 쓰던 시절, epaint 의
//! 채움은 첫 꼭짓점 삼각팬이라 **오목한 부분이 메워졌다**. 반면 `layer_engine::rasterize`
//! 는 자체 스캔라인이라 오목도 정확했다. 그래서 화면과 시험이 서로 다른 결과를 냈다 —
//! 시험은 통과하는데 화면은 틀린 상태였다.
//!
//! 실제로 이런 도형이 나온다: L자 시굴 트렌치(`strata_geo::trench_lshape::build_lshape_ring`),
//! 클릭으로 그린 조사구역(`cad_session` 은 볼록성 검사를 하지 않는다), GPKG 필지 링.
//!
//! ## 이 파일이 고정하는 것
//!
//! 1. 오목 다각형을 egui 메시로 칠하면 면적이 원래 다각형과 같다(파인 곳이 안 메워진다).
//! 2. 검증 경로(rasterize)도 같은 면적을 낸다 → 화면 = 시험.
//! 3. L자 트렌치 실제 링이 오목임을 확인한다(회귀 감시).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_canvas2d::layer_engine::{
    is_concave, rasterize, render_stage, triangulate_polygon, DomainStyle, Feature, GeometryKind,
    LayerStage, MapLayer, RenderOptions, ScreenTransform, ScreenTriangle,
};

/// ㄷ자(오목) 다각형 — 위쪽 가운데가 파여 있다.
///
/// ```text
///  ┌─────┐
///  │     │
///  │ ┌─┐ │   ← 파인 홈 (30..70 x, 30..100 y)
///  │ │ │ │
///  └─┘ └─┘
/// ```
fn u_shape() -> Vec<[f64; 2]> {
    vec![
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 100.0],
        [70.0, 100.0],
        [70.0, 30.0],
        [30.0, 30.0],
        [30.0, 100.0],
        [0.0, 100.0],
    ]
}

fn ident(p: [f64; 2]) -> [f32; 2] {
    [p[0] as f32, p[1] as f32]
}

fn tri_area(t: &ScreenTriangle) -> f32 {
    ((t.b[0] - t.a[0]) * (t.c[1] - t.a[1]) - (t.b[1] - t.a[1]) * (t.c[0] - t.a[0])).abs() * 0.5
}

fn stage_with(ring: Vec<[f64; 2]>) -> LayerStage {
    let mut st = LayerStage::new();
    let mut l = MapLayer::new("survey_area", "조사구역", GeometryKind::Polygon, 0);
    l.style = DomainStyle::for_key("survey_area");
    l.features = vec![Feature::polygon(ring)];
    st.upsert(l);
    st
}

/// 화면 y 는 뒤집히므로 origin.y=100 이면 화면상 0..100 이 된다.
fn xf() -> ScreenTransform {
    ScreenTransform::new([0.0, 100.0], 1.0, [0.0, 0.0])
}

/// 1) ㄷ자는 정말 오목하다 — 이 전제가 틀리면 아래 시험들이 의미 없다.
#[test]
fn u_shape_is_concave() {
    assert!(is_concave(&u_shape()), "ㄷ자 모양은 오목이어야 한다");
    let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    assert!(!is_concave(&square), "정사각형은 볼록이어야 한다");
}

/// 2) **핵심**: 화면이 쓰는 삼각분할 결과의 면적 == 다각형 실제 면적.
///
/// 예전 `convex_polygon` 경로라면 삼각팬이 홈을 메워 면적이 10000 이 됐을 것이다.
#[test]
fn screen_mesh_area_matches_polygon_not_convex_hull() {
    let ring = u_shape();
    let tris = triangulate_polygon(&ring, &[], ident);
    assert!(!tris.is_empty(), "삼각형이 나와야 한다");

    let sum: f32 = tris.iter().map(tri_area).sum();
    // ㄷ자 = 10000 - 40*70 = 7200
    assert!(
        (sum - 7200.0).abs() < 0.5,
        "삼각분할 면적 {sum} 이 7200 이어야 한다(파인 홈이 메워지면 10000 이 된다)"
    );
}

/// 3) 검증 경로(rasterize)도 오목을 정확히 칠한다 — 화면과 같은 기준.
#[test]
fn rasterizer_agrees_that_notch_is_empty() {
    let draws = render_stage(&stage_with(u_shape()), &xf(), RenderOptions::default());
    let r = rasterize(&draws, 100, 100, [255, 255, 255, 255]);

    // world(50,10) -> screen(50,90): 아래 허리는 칠해져야 한다.
    let bottom = r.pixel(50, 90).expect("in bounds");
    assert_ne!(bottom[..3], [255, 255, 255], "아래 허리는 조사구역 색");

    // world(50,70) -> screen(50,30): 파인 홈은 비어야 한다.
    let notch = r.pixel(50, 30).expect("in bounds");
    assert_eq!(
        notch[..3],
        [255, 255, 255],
        "파인 홈은 비어 있어야 한다 — 화면이 홈을 메우면 여기와 어긋난다"
    );

    // world(10,50) -> screen(10,50): 왼쪽 다리는 칠해져야 한다.
    let left = r.pixel(10, 50).expect("in bounds");
    assert_ne!(left[..3], [255, 255, 255], "왼쪽 다리도 칠해져야 한다");
}

/// 4) L자 시굴 트렌치가 실제로 오목하다 — 화면 버그가 실재했음을 고정한다.
///
/// `build_lshape_ring` 은 네 방향 모두 ㄱ자다. ㄱ자·ㄷ자는 오목이다.
#[test]
fn real_lshape_trench_is_concave() {
    for bend in 0..4 {
        let ring = strata_geo::trench_lshape::build_lshape_ring(
            200_000.0,
            500_000.0,
            20.0, // arm
            10.0, // width
            bend,
            Some(5.0), // grid_step
        );
        assert!(
            is_concave(&ring),
            "bend={bend} L자 트렌치가 오목이어야 한다 — 화면에서 파인 곳이 메워지던 원인"
        );
        // 삼각분할 면적이 ㄱ자 실제 면적과 같아야 한다.
        let tris = triangulate_polygon(&ring, &[], ident);
        assert!(!tris.is_empty(), "bend={bend} 삼각형이 나와야 한다");
    }
}

/// 5) 볼록 다각형은 예전 경로 그대로여도 정확하다(회귀 방지).
#[test]
fn convex_polygon_still_fills_fully() {
    let square = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    assert!(!is_concave(&square));
    let draws = render_stage(&stage_with(square), &xf(), RenderOptions::default());
    let r = rasterize(&draws, 100, 100, [255, 255, 255, 255]);
    let center = r.pixel(50, 50).expect("in bounds");
    assert_ne!(center[..3], [255, 255, 255], "정사각형 가운데는 칠해져야 한다");
}
