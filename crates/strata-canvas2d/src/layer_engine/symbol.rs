//! 지오메트리 + 스타일 → 그리기 도형. C++ `LayerStyleDefaults::buildSymbol` 대응.
//!
//! C++ 은 QGIS 심볼 객체를 만들어 레이어에 붙인다. Rust 는 심볼을 **도형 목록**으로
//! 바꾼다(`draw::DrawShape`). 그러면 egui·PDF 어디에나 같은 결과를 낼 수 있다.

use super::draw::{
    casing_shape, line_shape, point_shape, polygon_shape, DrawShape, ScreenTransform,
};
use super::stage::{Feature, GeometryKind, MapLayer};

/// 한 레이어의 모든 피처를 화면 도형으로 바꾼다.
///
/// 순서: 케이싱(있으면) → 본체. C++ 도 케이싱 레이어를 먼저 깔고 코어를 얹는다.
#[must_use]
pub fn layer_to_shapes(layer: &MapLayer, xf: &ScreenTransform) -> Vec<DrawShape> {
    let mut out = Vec::with_capacity(layer.features.len() * 2);
    for f in &layer.features {
        push_feature(&mut out, layer.geometry, f, layer, xf);
    }
    out
}

fn push_feature(
    out: &mut Vec<DrawShape>,
    kind: GeometryKind,
    f: &Feature,
    layer: &MapLayer,
    xf: &ScreenTransform,
) {
    let screen_pts: Vec<[f32; 2]> = f.pts.iter().map(|&p| xf.world_to_screen(p)).collect();
    match kind {
        GeometryKind::Polygon => {
            if screen_pts.len() >= 3 {
                out.push(polygon_shape(screen_pts, &layer.style));
            }
        }
        GeometryKind::Line => {
            if screen_pts.len() >= 2 {
                // 케이싱을 먼저 깔고 그 위에 코어를 얹는다(C++ 과 같은 순서).
                if layer.style.use_casing() {
                    out.push(casing_shape(screen_pts.clone(), &layer.style));
                }
                out.push(line_shape(screen_pts, &layer.style));
            }
        }
        GeometryKind::Point => {
            if let Some(&at) = screen_pts.first() {
                out.push(point_shape(at, &layer.style));
            }
        }
    }
}

/// 레이어의 라벨을 도형으로 만든다.
///
/// C++ 은 `applyAreaM2Labels`/`applyNameAttributeLabels` 로 라벨을 붙인다.
/// 라벨 글자는 이미 `Feature::label` 에 들어 있으므로 위치만 정한다.
/// `size_px` 는 C++ `kDefaultLabelSizePt`(9pt) 를 px 로 환산한 값이다.
#[must_use]
pub fn layer_labels(layer: &MapLayer, xf: &ScreenTransform, size_px: f32) -> Vec<DrawShape> {
    let mut out = Vec::new();
    for f in &layer.features {
        if f.label.is_empty() {
            continue;
        }
        let Some(c) = f.center() else {
            continue;
        };
        out.push(super::draw::DrawShape::Label {
            at: xf.world_to_screen(c),
            text: f.label.clone(),
            size_px,
            color: layer.style.stroke,
        });
    }
    out
}

