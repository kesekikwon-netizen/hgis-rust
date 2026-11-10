use crate::layer_engine::rasterize::rasterize;
use crate::layer_engine::draw::ScreenTransform;
use crate::layer_engine::renderer::{render_stage, RenderOptions};
use crate::layer_engine::stage::{Feature, GeometryKind, LayerStage, MapLayer};
use crate::layer_engine::style::DomainStyle;

const BG: [u8; 4] = [255, 255, 255, 255];

fn xf() -> ScreenTransform {
    ScreenTransform::new([0.0, 100.0], 1.0, [0.0, 0.0])
}

/// 조사구역 면을 칠하면 그 면적만큼 픽셀이 칠해진다.
#[test]
fn polygon_paints_its_area() {
    let mut st = LayerStage::new();
    let mut l = MapLayer::new("survey_area", "조사구역", GeometryKind::Polygon, 0);
    l.style = DomainStyle::for_key("survey_area");
    // xf: origin=(0,100), 1m/px → world(10,40)~(60,90) → screen(10,10)~(60,60)
    l.features = vec![Feature::polygon(vec![
        [10.0, 90.0],
        [60.0, 90.0],
        [60.0, 40.0],
        [10.0, 40.0],
    ])];
    st.upsert(l);
    let draws = render_stage(&st, &xf(), RenderOptions::default());
    let img = rasterize(&draws, 100, 100, BG);
    let painted = img.count_painted(BG);
    // 50×50 = 2500 (면) + 테두리(굵기 1.6mm ≈ 6px) → 3000 안팎
    assert!(
        (2800..=3500).contains(&painted),
        "칠해진 픽셀 {painted} — 2500(면)+테두리 근처여야 한다"
    );
    // 한가운데는 조사구역 채움색 계열이어야 한다.
    // fill 은 (180,83,9) 에 알파 70 이라 흰 배경과 섞여 밝은 갈색이 된다.
    let center = img.pixel(35, 35).expect("판 안");
    assert!(
        center[0] > center[1] && center[1] > center[2],
        "중심색이 갈색 계열(R>G>B)이어야 한다: {center:?}"
    );
    assert_ne!(center, BG, "배경 그대로면 칠해지지 않은 것");
}

/// 레이어를 끄면 픽셀이 하나도 안 칠해진다 — 이게 회귀 방지의 핵심.
#[test]
fn hidden_layer_paints_nothing() {
    let mut st = LayerStage::new();
    let mut l = MapLayer::new("survey_area", "조사구역", GeometryKind::Polygon, 0);
    l.features = vec![Feature::polygon(vec![
        [10.0, 90.0],
        [60.0, 90.0],
        [60.0, 40.0],
        [10.0, 40.0],
    ])];
    st.upsert(l);
    st.set_visible("survey_area", false);
    let draws = render_stage(&st, &xf(), RenderOptions::default());
    let img = rasterize(&draws, 100, 100, BG);
    assert_eq!(img.count_painted(BG), 0, "숨긴 레이어는 아무것도 안 칠한다");
}

/// z순서가 뒤집히면 위 레이어 색이 이긴다(겹칠 때).
#[test]
fn paint_order_decides_which_color_wins() {
    let mk = |key: &str, z: i32, color: [u8; 4]| {
        let mut l = MapLayer::new(key, "엘", GeometryKind::Polygon, z);
        l.style.fill = color;
        l.style.stroke_width_mm = 0.0;
        l.features = vec![Feature::polygon(vec![
            [10.0, 90.0],
            [90.0, 90.0],
            [90.0, 10.0],
            [10.0, 10.0],
        ])];
        l
    };
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];

    // 파랑(z=10)이 위 → 파랑이 이긴다
    let mut a = LayerStage::new();
    a.upsert(mk("red", 0, red)).upsert(mk("blue", 10, blue));
    let img_a = rasterize(
        &render_stage(&a, &xf(), RenderOptions::default()),
        100,
        100,
        BG,
    );
    let c = img_a.pixel(50, 50).expect("판 안");
    assert!(c[2] > 200 && c[0] < 60, "위 레이어(파랑)가 이겨야: {c:?}");

    // 반대로 하면 빨강이 이긴다
    let mut b = LayerStage::new();
    b.upsert(mk("blue", 0, blue)).upsert(mk("red", 10, red));
    let img_b = rasterize(
        &render_stage(&b, &xf(), RenderOptions::default()),
        100,
        100,
        BG,
    );
    let c2 = img_b.pixel(50, 50).expect("판 안");
    assert!(c2[0] > 200 && c2[2] < 60, "위 레이어(빨강)가 이겨야: {c2:?}");
}

/// 점 레이어는 작고 둥근 얼룩을 남긴다.
#[test]
fn point_paints_a_small_disc() {
    let mut st = LayerStage::new();
    let mut l = MapLayer::new("control_points", "기준점", GeometryKind::Point, 0);
    l.style = DomainStyle::for_key("control_points");
    l.features = vec![Feature::point(50.0, 50.0)];
    st.upsert(l);
    let img = rasterize(
        &render_stage(&st, &xf(), RenderOptions::default()),
        100,
        100,
        BG,
    );
    let n = img.count_painted(BG);
    assert!((80..=260).contains(&n), "점 픽셀 {n} — 작은 원이어야");
    // 노란 계열
    let c = img.pixel(50, 50).expect("판 안");
    assert!(c[0] > 200 && c[1] > 150, "기준점은 노랑: {c:?}");
}
