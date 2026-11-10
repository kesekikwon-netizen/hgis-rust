//! 조사 데이터를 레이어 엔진의 창고로 옮긴다.
//!
//! C++ 은 GPKG 를 읽어 `QgsVectorLayer` 를 만들고 프로젝트에 넣는다. Rust 는
//! `RealSurveyData`(앱이 이미 들고 있는 데이터)를 [`LayerStage`] 로 옮긴다.
//! 이 다리가 있어야 "레이어를 끄면 진짜로 꺼지는" 동작이 된다.
//!
//! ## z순서 (C++ 레이어 트리와 같은 순서 — 아래에서 위로)
//!
//! ```text
//!  0  buffer_1000m    주변 1km (가장 아래)
//!  1  buffer_500m     주변 500m
//!  2  cadastral       지적도
//!  3  heritage        지정유산
//!  4  survey_area     조사구역
//!  5  trial_trench    시굴격자
//!  6  section_line    단면선 (가장 위)
//! ```

use strata_canvas2d::layer_engine::{
    DomainStyle, Feature, GeometryKind, LayerStage, MapLayer,
};

use crate::real_survey::RealSurveyData;

/// 레이어 키 — 앱에서 쓰는 이름과 같게 둔다(패널·범례 호환).
pub const KEY_BUFFER_1000: &str = "buffer_1000m";
pub const KEY_BUFFER_500: &str = "buffer_500m";
pub const KEY_CADASTRAL: &str = "cadastral_parcel";
pub const KEY_SURVEY_AREA: &str = "survey_area";
pub const KEY_TRIAL_TRENCH: &str = "trial_trench";

/// 조사 데이터 → 레이어 창고.
///
/// 피처가 없는 레이어는 **넣지 않는다**(C++ `addNonEmptyDomainLayers` 와 같은 규칙).
/// 그래야 빈 새 조사에서 빈 레이어가 화면을 채우지 않는다.
#[must_use]
pub fn stage_from_survey(rs: &RealSurveyData) -> LayerStage {
    let mut stage = LayerStage::new();

    // 주변 1km(있을 때만).
    if rs.show_buffer_1000m && rs.buffer_1000m.len() >= 3 {
        let mut l = MapLayer::new(KEY_BUFFER_1000, "주변 1km", GeometryKind::Polygon, 0)
            .as_reference();
        l.style = DomainStyle::for_key("survey_area");
        l.style.fill = [96, 128, 200, 18];
        l.style.stroke = [96, 128, 200, 140];
        l.style.stroke_width_mm = 0.8;
        l.features = vec![Feature::polygon(rs.buffer_1000m.clone())];
        l.visible = rs.is_layer_visible("주변 1km");
        stage.upsert(l);
    }

    // 주변 500m.
    if rs.buffer_500m.len() >= 3 {
        let mut l = MapLayer::new(KEY_BUFFER_500, "주변 500m", GeometryKind::Polygon, 1)
            .as_reference();
        l.style = DomainStyle::for_key("survey_area");
        l.style.fill = [176, 62, 26, 18];
        l.style.stroke = [176, 62, 26, 200];
        l.style.stroke_width_mm = 1.0;
        l.features = vec![Feature::polygon(rs.buffer_500m.clone())];
        l.visible = rs.is_layer_visible("주변 500m");
        stage.upsert(l);
    }

    // 지적도(조사 주변 5km에서 받은 필지).
    if !rs.cadastral_parcels.is_empty() {
        let mut l = MapLayer::new(KEY_CADASTRAL, "지적도", GeometryKind::Line, 2).as_reference();
        l.style = DomainStyle::for_key("section_line");
        l.style.stroke = [60, 58, 52, 135];
        l.style.stroke_width_mm = 0.55;
        l.style.casing = false;
        l.features = rs
            .cadastral_parcels
            .iter()
            .filter(|p| p.ring.len() >= 2)
            .map(|p| Feature::line(p.ring.clone()).with_label(p.jibun.clone()))
            .collect();
        l.visible = rs.is_layer_visible("지적도 · 조사 주변 5km");
        stage.upsert(l);
    }

    // 지정유산(초록/자홍 구분은 스타일이 아니라 피처 순서로 처리 —
    // 엔진은 레이어 하나에 스타일 하나이므로, 초록과 자홍을 레이어 둘로 나눈다).
    if !rs.heritage_polys.is_empty() {
        let green: Vec<Feature> = rs
            .heritage_polys
            .iter()
            .filter(|h| h.is_green && h.ring.len() >= 3)
            .map(|h| Feature::polygon(h.ring.clone()).with_label(h.site_name.clone()))
            .collect();
        let magenta: Vec<Feature> = rs
            .heritage_polys
            .iter()
            .filter(|h| !h.is_green && h.ring.len() >= 3)
            .map(|h| Feature::polygon(h.ring.clone()).with_label(h.site_name.clone()))
            .collect();
        if !green.is_empty() {
            let mut l = MapLayer::new("heritage_green", "국가지정유산", GeometryKind::Polygon, 3)
                .as_reference();
            l.style = DomainStyle::for_key("feature_poly");
            l.style.fill = [34, 148, 52, 26];
            l.style.stroke = [34, 148, 52, 255];
            l.style.stroke_width_mm = 0.35;
            l.features = green;
            l.visible = rs.is_layer_visible("지정유산");
            stage.upsert(l);
        }
        if !magenta.is_empty() {
            let mut l = MapLayer::new("heritage_magenta", "보호구역", GeometryKind::Polygon, 4)
                .as_reference();
            l.style = DomainStyle::for_key("feature_poly");
            l.style.fill = [176, 28, 128, 26];
            l.style.stroke = [176, 28, 128, 255];
            l.style.stroke_width_mm = 0.32;
            l.features = magenta;
            l.visible = rs.is_layer_visible("지정유산");
            stage.upsert(l);
        }
    }

    // 조사구역 — 클릭으로 그린 링들. 면적 라벨을 붙인다.
    if !rs.survey_rings.is_empty() {
        let mut l = MapLayer::new(KEY_SURVEY_AREA, "조사구역", GeometryKind::Polygon, 5);
        l.style = DomainStyle::for_key("survey_area");
        l.features = rs
            .survey_rings
            .iter()
            .filter(|r| r.len() >= 3)
            .map(|r| {
                let m2 = crate::real_survey::ring_area_m2(r);
                Feature::polygon(r.clone()).with_label(format_area(m2))
            })
            .collect();
        l.visible = rs.is_layer_visible("조사구역");
        stage.upsert(l);
    }

    // 시굴 트렌치.
    if !rs.trenches.is_empty() {
        let mut l = MapLayer::new(KEY_TRIAL_TRENCH, "시굴격자", GeometryKind::Polygon, 6);
        l.style = DomainStyle::for_key("trial_trench");
        l.features = rs
            .trenches
            .iter()
            .filter(|t| t.ring.len() >= 3)
            .map(|t| Feature::polygon(t.ring.clone()).with_label(t.name.clone()))
            .collect();
        l.visible = rs.is_layer_visible("시굴격자");
        stage.upsert(l);
    }

    stage
}

