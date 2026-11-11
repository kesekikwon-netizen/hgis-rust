//! 시굴격자 「구역에 깔기」 — 조사구역 위에 트렌치를 실제로 계산해 깐다.
//!
//! C++ `KaTrenchDialogPlan.cpp` → `TrenchGridGenerator::pickAutoFillArea` / `buildForTargetRatio`
//! / `buildInArea`. 예전 Rust 판은 레이어를 보이게만 하고 「배치했습니다」라고 했다.

use geo::{LineString, MultiPolygon, Polygon};

use strata_geo::trench_ratio_fill::{build_for_target_ratio, build_in_area, total_area, TrenchCell};
use strata_storage::survey_doc::{FieldValue, SurveyFeature};
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;

/// 트렌치 폭(m). C++ 기본값이자 상한.
const TRENCH_WIDTH_M: f64 = 2.0;

impl StrataAppState {
    /// 격자를 깔 조사구역: 고른 조사구역이 있으면 그것들, 없으면 마지막에 그린 것 하나.
    /// (C++ `pickAutoFillArea` — 남아 있는 다른 조사구역까지 합치지 않는다.)
    fn trench_fill_area(&self) -> Option<MultiPolygon<f64>> {
        let features = &self.survey_doc.layer(DomainLayerKey::SurveyArea).features;
        let mut picked: Vec<usize> = self
            .all_selected()
            .into_iter()
            .filter(|p| p.0 == DomainLayerKey::SurveyArea)
            .map(|p| p.1)
            .collect();
        if picked.is_empty() {
            picked.push(features.len().checked_sub(1)?);
        }
        let line = |r: &Vec<[f64; 2]>| LineString::from(r.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>());
        let polys: Vec<Polygon<f64>> = picked
            .into_iter()
            .filter_map(|i| match features.get(i)?.geom.as_ref()? {
                WkbGeometry::Polygon(rings) => {
                    let (outer, holes) = rings.split_first()?;
                    Some(Polygon::new(line(outer), holes.iter().map(line).collect()))
                }
                _ => None,
            })
            .collect();
        (!polys.is_empty()).then(|| MultiPolygon::new(polys))
    }

    /// 「구역에 깔기」. 깔았으면 `true`. 못 깔면 이유를 알리고 기존 격자는 그대로 둔다.
    pub(crate) fn place_trench_grid_in_area(&mut self) -> bool {
        self.sync_view_into_survey_doc();
        if self.modals.trench_shape.arm_bend().is_some() {
            self.home_map.draw_guide_message = Some(
                "ㄱ·ㄴ자 트렌치 자동 배치는 아직 Rust 판에 옮기지 못했습니다. 「곧은 트렌치」를 고르세요. 기존 격자는 유지됩니다."
                    .to_owned(),
            );
            return false;
        }
        let Some(area) = self.trench_fill_area() else {
            self.home_map.draw_guide_message =
                Some("조사구역이 없습니다. 조사구역을 먼저 그리거나 선택하세요.".to_owned());
            return false;
        };
        let length = self.modals.trench_length.meters();
        let azimuth = self.modals.trench_azimuth_deg;
        let kind = self.modals.trench_survey_kind_idx.min(2);
        // 0 시굴조사 10 %, 1 표본조사 2 %, 2 직접 지정(둑·접두를 사용자가 정한다).
        let planned: Result<(Vec<TrenchCell>, f64), String> = if kind == 2 {
            let prefix = self.modals.trench_prefix.clone();
            build_in_area(&area, TRENCH_WIDTH_M, length, self.modals.trench_balk_m, azimuth, &prefix)
                .map(|cells| (cells, self.modals.trench_balk_m))
        } else {
            let target = if kind == 1 { 2.0 } else { 10.0 };
            build_for_target_ratio(&area, target, TRENCH_WIDTH_M, azimuth, length)
                .map(|fill| (fill.cells, fill.balk))
        };
        let (cells, balk) = match planned {
            Ok(plan) => plan,
            Err(reason) => {
                self.home_map.draw_guide_message = Some(reason);
                return false;
            }
        };

        // C++ 「새로 만들기」는 앞의 격자 위에 쌓지 않고 바꿔 놓는다.
        self.push_undo();
        let trenches = &mut self.survey_doc.layer_mut(DomainLayerKey::TrialTrench).features;
        trenches.clear();
        for cell in &cells {
            let mut feature = SurveyFeature::new_drawn(WkbGeometry::Polygon(vec![cell.ring.clone()]));
            feature.set_text("name", &cell.name);
            feature.attrs.insert("width".to_owned(), FieldValue::Real(cell.width));
            feature.attrs.insert("length".to_owned(), FieldValue::Real(cell.length));
            trenches.push(feature);
        }
        self.after_doc_edit();
        self.modals.trench_balk_m = balk;
        self.home_map
            .real_survey
            .layer_visible
            .insert("시굴격자".to_owned(), true);
        let total = total_area(&cells);
        let ratio = total / geo::Area::unsigned_area(&area) * 100.0;
        self.home_map.draw_guide_message = Some(format!(
            "시굴격자 {}개를 조사구역 위에 깔았습니다 (총 {total:.0}㎡ · 비율 {ratio:.1}% · 둑 {balk:.0} m · 방위 {azimuth:.0}°). Ctrl+Z로 되돌릴 수 있습니다.",
            cells.len()
        ));
        true
    }
}