/// 라벨 기본 크기(px). C++ `kDefaultLabelSizePt` = 9pt → 9 * 96/72 = 12px.
pub const DEFAULT_LABEL_SIZE_PX: f32 = 12.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_engine::style::DomainStyle;

    fn xf() -> ScreenTransform {
        ScreenTransform::new([0.0, 0.0], 1.0, [0.0, 0.0])
    }

    fn layer(kind: GeometryKind, features: Vec<Feature>) -> MapLayer {
        let mut l = MapLayer::new("survey_area", "조사구역", kind, 0);
        l.features = features;
        l
    }

    /// 면 레이어는 폴리곤 도형 하나를 낸다.
    #[test]
    fn polygon_layer_makes_polygon_shape() {
        let l = layer(
            GeometryKind::Polygon,
            vec![Feature::polygon(vec![
                [0.0, 0.0],
                [10.0, 0.0],
                [10.0, 10.0],
                [0.0, 10.0],
            ])],
        );
        let s = layer_to_shapes(&l, &xf());
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0], DrawShape::Polygon { .. }));
    }

    /// 점이 2개뿐인 면은 그리지 않는다(퇴화 도형).
    #[test]
    fn degenerate_polygon_is_skipped() {
        let l = layer(GeometryKind::Polygon, vec![Feature::line(vec![[0.0, 0.0], [1.0, 1.0]])]);
        assert!(layer_to_shapes(&l, &xf()).is_empty());
    }

    /// section_line 은 케이싱 + 코어 2개를 낸다(C++ 과 같은 순서).
    #[test]
    fn cased_line_emits_casing_then_core() {
        let mut l = MapLayer::new("section_line", "단면선", GeometryKind::Line, 0);
        l.style = DomainStyle::for_key("section_line");
        l.features = vec![Feature::line(vec![[0.0, 0.0], [100.0, 0.0]])];
        let s = layer_to_shapes(&l, &xf());
        assert_eq!(s.len(), 2, "케이싱 + 코어");
        match (&s[0], &s[1]) {
            (DrawShape::Line { width_px: cw, color: cc, .. },
             DrawShape::Line { width_px: kw, color: kc, .. }) => {
                assert!(*cw > *kw, "케이싱이 더 굵어야 한다: {cw} vs {kw}");
                assert_eq!(*cc, [255, 255, 255, 255], "케이싱은 흰색");
                assert_eq!(*kc, [190, 24, 93, 255], "코어는 C++ 색");
            }
            _ => panic!("둘 다 Line 이어야 한다"),
        }
    }

    /// 점 레이어는 점 도형 하나.
    #[test]
    fn point_layer_makes_point_shape() {
        let mut l = MapLayer::new("control_points", "기준점", GeometryKind::Point, 0);
        l.style = DomainStyle::for_key("control_points");
        l.features = vec![Feature::point(5.0, 5.0).with_label("P1")];
        let s = layer_to_shapes(&l, &xf());
        assert_eq!(s.len(), 1);
        match &s[0] {
            DrawShape::Point { radius_px, fill, .. } => {
                // 4.0mm → 약 15.1px 지름 → 반지름 7.56px
                assert!((*radius_px - 7.56).abs() < 0.2, "반지름 {radius_px}");
                assert_eq!(*fill, [234, 179, 8, 255]);
            }
            _ => panic!("Point 여야 한다"),
        }
    }

    /// 라벨은 글자가 있는 피처에만, 무게중심에 붙는다.
    #[test]
    fn labels_only_where_text_exists() {
        let l = layer(
            GeometryKind::Polygon,
            vec![
                Feature::polygon(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]])
                    .with_label("100㎡"),
                Feature::polygon(vec![[20.0, 0.0], [30.0, 0.0], [30.0, 10.0], [20.0, 10.0]]),
            ],
        );
        let labs = layer_labels(&l, &xf(), DEFAULT_LABEL_SIZE_PX);
        assert_eq!(labs.len(), 1, "글자 없는 피처에는 라벨 없음");
        match &labs[0] {
            DrawShape::Label { at, text, .. } => {
                assert_eq!(text, "100㎡");
                assert!((at[0] - 5.0).abs() < 1e-3, "무게중심 x");
                // y 는 뒤집힘: world y=5 → screen y=-5
                assert!((at[1] + 5.0).abs() < 1e-3, "무게중심 y(뒤집힘)");
            }
            _ => panic!("Label 이어야 한다"),
        }
    }

    /// 화면 y 는 아래로 증가하므로 지도 y 와 부호가 반대다.
    #[test]
    fn screen_y_is_flipped() {
        let xf = ScreenTransform::new([0.0, 100.0], 1.0, [0.0, 0.0]);
        let top = xf.world_to_screen([0.0, 100.0]); // 위쪽
        let bottom = xf.world_to_screen([0.0, 0.0]); // 아래쪽
        assert!(top[1] < bottom[1], "위쪽이 화면 y 가 작아야 한다");
    }
}
