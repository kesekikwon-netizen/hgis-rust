//! 창고의 레이어를 순서대로 그린다. C++ `QgsMapRenderer` 자리.
//!
//! C++ 은 레이어 트리를 아래에서 위로 그린다. 여기서도 `z_order` 가 작은 것부터
//! 그린다. **그리기 순서를 코드가 아니라 데이터가 정한다** — 이게 지금까지
//! 없던 것이고, 레이어를 끄거나 순서를 바꿀 수 있게 하는 핵심이다.

use super::draw::{DrawShape, LayerDraw, ScreenTransform};
use super::stage::LayerStage;
use super::symbol::{layer_labels, layer_to_shapes, DEFAULT_LABEL_SIZE_PX};

/// 그리기 옵션.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    /// 라벨을 그릴지.
    pub labels: bool,
    /// 라벨 크기(px).
    pub label_size_px: f32,
    /// 선택된 레이어 키(있으면 강조).
    pub selected: Option<usize>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            labels: true,
            label_size_px: DEFAULT_LABEL_SIZE_PX,
            selected: None,
        }
    }
}

/// 창고 전체를 그려 도형 목록을 낸다.
///
/// 돌려주는 순서가 곧 그리는 순서다(아래 → 위). 호출자는 이 순서대로 칠하면 된다.
#[must_use]
pub fn render_stage(
    stage: &LayerStage,
    xf: &ScreenTransform,
    opts: RenderOptions,
) -> Vec<LayerDraw> {
    let mut out = Vec::with_capacity(stage.len());
    for layer in stage.paint_order() {
        let mut shapes = layer_to_shapes(layer, xf);
        if opts.labels {
            shapes.extend(layer_labels(layer, xf, opts.label_size_px));
        }
        if shapes.is_empty() {
            continue;
        }
        let mut d = LayerDraw::new(&layer.key);
        d.shapes = shapes;
        out.push(d);
    }
    out
}

/// 그린 도형 총 개수(진단·시험용).
#[must_use]
pub fn count_shapes(draws: &[LayerDraw]) -> usize {
    draws.iter().map(LayerDraw::len).sum()
}

/// 특정 화면 점을 덮는 **가장 위** 레이어를 찾는다(선택 판정).
///
/// C++ `QgsMapToolSelect` 가 하는 일의 최소 버전. 위에서부터 훑어 처음 맞는 것을 준다.
#[must_use]
pub fn hit_test(draws: &[LayerDraw], at: [f32; 2], tolerance_px: f32) -> Option<usize> {
    for (i, d) in draws.iter().enumerate().rev() {
        for shape in &d.shapes {
            if shape_hits(shape, at, tolerance_px) {
                return Some(i);
            }
        }
    }
    None
}

/// 한 도형이 점을 덮는지.
fn shape_hits(shape: &DrawShape, at: [f32; 2], tol: f32) -> bool {
    match shape {
        DrawShape::Polygon { pts, .. } => point_in_polygon(at, pts),
        DrawShape::Point { at: c, radius_px, .. } => {
            let dx = at[0] - c[0];
            let dy = at[1] - c[1];
            (dx * dx + dy * dy).sqrt() <= radius_px + tol
        }
        DrawShape::Line { pts, width_px, .. } => {
            let t = tol.max(*width_px * 0.5);
            pts.windows(2).any(|w| point_near_segment(at, w[0], w[1], t))
        }
        // 라벨은 선택 대상이 아니다.
        DrawShape::Label { .. } => false,
    }
}

