//! 조사 레이어의 기본 모양과 라벨 — C++ `LayerStyleDefaults::forKey`(`LayerStyleDefaults.cpp:41-77`),
//! `LayerOps::applyDomainDrawStyle`(`LayerOps.cpp:178-235`), `LabelOps.cpp`(applyNameAttributeLabels
//! :67-130, applyAreaM2Labels :217-245), `LayerOps::detectNameField`(`LayerOps.cpp:239-310`).

use std::collections::BTreeMap;

/// QGIS 지도 캔버스는 논리 96 dpi 로 mm 를 그린다(`qgis/respect_screen_dpi` 꺼짐). 화면 배율은
/// Qt devicePixelRatio 가 곱하므로 egui 점(pixels_per_point 가 같은 배율)과 1:1 이다.
/// 실측: C++ 화면(배율 1.5)에서 조사구역 테두리 1.6 mm = 9 px, 유구(선) 1.8 mm = 10 px.
const POINTS_PER_MM: f32 = 96.0 / 25.4;
/// 글자 크기는 egui 도 Qt 처럼 em 이다(epaint `fonts.rs:843-844` 가 높이/em 비율을 되돌린다).
/// 실측: C++ 「12,000.00 ㎡」 폭 90 px(배율 1.5) = Rust 같은 글.
const POINTS_PER_PT: f32 = 96.0 / 72.0;

/// `LayerOps.h:93` kDefaultLabelSizePt.
pub(crate) const LABEL_SIZE_PT: f32 = 8.0;
/// `LabelOps.cpp:40-43` 라벨 둘레 흰 테(0.8 mm, 흰색 알파 230).
pub(crate) const LABEL_HALO_MM: f32 = 0.8;
pub(crate) const LABEL_HALO_RGBA: [u8; 4] = [255, 255, 255, 230];
/// `LayerStyleDefaults.cpp:137` 점 기호 테두리 굵기.
pub(crate) const MARKER_OUTLINE_MM: f32 = 0.6;

pub(crate) fn mm_to_points(mm: f32) -> f32 {
    mm * POINTS_PER_MM
}

/// Qt 글자 크기(pt) → egui 글자 크기(점).
pub(crate) fn label_font_size_points(pt: f32) -> f32 {
    pt * POINTS_PER_PT
}

/// Qt 처럼 sRGB 값에 알파를 곱한 색(egui 의 premultiplied 로 그대로 넘긴다). egui
/// `Color32::from_rgba_unmultiplied` 는 선형 빛에서 곱해 반투명 채움이 C++ 보다 진해진다.
pub(crate) fn qt_premultiplied(c: [u8; 4]) -> [u8; 4] {
    let a = u16::from(c[3]);
    let m = |v: u8| ((u16::from(v) * a + 127) / 255) as u8;
    [m(c[0]), m(c[1]), m(c[2]), c[3]]
}

/// 레이어 기본 모양 한 벌(색은 RGBA).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DomainStyle {
    pub(crate) fill: [u8; 4],
    pub(crate) stroke: [u8; 4],
    pub(crate) stroke_width_mm: f32,
    pub(crate) marker_size_mm: f32,
    /// 단면선의 흰 덧선(색, 굵기 mm).
    pub(crate) casing: Option<([u8; 4], f32)>,
}

/// 도형 형식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeomKind {
    Point,
    Line,
    Polygon,
}

impl GeomKind {
    /// gpkg_geometry_columns 의 형식 이름에서.
    pub(crate) fn from_type_name(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().trim_start_matches("MULTI") {
            "POINT" => Some(Self::Point),
            "LINESTRING" | "CURVE" | "COMPOUNDCURVE" => Some(Self::Line),
            "POLYGON" | "SURFACE" | "CURVEPOLYGON" => Some(Self::Polygon),
            _ => None,
        }
    }
}