#[cfg(test)]
mod tests {
    use strata_canvas2d::map_tools::MapToolKind;
    use strata_types::schema::DomainLayerKey as K;

    use crate::app_state::StrataAppState;

    fn app_with_area(size: f64) -> StrataAppState {
        let mut app = StrataAppState::new_startup();
        app.draw_target = K::SurveyArea;
        let _ = app.commit_drawn_shape(
            MapToolKind::DrawPolygon,
            vec![[0.0, 0.0], [size, 0.0], [size, size], [0.0, size]],
        );
        app.modals.trench_survey_kind_idx = 0;
        app.modals.trench_azimuth_deg = 0.0;
        app
    }

    fn message(app: &StrataAppState) -> &str {
        app.home_map.draw_guide_message.as_deref().unwrap_or("")
    }

    #[test]
    fn laying_the_grid_fills_ten_percent_and_records_name_width_and_length() {
        let mut app = app_with_area(200.0);
        assert!(app.place_trench_grid_in_area(), "{}", message(&app));
        let trenches = &app.survey_doc.layer(K::TrialTrench).features;
        // 40,000 m² 의 10 % = 4,000 m² = 2×20 m 100개 (반 칸 이내).
        assert!((99..=101).contains(&trenches.len()), "n={}", trenches.len());
        assert_eq!(trenches[0].text("name"), "Tr-1");
        assert_eq!(trenches[0].text("length"), "20");
        assert_eq!(app.home_map.real_survey.trenches.len(), trenches.len(), "지도가 그릴 목록도 같다");
        assert!(message(&app).contains("비율 10.0%") || message(&app).contains("비율 9.9%") || message(&app).contains("비율 10.1%"), "{}", message(&app));
        assert!(app.is_dirty);
    }

    #[test]
    fn laying_again_replaces_the_old_grid_and_undo_brings_it_back() {
        let mut app = app_with_area(200.0);
        assert!(app.place_trench_grid_in_area());
        let first = app.survey_doc.layer(K::TrialTrench).features.len();
        app.modals.trench_survey_kind_idx = 1; // 표본조사 2 %
        assert!(app.place_trench_grid_in_area());
        let second = app.survey_doc.layer(K::TrialTrench).features.len();
        assert!(second < first / 3, "first={first} second={second}");
        assert!(app.survey_doc.layer(K::TrialTrench).features[0].text("name").starts_with("Sp-"));
        assert!(app.undo_edit());
        assert_eq!(app.survey_doc.layer(K::TrialTrench).features.len(), first);
    }

    #[test]
    fn it_says_why_and_keeps_the_old_grid_when_it_cannot_lay_one() {
        let mut empty = StrataAppState::new_startup();
        assert!(!empty.place_trench_grid_in_area());
        assert_eq!(message(&empty), "조사구역이 없습니다. 조사구역을 먼저 그리거나 선택하세요.");

        let mut app = app_with_area(200.0);
        assert!(app.place_trench_grid_in_area());
        let kept = app.survey_doc.layer(K::TrialTrench).features.len();
        // 너무 작은 조사구역을 새로 그리면(마지막 것만 쓴다) 들어갈 자리가 없다.
        let _ = app.commit_drawn_shape(
            MapToolKind::DrawPolygon,
            vec![[500.0, 0.0], [508.0, 0.0], [508.0, 8.0], [500.0, 8.0]],
        );
        assert!(!app.place_trench_grid_in_area());
        assert!(message(&app).contains("들어가지 않습니다"), "{}", message(&app));
        assert_eq!(app.survey_doc.layer(K::TrialTrench).features.len(), kept);
    }
}
