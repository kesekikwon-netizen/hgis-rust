//! 주변유적경계 — 조사구역에서 500 m·1000 m 떨어진 둥근 경계선을 실제로 계산한다.
//!
//! C++ `BufferAnalysis.cpp:218` — `poly.buffer(meters, 36, Round, Round, 2.0)`.
//! QGIS 의 36 은 「사분원당 36 조각」이므로 원 한 바퀴는 144 조각이다.
//! 예전 Rust 판은 견본 파일에 있던 선을 보이거나 타원을 그렸다.

use geo::{Area, BooleanOps, LineString, MultiPolygon, Polygon};

use strata_geo::buffer_round::round_buffer_polygon;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;

/// 원 한 바퀴의 조각 수(QGIS `buffer(…, 36, …)` 와 같은 매끄러움).
const CIRCLE_SEGMENTS: usize = 144;

impl StrataAppState {
    /// 모든 조사구역을 `radius_m` 만큼 부풀린 바깥 경계선. 조사구역이 없으면 `None`.
    #[must_use]
    pub(crate) fn survey_buffer_ring(&self, radius_m: f64) -> Option<Vec<[f64; 2]>> {
        let mut merged = MultiPolygon::new(Vec::new());
        for feature in &self.survey_doc.layer(DomainLayerKey::SurveyArea).features {
            let Some(WkbGeometry::Polygon(rings)) = feature.geom.as_ref() else {
                continue;
            };
            let Some(outer) = rings.first() else {
                continue;
            };
            let poly = Polygon::new(
                LineString::from(outer.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>()),
                Vec::new(),
            );
            let buffered = round_buffer_polygon(&poly, radius_m, CIRCLE_SEGMENTS);
            merged = merged.union(&MultiPolygon::new(vec![buffered]));
        }
        merged
            .into_iter()
            .max_by(|a, b| a.unsigned_area().total_cmp(&b.unsigned_area()))
            .map(|p| p.exterior().coords().map(|c| [c.x, c.y]).collect())
    }

    /// 지금 켜져 있는 경계선을 조사구역에서 다시 계산한다(조사구역이 바뀔 때마다 부른다).
    pub(crate) fn refresh_survey_buffers(&mut self) {
        let show_500 = !self.home_map.real_survey.buffer_500m.is_empty();
        let show_1000 = self.home_map.real_survey.show_buffer_1000m;
        let ring_500 = if show_500 { self.survey_buffer_ring(500.0) } else { None };
        let ring_1000 = if show_1000 { self.survey_buffer_ring(1000.0) } else { None };
        let rs = &mut self.home_map.real_survey;
        rs.buffer_500m = ring_500.unwrap_or_default();
        rs.buffer_1000m = ring_1000.unwrap_or_default();
    }

    /// 「버퍼 500m / 1000m」 단추: 경계선을 켜고 끈다. 켤 때 조사구역에서 계산한다.
    pub(crate) fn toggle_survey_buffer(&mut self, radius_m: f64) {
        self.sync_view_into_survey_doc();
        let is_500 = radius_m < 750.0;
        let rs = &self.home_map.real_survey;
        let currently_on = if is_500 {
            !rs.buffer_500m.is_empty() && rs.is_layer_visible("주변 500m")
        } else {
            rs.show_buffer_1000m && !rs.buffer_1000m.is_empty()
        };
        if currently_on {
            let rs = &mut self.home_map.real_survey;
            if is_500 {
                rs.buffer_500m.clear();
            } else {
                rs.show_buffer_1000m = false;
                rs.buffer_1000m.clear();
            }
            self.home_map.draw_guide_message =
                Some(format!("조사구역 주변 {radius_m:.0} m 경계를 껐습니다."));
            return;
        }
        let Some(ring) = self.survey_buffer_ring(radius_m) else {
            self.home_map.draw_guide_message = Some(
                "조사구역이 없어 주변 경계를 만들 수 없습니다. 먼저 조사구역을 그리세요.".to_owned(),
            );
            return;
        };
        let rs = &mut self.home_map.real_survey;
        if is_500 {
            rs.buffer_500m = ring;
            rs.layer_visible.insert("주변 500m".to_owned(), true);
        } else {
            rs.buffer_1000m = ring;
            rs.show_buffer_1000m = true;
        }
        self.home_map.draw_guide_message = Some(format!(
            "조사구역에서 {radius_m:.0} m 떨어진 주변 경계를 지도에 그렸습니다."
        ));
    }
}