/// 라벨 글을 만드는 규칙(C++ 라벨 식).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum LabelRule {
    /// `coalesce("f", '') || '\n(' || format_number(area($geometry), 1) || ' ㎡)'` (`LabelOps.cpp:91-93`)
    NameWithArea(String),
    /// `format_number(area($geometry), 1) || ' ㎡'` (`LabelOps.cpp:97-99`)
    Area1,
    /// `format_number(area($geometry), 2) || ' ㎡'` (`LabelOps.cpp:226`)
    Area2,
    /// `"f"` (`LabelOps.cpp:94-96`)
    Field(String),
    /// 면적을 끄고 항목도 없는 빈 식 `''`(LayerLabelControls.cpp:57-62 managedExpression) — 아무것도 그리지 않는다.
    Empty,
    None,
}

/// LayerLabelControls replaceManagedContent 가 쓰는 식(managedExpression, :55-62) — 면적+항목·면적만·항목·빈 식.
pub(crate) fn managed_rule(field: &str, area: bool) -> LabelRule {
    match (area, field.is_empty()) {
        (true, false) => LabelRule::NameWithArea(field.to_owned()),
        (true, true) => LabelRule::Area1,
        (false, true) => LabelRule::Empty,
        (false, false) => LabelRule::Field(field.to_owned()),
    }
}

/// `LayerStyleDefaults::forKey` — 모르는 표는 일반 파란색.
pub(crate) fn domain_style_for_key(key: &str) -> DomainStyle {
    let mut s = DomainStyle {
        fill: [37, 99, 235, 90],
        stroke: [37, 99, 235, 255],
        stroke_width_mm: 1.2,
        marker_size_mm: 3.5,
        casing: None,
    };
    match key {
        "survey_area" => {
            s.fill = [180, 83, 9, 70];
            s.stroke = [146, 64, 14, 255];
            s.stroke_width_mm = 1.6;
        }
        "feature_poly" => {
            s.fill = [22, 163, 74, 90];
            s.stroke = [17, 94, 44, 255];
            s.stroke_width_mm = 1.8;
        }
        "feature_line" => {
            s.stroke = [202, 138, 4, 255];
            s.stroke_width_mm = 1.8;
        }
        "section_line" => {
            s.stroke = [190, 24, 93, 255];
            s.stroke_width_mm = 1.8;
            s.casing = Some(([255, 255, 255, 255], 3.0));
        }
        "control_points" => {
            s.fill = [234, 179, 8, 255];
            s.stroke = [161, 98, 7, 255];
            s.marker_size_mm = 4.0;
        }
        "artifact_point" => {
            s.fill = [185, 28, 28, 255];
            s.stroke = [127, 29, 29, 255];
            s.marker_size_mm = 3.6;
        }
        "trial_trench" => {
            s.fill = [220, 38, 38, 18];
            s.stroke = [220, 38, 38, 255];
            s.stroke_width_mm = 0.5;
        }
        _ => {}
    }
    s
}