/// 점이 폴리곤 안에 있는지(광선 투사법).
fn point_in_polygon(p: [f32; 2], poly: &[[f32; 2]]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = (poly[i][0], poly[i][1]);
        let (xj, yj) = (poly[j][0], poly[j][1]);
        let crosses = (yi > p[1]) != (yj > p[1]);
        if crosses {
            let x_at = (xj - xi) * (p[1] - yi) / (yj - yi) + xi;
            if p[0] < x_at {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// 점이 선분에서 `tol` 이내인지.
fn point_near_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2], tol: f32) -> bool {
    let abx = b[0] - a[0];
    let aby = b[1] - a[1];
    let apx = p[0] - a[0];
    let apy = p[1] - a[1];
    let len2 = abx * abx + aby * aby;
    if len2 <= f32::EPSILON {
        return (apx * apx + apy * apy).sqrt() <= tol;
    }
    let t = ((apx * abx + apy * aby) / len2).clamp(0.0, 1.0);
    let cx = a[0] + abx * t;
    let cy = a[1] + aby * t;
    let dx = p[0] - cx;
    let dy = p[1] - cy;
    (dx * dx + dy * dy).sqrt() <= tol
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_engine::stage::{Feature, GeometryKind, MapLayer};

    fn xf() -> ScreenTransform {
        ScreenTransform::new([0.0, 0.0], 1.0, [0.0, 0.0])
    }

    fn stage_with_two() -> LayerStage {
        let mut st = LayerStage::new();
        let mut back = MapLayer::new("back", "뒤", GeometryKind::Polygon, 0);
        back.features = vec![Feature::polygon(vec![
            [-100.0, -100.0],
            [100.0, -100.0],
            [100.0, 100.0],
            [-100.0, 100.0],
        ])];
        let mut front = MapLayer::new("front", "앞", GeometryKind::Point, 10);
        front.features = vec![Feature::point(0.0, 0.0)];
        st.upsert(back).upsert(front);
        st
    }

    /// 그리는 순서가 z_order 를 따른다(작은 것 먼저 = 아래).
    #[test]
    fn paint_order_follows_z_order() {
        let st = stage_with_two();
        let draws = render_stage(&st, &xf(), RenderOptions::default());
        let keys: Vec<&str> = draws.iter().map(|d| d.key.as_str()).collect();
        assert_eq!(keys, vec!["back", "front"], "z=0 이 먼저");
    }

    /// 숨긴 레이어는 그리지 않는다 — 이게 없어서 "레이어를 꺼도 안 꺼졌다".
    #[test]
    fn hidden_layer_is_not_drawn() {
        let mut st = stage_with_two();
        st.set_visible("front", false);
        let draws = render_stage(&st, &xf(), RenderOptions::default());
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].key, "back");
    }

    /// 빈 레이어는 그리지 않는다 — 새 조사에 빈 레이어가 남지 않게.
    #[test]
    fn empty_layer_is_not_drawn() {
        let mut st = LayerStage::new();
        st.upsert(MapLayer::new("empty", "빈", GeometryKind::Polygon, 0));
        assert!(render_stage(&st, &xf(), RenderOptions::default()).is_empty());
    }

    /// 히트 테스트는 **가장 위** 레이어를 준다(뒤 폴리곤이 아니라 앞 점).
    #[test]
    fn hit_test_prefers_top_layer() {
        let st = stage_with_two();
        let draws = render_stage(&st, &xf(), RenderOptions::default());
        let hit = hit_test(&draws, [0.0, 0.0], 3.0).expect("점이 있어야 한다");
        assert_eq!(draws[hit].key, "front");
    }

    /// 폴리곤 안쪽/바깥쪽 판정.
    #[test]
    fn point_in_polygon_works() {
        let poly = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(point_in_polygon([5.0, 5.0], &poly));
        assert!(!point_in_polygon([15.0, 5.0], &poly));
        assert!(!point_in_polygon([-1.0, 5.0], &poly));
    }

    /// 라벨은 그리지만 선택 대상은 아니다.
    #[test]
    fn labels_are_drawn_but_not_selectable() {
        let mut st = LayerStage::new();
        let mut l = MapLayer::new("l", "엘", GeometryKind::Polygon, 0);
        l.features = vec![Feature::polygon(vec![
            [200.0, 200.0],
            [210.0, 200.0],
            [210.0, 210.0],
            [200.0, 210.0],
        ])
        .with_label("1㎡")];
        st.upsert(l);
        let draws = render_stage(&st, &xf(), RenderOptions::default());
        // 폴리곤 1 + 라벨 1
        assert_eq!(draws[0].len(), 2);
        // 화면 y 는 뒤집히므로 world(205,205) → screen(205,-205).
        // 폴리곤 안이므로 선택된다(라벨 도형 자체는 선택 대상이 아니지만
        // 같은 자리의 폴리곤이 잡힌다).
        assert!(hit_test(&draws, [205.0, -205.0], 2.0).is_some());
        // 라벨 도형만 있는 곳(폴리곤 밖)은 잡히지 않는다.
        assert!(hit_test(&draws, [400.0, -400.0], 2.0).is_none());
    }
}
