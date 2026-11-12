//! 조사 레이어 모양·라벨 규칙이 C++ 값·식과 같은지.

use std::collections::BTreeMap;

use super::*;

fn style(fill: [u8; 4], stroke: [u8; 4], w: f32, m: f32) -> DomainStyle {
    DomainStyle { fill, stroke, stroke_width_mm: w, marker_size_mm: m, casing: None }
}

const BLUE_FILL: [u8; 4] = [37, 99, 235, 90];

/// `LayerStyleDefaults.cpp:41-77` forKey 의 값 그대로.
#[test]
fn style_table_matches_cpp_for_key() {
    assert_eq!(domain_style_for_key("survey_area"), style([180, 83, 9, 70], [146, 64, 14, 255], 1.6, 3.5));
    assert_eq!(domain_style_for_key("feature_poly"), style([22, 163, 74, 90], [17, 94, 44, 255], 1.8, 3.5));
    assert_eq!(domain_style_for_key("feature_line"), style(BLUE_FILL, [202, 138, 4, 255], 1.8, 3.5));
    assert_eq!(
        domain_style_for_key("section_line"),
        DomainStyle { casing: Some(([255, 255, 255, 255], 3.0)), ..style(BLUE_FILL, [190, 24, 93, 255], 1.8, 3.5) }
    );
    assert_eq!(domain_style_for_key("control_points"), style([234, 179, 8, 255], [161, 98, 7, 255], 1.2, 4.0));
    assert_eq!(domain_style_for_key("artifact_point"), style([185, 28, 28, 255], [127, 29, 29, 255], 1.2, 3.6));
    assert_eq!(domain_style_for_key("trial_trench"), style([220, 38, 38, 18], [220, 38, 38, 255], 0.5, 3.5));
    assert_eq!(domain_style_for_key("유적_목록"), style(BLUE_FILL, [37, 99, 235, 255], 1.2, 3.5));
}

/// QGIS `format_number` (한국어 locale: 천 단위 쉼표, 소수 자리 고정).
#[test]
fn format_number_uses_thousands_separators_and_fixed_decimals() {
    assert_eq!(format_number(12_000.0, 2), "12,000.00");
    assert_eq!(format_number(42.0, 2), "42.00");
    assert_eq!(format_number(40.0, 1), "40.0");
    assert_eq!(format_number(1_234_567.891, 2), "1,234,567.89");
    assert_eq!(format_number(-1_234.5, 1), "-1,234.5");
    assert_eq!(format_number(999.0, 0), "999");
    assert_eq!(format_number(0.0, 2), "0.00");
}

fn cols(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(n, t)| ((*n).to_owned(), (*t).to_owned())).collect()
}

/// `LayerOps.cpp:225-232` — 시굴격자는 이름+면적, 면은 면적, 그 밖은 detectNameField 가 고른 항목.
/// 항목은 demo.gpkg 의 표 그대로.
#[test]
fn label_rules_follow_apply_domain_draw_style() {
    use GeomKind::{Line, Point, Polygon};
    let rule = |key: &str, kind, c: &[(&str, &str)]| label_rule(key, kind, &cols(c));
    let text = [("kind", "TEXT"), ("period", "TEXT")];
    assert_eq!(rule("survey_area", Polygon, &[("survey_name", "TEXT"), ("site_name", "TEXT")]), LabelRule::Area2);
    assert_eq!(rule("feature_poly", Polygon, &[("kind", "TEXT"), ("feature_no", "TEXT")]), LabelRule::Area2);
    assert_eq!(rule("feature_line", Line, &[text[0], text[1], ("note", "TEXT")]), LabelRule::Field("kind".into()));
    assert_eq!(rule("section_line", Line, &[("section_id", "TEXT"), ("note", "TEXT")]), LabelRule::Field("section_id".into()));
    assert_eq!(rule("control_points", Point, &[("point_id", "TEXT"), ("x", "REAL"), ("datum", "TEXT")]), LabelRule::Field("point_id".into()));
    assert_eq!(rule("artifact_point", Point, &[text[0], text[1], ("artifact_no", "TEXT")]), LabelRule::Field("kind".into()));
    assert_eq!(rule("trial_trench", Polygon, &[("name", "TEXT"), ("width", "REAL")]), LabelRule::NameWithArea("name".into()));
    assert_eq!(rule("trial_trench", Polygon, &[("width", "REAL")]), LabelRule::Area1);
    // 도메인이 아닌 표: 저장된 모양이 없으면 면만 면적 라벨(`LayerOps.cpp:1907-1908`).
    assert_eq!(rule("user_poly_1", Polygon, &[("memo", "TEXT")]), LabelRule::Area2);
    assert_eq!(rule("도로", Line, &[("memo", "TEXT")]), LabelRule::None);
}