/// QGIS `format_number(value, places)` — 한국어 locale 의 천 단위 쉼표, 소수 자리 고정.
pub(crate) fn format_number(value: f64, decimals: usize) -> String {
    let fixed = format!("{:.*}", decimals, value.abs());
    let (int_part, frac) = fixed.split_once('.').map_or((fixed.as_str(), None), |(i, f)| (i, Some(f)));
    let mut grouped = String::new();
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if value < 0.0 && fixed.chars().any(|c| c.is_ascii_digit() && c != '0') { "-" } else { "" };
    match frac {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

fn is_text_type(decl: &str) -> bool {
    let t = decl.to_ascii_uppercase();
    t.starts_with("TEXT") || t.contains("CHAR") || t.contains("STRING") || t.contains("CLOB")
}

/// `LayerOps::detectNameField` — QGIS 는 GPKG 의 fid 를 첫 항목으로 보이므로 맨 끝 대안은 "fid".
/// `columns` 는 fid·도형 칸을 뺀 (이름, 선언 형식), 표 순서.
pub(crate) fn detect_name_field(columns: &[(String, String)]) -> String {
    const GROUPS: [&[&str]; 4] = [
        &[
            "사업명", "유적명", "유적명칭", "조사명", "보고서명", "소재지", "번호", "유적번호", "site_no",
            "yujuk_nm", "yujeok_nm", "site_name", "hist_nm", "rem_nm", "명칭", "name", "title",
        ],
        &["jibun", "지번", "a2", "a1", "pnu"],
        &["feature_no", "유구번호", "유구명", "호수", "kind_ko", "kind"],
        &["buld_nm", "건물명", "지명", "road_nm", "도로명", "label", "kor_nm"],
    ];
    // QgsFields::lookupField — 같은 이름 먼저, 없으면 대소문자 무시.
    let lookup = |want: &str| {
        columns
            .iter()
            .find(|(n, _)| n == want)
            .or_else(|| columns.iter().find(|(n, _)| n.eq_ignore_ascii_case(want)))
            .map(|(n, _)| n.clone())
    };
    for group in GROUPS {
        if let Some(found) = group.iter().find_map(|c| lookup(c)) {
            return found;
        }
    }
    let name_like = ["명", "name", "title", "label", "지번", "jibun"];
    if let Some((n, _)) = columns
        .iter()
        .find(|(n, t)| is_text_type(t) && name_like.iter().any(|w| n.to_lowercase().contains(w)))
    {
        return n.clone();
    }
    if let Some((n, _)) = columns.iter().find(|(_, t)| is_text_type(t)) {
        return n.clone();
    }
    "fid".to_owned()
}

/// `applyDomainDrawStyle`(:225-232)이 처음 정하는 라벨. 도메인이 아닌 표는 저장된 모양이 없을 때
/// 면만 면적 라벨(`LayerOps.cpp:1907-1908` applyAreaM2Labels).
pub(crate) fn label_rule(key: &str, kind: GeomKind, columns: &[(String, String)]) -> LabelRule {
    let domain = strata_types::schema::DomainLayerKey::from_id(key).is_some();
    if !domain {
        return if kind == GeomKind::Polygon { LabelRule::Area2 } else { LabelRule::None };
    }
    if key == "trial_trench" {
        // applyNameAttributeLabels(layer, "name", 8, true) — 항목이 없으면 면적만(:97-99).
        let has_name = columns.iter().any(|(n, _)| n == "name");
        return match (has_name, kind) {
            (true, GeomKind::Polygon) => LabelRule::NameWithArea("name".to_owned()),
            (true, _) => LabelRule::Field("name".to_owned()),
            (false, GeomKind::Polygon) => LabelRule::Area1,
            (false, _) => LabelRule::Field(detect_name_field(columns)),
        };
    }
    if kind == GeomKind::Polygon {
        return LabelRule::Area2;
    }
    LabelRule::Field(detect_name_field(columns))
}

/// 라벨 식이 만드는 글. 항목 값이 없으면(NULL) 라벨을 그리지 않는다.
pub(crate) fn label_text(rule: &LabelRule, attrs: &BTreeMap<String, String>, area_m2: f64) -> Option<String> {
    match rule {
        LabelRule::NameWithArea(f) => Some(format!(
            "{}\n({} ㎡)",
            attrs.get(f).map_or("", String::as_str),
            format_number(area_m2, 1)
        )),
        LabelRule::Area1 => Some(format!("{} ㎡", format_number(area_m2, 1))),
        LabelRule::Area2 => Some(format!("{} ㎡", format_number(area_m2, 2))),
        LabelRule::Field(f) => attrs.get(f).cloned(),
        LabelRule::Empty | LabelRule::None => None,
    }
}

/// 면적만 보이는 라벨(applyAreaM2Labels)은 (15,23,42), 이름 라벨(applyNameAttributeLabels)은 (31,35,40).
pub(crate) fn label_ink(rule: &LabelRule) -> [u8; 3] {
    match rule {
        LabelRule::Area2 => [15, 23, 42],
        _ => [31, 35, 40],
    }
}

#[cfg(test)]
#[path = "survey_style_tests.rs"]
mod tests;
