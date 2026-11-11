//! 조사 레이어의 글자(라벨)를 지도에 그린다.
//!
//! C++ 은 QGIS 의 PAL 이 글자를 놓는다. 기본 규칙은 `LayerOps.cpp:226-233` 과
//! `LabelOps.cpp:30-46,104-118` 이다:
//! - 시굴격자: 이름 + 줄바꿈 + (면적 1자리 ㎡)
//! - 그 밖의 면(조사구역·유구 면): 면적 2자리 ㎡
//! - 선·점: 이름 속성(단면선 번호, 기준점 이름, 유물 번호, 유구 번호)
//! - 굵은 맑은 고딕 8 pt, 글자색 (31,35,40), 흰 테두리 0.8 mm
//! - 면은 가운데, 선은 선 위, 점은 옆. 글자끼리 겹치면 나중 것을 그리지 않는다.
//!
//! PAL 만큼 정교하게 자리를 옮겨 가며 찾지는 않는다 — 겹치면 건너뛴다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Vec2};

use strata_storage::survey_doc::SurveyFeature;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::real_survey::{ring_area_m2, ring_centroid};

use super::gui_tab_map_geom::w2s;

/// 8 pt 를 화면 픽셀로(96 DPI).
const LABEL_PX: f32 = 8.0 * 96.0 / 72.0;
/// 흰 테두리 0.8 mm.
const HALO_PX: f32 = 0.8 * 96.0 / 25.4;
const INK: Color32 = Color32::from_rgb(31, 35, 40);

/// 천 단위 쉼표를 넣은 숫자(QGIS `format_number`).
#[must_use]
pub(crate) fn format_number(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let (int_part, frac_part) = text.split_once('.').unwrap_or((&text, ""));
    let (sign, digits) = int_part.strip_prefix('-').map_or(("", int_part), |d| ("-", d));
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if frac_part.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{frac_part}")
    }
}

fn polygon_area(rings: &[Vec<[f64; 2]>]) -> f64 {
    let outer = rings.first().map_or(0.0, |r| ring_area_m2(r));
    outer - rings.iter().skip(1).map(|r| ring_area_m2(r)).sum::<f64>()
}

/// 이 도형에 붙일 글자. 붙일 것이 없으면 `None`.
#[must_use]
pub(crate) fn label_text(key: DomainLayerKey, feature: &SurveyFeature) -> Option<String> {
    let named = |column: &str| Some(feature.text(column)).filter(|t| !t.trim().is_empty());
    match (key, feature.geom.as_ref()?) {
        (DomainLayerKey::TrialTrench, WkbGeometry::Polygon(rings)) => Some(format!(
            "{}\n({} ㎡)",
            feature.text("name"),
            format_number(polygon_area(rings), 1)
        )),
        (_, WkbGeometry::Polygon(rings)) => Some(format!("{} ㎡", format_number(polygon_area(rings), 2))),
        (DomainLayerKey::SectionLine, _) => named("section_id"),
        (DomainLayerKey::ControlPoints, _) => named("point_id"),
        (DomainLayerKey::ArtifactPoint, _) => named("artifact_no").or_else(|| named("kind")),
        (DomainLayerKey::FeatureLine, _) => named("feature_no").or_else(|| named("kind")),
        _ => None,
    }
}

/// 글자를 놓을 지도 좌표와, 그 점에서 글자를 어디에 둘지(가운데 / 오른쪽 위).
fn anchor(geom: &WkbGeometry) -> Option<([f64; 2], bool)> {
    match geom {
        WkbGeometry::Polygon(rings) => Some((ring_centroid(rings.first()?), true)),
        WkbGeometry::LineString(pts) => {
            // 선 길이의 가운데 점.
            let lens: Vec<f64> = pts
                .windows(2)
                .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
                .collect();
            let mut half = lens.iter().sum::<f64>() * 0.5;
            for (w, len) in pts.windows(2).zip(&lens) {
                if half <= *len && *len > 0.0 {
                    let t = half / len;
                    return Some(([w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t], true));
                }
                half -= len;
            }
            pts.first().map(|p| (*p, true))
        }
        WkbGeometry::Point(p) => Some((*p, false)),
    }
}