/// detectNameField 의 차례: 유적 후보 → 지적 → 유구 → 지형 → 이름 낱말이 든 글자 항목 → 첫 글자 항목 → 첫 항목.
#[test]
fn detect_name_field_follows_cpp_order() {
    let pick = |c: &[(&str, &str)]| detect_name_field(&cols(c));
    assert_eq!(pick(&[("kind", "TEXT"), ("유적명", "TEXT")]), "유적명");
    assert_eq!(pick(&[("NAME", "TEXT")]), "NAME");
    assert_eq!(pick(&[("kind", "TEXT"), ("jibun", "TEXT")]), "jibun");
    assert_eq!(pick(&[("kind", "TEXT"), ("feature_no", "TEXT")]), "feature_no");
    assert_eq!(pick(&[("x", "REAL"), ("site_title", "TEXT")]), "site_title");
    assert_eq!(pick(&[("x", "REAL"), ("memo", "TEXT"), ("note", "TEXT")]), "memo");
    // 글자 항목이 없으면 첫 항목 — QGIS 는 GPKG 의 fid 를 첫 항목으로 보인다.
    assert_eq!(pick(&[("x", "REAL")]), "fid");
}

fn attrs(list: &[(&str, &str)]) -> BTreeMap<String, String> {
    list.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

/// `LabelOps.cpp:91-100, :226` 의 식이 만드는 글.
#[test]
fn label_text_matches_cpp_expressions() {
    assert_eq!(label_text(&LabelRule::Area2, &attrs(&[]), 12_000.0).as_deref(), Some("12,000.00 ㎡"));
    assert_eq!(
        label_text(&LabelRule::NameWithArea("name".into()), &attrs(&[("name", "TR1")]), 40.0).as_deref(),
        Some("TR1\n(40.0 ㎡)")
    );
    assert_eq!(label_text(&LabelRule::NameWithArea("name".into()), &attrs(&[]), 40.0).as_deref(), Some("\n(40.0 ㎡)"));
    assert_eq!(label_text(&LabelRule::Area1, &attrs(&[]), 40.0).as_deref(), Some("40.0 ㎡"));
    assert_eq!(label_text(&LabelRule::Field("kind".into()), &attrs(&[("kind", "구")]), 0.0).as_deref(), Some("구"));
    assert_eq!(label_text(&LabelRule::Field("kind".into()), &attrs(&[]), 0.0), None);
    assert_eq!(label_text(&LabelRule::None, &attrs(&[("kind", "구")]), 0.0), None);
}

/// 면적만 보이는 라벨은 진한 남색(15,23,42), 이름 라벨은 (31,35,40) — `LabelOps.cpp:120, :236`.
#[test]
fn label_ink_follows_the_label_kind() {
    assert_eq!(label_ink(&LabelRule::Area2), [15, 23, 42]);
    assert_eq!(label_ink(&LabelRule::Field("kind".into())), [31, 35, 40]);
    assert_eq!(label_ink(&LabelRule::NameWithArea("name".into())), [31, 35, 40]);
}

/// mm·pt 를 egui 점으로 — QGIS 캔버스는 논리 96 dpi(화면 배율은 Qt devicePixelRatio = egui
/// pixels_per_point). 글자 크기는 둘 다 em 이라 8 pt = 10.67 점.
#[test]
fn millimetres_and_points_convert_to_egui_points() {
    assert!((mm_to_points(1.6) - 6.047_244).abs() < 1e-4);
    assert!((label_font_size_points(8.0) - 10.666_667).abs() < 1e-4);
    assert!((mm_to_points(LABEL_HALO_MM) - 3.023_622).abs() < 1e-4);
}

/// 반투명 색은 Qt 처럼 sRGB 값에 알파를 곱해 섞는다. egui `from_rgba_unmultiplied` 는 선형 빛에서
/// 곱해 (180,83,9,70) 이 (99,43,2) 가 되어 조사구역이 C++ 보다 훨씬 붉게 보였다(실측 252,207,146).
#[test]
fn translucent_colours_premultiply_like_qt() {
    assert_eq!(qt_premultiplied([180, 83, 9, 70]), [49, 23, 2, 70]);
    assert_eq!(qt_premultiplied([255, 255, 255, 230]), [230, 230, 230, 230]);
    assert_eq!(qt_premultiplied([146, 64, 14, 255]), [146, 64, 14, 255]);
    assert_eq!(qt_premultiplied([220, 38, 38, 0]), [0, 0, 0, 0]);
}