#[cfg(test)]
mod tests {
    use strata_canvas2d::map_tools::MapToolKind;
    use strata_types::schema::DomainLayerKey as K;

    use crate::app_state::StrataAppState;
    use crate::real_survey::ring_area_m2;

    fn app_with_square_area() -> StrataAppState {
        let mut app = StrataAppState::new_startup();
        app.draw_target = K::SurveyArea;
        let _ = app.commit_drawn_shape(
            MapToolKind::DrawPolygon,
            vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
        );
        app
    }

    #[test]
    fn the_500m_boundary_is_everywhere_500m_from_the_survey_area() {
        let mut app = app_with_square_area();
        app.toggle_survey_buffer(500.0);
        let ring = app.home_map.real_survey.buffer_500m.clone();
        assert!(ring.len() > 100, "둥근 모서리가 있어야 한다");
        // 100×100 정사각형을 500 m 부풀린 넓이: 100² + 4·100·500 + π·500²
        let exact = 10_000.0 + 200_000.0 + std::f64::consts::PI * 250_000.0;
        let area = ring_area_m2(&ring);
        assert!((area - exact).abs() / exact < 0.001, "area={area} exact={exact}");
        // 모든 꼭짓점이 정사각형에서 500 m (±0.5 m) 떨어져 있다.
        for p in &ring {
            let dx = (p[0] - p[0].clamp(0.0, 100.0)).abs();
            let dy = (p[1] - p[1].clamp(0.0, 100.0)).abs();
            let d = (dx * dx + dy * dy).sqrt();
            assert!((d - 500.0).abs() < 0.5, "거리 {d}");
        }
    }

    #[test]
    fn the_buttons_toggle_and_say_so_when_there_is_no_survey_area() {
        let mut empty = StrataAppState::new_startup();
        empty.toggle_survey_buffer(500.0);
        assert!(empty.home_map.real_survey.buffer_500m.is_empty());
        assert!(empty
            .home_map
            .draw_guide_message
            .as_deref()
            .unwrap_or("")
            .starts_with("조사구역이 없어"));

        let mut app = app_with_square_area();
        app.toggle_survey_buffer(1000.0);
        assert!(app.home_map.real_survey.show_buffer_1000m);
        let a1000 = ring_area_m2(&app.home_map.real_survey.buffer_1000m);
        assert!(a1000 > 3_500_000.0 && a1000 < 3_560_000.0, "a1000={a1000}");
        app.toggle_survey_buffer(1000.0);
        assert!(!app.home_map.real_survey.show_buffer_1000m);
    }

    #[test]
    fn the_boundary_follows_the_survey_area_when_it_changes() {
        let mut app = app_with_square_area();
        app.toggle_survey_buffer(500.0);
        let before = ring_area_m2(&app.home_map.real_survey.buffer_500m);
        // 조사구역을 하나 더 그리면 경계가 넓어진다.
        let _ = app.commit_drawn_shape(
            MapToolKind::DrawPolygon,
            vec![[300.0, 0.0], [400.0, 0.0], [400.0, 100.0], [300.0, 100.0]],
        );
        let after = ring_area_m2(&app.home_map.real_survey.buffer_500m);
        assert!(after > before + 100_000.0, "before={before} after={after}");
        // 되돌리면 경계도 돌아온다.
        assert!(app.undo_edit());
        let undone = ring_area_m2(&app.home_map.real_survey.buffer_500m);
        assert!((undone - before).abs() < 1.0);
    }
}