/// 면적을 C++ `applyAreaM2Labels` 와 같은 모양으로 쓴다: `"1,234㎡"`.
///
/// 1,000 미만은 소수점 없이, 그 이상은 천 단위 쉼표.
#[must_use]
pub fn format_area(m2: f64) -> String {
    if !m2.is_finite() || m2 < 0.0 {
        return String::new();
    }
    let rounded = m2.round();
    if rounded < 1.0 {
        return String::new();
    }
    let n = rounded as i64;
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + 3);
    let bytes = s.as_bytes();
    for (i, ch) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*ch as char);
    }
    out.push('㎡');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 면적 서식 — C++ `applyAreaM2Labels` 와 같은 모양.
    #[test]
    fn area_format_matches_cpp() {
        assert_eq!(format_area(0.0), "");
        assert_eq!(format_area(0.4), "");
        assert_eq!(format_area(1.0), "1㎡");
        assert_eq!(format_area(999.0), "999㎡");
        assert_eq!(format_area(1000.0), "1,000㎡");
        assert_eq!(format_area(1234.0), "1,234㎡");
        assert_eq!(format_area(86_865.77), "86,866㎡");
        assert_eq!(format_area(f64::NAN), "");
        assert_eq!(format_area(-5.0), "");
    }

    fn empty_survey() -> RealSurveyData {
        RealSurveyData::new_empty_survey(&std::env::temp_dir())
    }

    /// 빈 조사는 레이어가 하나도 없다 — 예전에 이게 안 돼서 화면이 채워졌다.
    #[test]
    fn empty_survey_makes_no_layers() {
        let st = stage_from_survey(&empty_survey());
        assert_eq!(st.len(), 0, "빈 조사에 레이어가 생기면 안 된다");
    }

    /// 조사구역을 넣으면 레이어 하나 + 면적 라벨.
    #[test]
    fn survey_ring_becomes_one_layer_with_area_label() {
        let mut rs = empty_survey();
        rs.survey_rings = vec![vec![
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 50.0],
            [0.0, 50.0],
        ]];
        let st = stage_from_survey(&rs);
        assert_eq!(st.len(), 1);
        let l = st.get(KEY_SURVEY_AREA).expect("조사구역 레이어");
        assert_eq!(l.features.len(), 1);
        assert_eq!(l.features[0].label, "5,000㎡", "100x50 = 5000");
        // C++ 스타일 표 값
        assert_eq!(l.style.fill, [180, 83, 9, 70]);
        assert_eq!(l.style.stroke, [146, 64, 14, 255]);
    }

    /// 레이어를 끄면 창고에서 안 그려진다(엔진과 연결 확인).
    #[test]
    fn layer_visibility_carries_from_survey() {
        let mut rs = empty_survey();
        rs.survey_rings = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]];
        rs.layer_visible.insert("조사구역".to_owned(), false);
        let st = stage_from_survey(&rs);
        let l = st.get(KEY_SURVEY_AREA).expect("레이어는 있음");
        assert!(!l.visible, "끈 레이어는 visible=false 여야");
        assert!(!l.has_content(), "그릴 내용이 없어야");
    }

    /// 시굴 트렌치는 조사구역보다 **위**(z 큼)에 그려진다.
    #[test]
    fn trench_draws_above_survey_area() {
        let mut rs = empty_survey();
        rs.survey_rings = vec![vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]];
        rs.trenches.push(crate::real_survey::RealTrenchPoly {
            name: "T1".to_owned(),
            ring: vec![[10.0, 10.0], [20.0, 10.0], [20.0, 20.0]],
            area_m2: 50.0,
            center: [15.0, 15.0],
        });
        let st = stage_from_survey(&rs);
        let area = st.get(KEY_SURVEY_AREA).expect("조사구역");
        let tr = st.get(KEY_TRIAL_TRENCH).expect("시굴");
        assert!(tr.z_order > area.z_order, "시굴이 위에 그려져야 한다");
    }
}