impl StrataAppState {
    /// 조사 레이어의 글자를 그린다. 도형을 다 그린 뒤에 부른다.
    pub(crate) fn paint_survey_doc_labels(&self, p: &egui::Painter, rect: Rect) {
        let vp = &self.home_map.viewport;
        let rs = &self.home_map.real_survey;
        let font = FontId::proportional(LABEL_PX);
        let mut placed: Vec<Rect> = Vec::new();
        // 위에 그려지는 것(점 → 선 → 면)의 글자를 먼저 놓아 자리를 준다.
        for key in [
            DomainLayerKey::ControlPoints,
            DomainLayerKey::ArtifactPoint,
            DomainLayerKey::SectionLine,
            DomainLayerKey::FeatureLine,
            DomainLayerKey::TrialTrench,
            DomainLayerKey::FeaturePoly,
            DomainLayerKey::SurveyArea,
        ] {
            let name = key.name_ko();
            if !rs.is_layer_visible(name) || !rs.is_label_visible(name) {
                continue;
            }
            for feature in &self.survey_doc.layer(key).features {
                let (Some(text), Some(geom)) = (label_text(key, feature), feature.geom.as_ref()) else {
                    continue;
                };
                let Some((world, centered)) = anchor(geom) else {
                    continue;
                };
                let at = w2s(rect, vp, world);
                if !rect.expand(40.0).contains(at) {
                    continue;
                }
                let galley = p.layout_no_wrap(text.clone(), font.clone(), INK);
                let size = galley.size();
                let top_left = if centered {
                    Pos2::new(at.x - size.x * 0.5, at.y - size.y * 0.5)
                } else {
                    Pos2::new(at.x + 8.0, at.y - size.y - 4.0)
                };
                let bounds = Rect::from_min_size(top_left, size).expand(2.0);
                if placed.iter().any(|r| r.intersects(bounds)) {
                    continue;
                }
                placed.push(bounds);
                // 흰 테두리: 같은 글자를 여덟 방향으로 흰색으로 먼저 찍는다.
                let halo = p.layout_no_wrap(text, font.clone(), Color32::from_rgba_unmultiplied(255, 255, 255, 230));
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
                    p.galley(
                        top_left + Vec2::new(dx * HALO_PX, dy * HALO_PX),
                        halo.clone(),
                        Color32::WHITE,
                    );
                }
                p.galley(top_left, galley, INK);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_storage::survey_doc::FieldValue;

    #[test]
    fn numbers_get_thousands_commas_like_qgis_format_number() {
        assert_eq!(format_number(12_000.0, 2), "12,000.00");
        assert_eq!(format_number(40.0, 1), "40.0");
        assert_eq!(format_number(1_234_567.891, 1), "1,234,567.9");
        assert_eq!(format_number(-1_500.0, 0), "-1,500");
    }

    #[test]
    fn each_layer_gets_the_label_cpp_gives_it_by_default() {
        let square = WkbGeometry::Polygon(vec![vec![[0.0, 0.0], [20.0, 0.0], [20.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]);
        let mut trench = SurveyFeature::new_drawn(square.clone());
        trench.set_text("name", "Tr-1");
        assert_eq!(label_text(DomainLayerKey::TrialTrench, &trench).as_deref(), Some("Tr-1\n(40.0 ㎡)"));
        // 유구 면·조사구역은 면적만(2자리).
        assert_eq!(label_text(DomainLayerKey::FeaturePoly, &trench).as_deref(), Some("40.00 ㎡"));
        assert_eq!(label_text(DomainLayerKey::SurveyArea, &trench).as_deref(), Some("40.00 ㎡"));

        let mut gcp = SurveyFeature::new_drawn(WkbGeometry::Point([1.0, 2.0]));
        assert_eq!(label_text(DomainLayerKey::ControlPoints, &gcp), None, "이름이 비면 글자도 없다");
        gcp.attrs.insert("point_id".to_owned(), FieldValue::Text("GCP1".to_owned()));
        assert_eq!(label_text(DomainLayerKey::ControlPoints, &gcp).as_deref(), Some("GCP1"));

        let mut section = SurveyFeature::new_drawn(WkbGeometry::LineString(vec![[0.0, 0.0], [10.0, 0.0]]));
        section.set_text("section_id", "A-A'");
        assert_eq!(label_text(DomainLayerKey::SectionLine, &section).as_deref(), Some("A-A'"));
    }

    #[test]
    fn a_line_label_sits_at_the_middle_of_its_length() {
        let line = WkbGeometry::LineString(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 30.0]]);
        // 전체 40 m 의 가운데(20 m)는 둘째 구간의 10 m 지점이다.
        assert_eq!(anchor(&line), Some(([10.0, 10.0], true)));
        assert_eq!(anchor(&WkbGeometry::Point([3.0, 4.0])), Some(([3.0, 4.0], false)));
    }
}
