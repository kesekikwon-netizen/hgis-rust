//! 조사 문서의 도형 고르기·속성 카드 연결·삭제.
//!
//! C++ 은 `KaFeatureSelectTool` + `FeaturePick`(QgsFeatureRequest 공간 필터)로 도형을 찾고,
//! `KaFeatureCard` 가 그 도형의 속성을 `changeAttributeValue` 로 고친다. 여기서는
//! `SurveyDoc` 에서 직접 찾고, 오른쪽 카드의 칸을 고른 도형의 속성과 주고받는다.

use strata_storage::survey_doc::{FieldValue, SurveyFeature};
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::real_survey::{ring_area_m2, ring_perimeter_m};

/// 클릭 허용 거리(화면 픽셀). C++ `FeaturePick` 의 선택 반경과 같은 10 px.
const PICK_TOLERANCE_PX: f64 = 10.0;

fn dist_point_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    };
    let (qx, qy) = (a[0] + t * dx, a[1] + t * dy);
    ((p[0] - qx).powi(2) + (p[1] - qy).powi(2)).sqrt()
}

fn dist_point_polyline(p: [f64; 2], pts: &[[f64; 2]]) -> f64 {
    pts.windows(2)
        .map(|w| dist_point_segment(p, w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

/// 고리를 감싸는 사각형 `(왼쪽 아래, 오른쪽 위)`.
pub(crate) fn ring_bbox(ring: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in ring {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    (lo, hi)
}

/// 점이 고리 안에 있는지(짝홀 규칙).
fn point_in_ring(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let n = ring.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 구멍을 뺀 다각형 안에 있는지.
fn point_in_polygon(p: [f64; 2], rings: &[Vec<[f64; 2]>]) -> bool {
    match rings.split_first() {
        Some((outer, holes)) => {
            point_in_ring(p, outer) && !holes.iter().any(|h| point_in_ring(p, h))
        }
        None => false,
    }
}

/// 카드의 칸과 짝이 되는 속성 열 이름(레이어마다 다르다). (번호, 종류, 시대, 메모)
const fn card_columns(key: DomainLayerKey) -> [Option<&'static str>; 4] {
    match key {
        DomainLayerKey::FeaturePoly | DomainLayerKey::FeatureLine => {
            [Some("feature_no"), Some("kind"), Some("period"), Some("note")]
        }
        DomainLayerKey::ArtifactPoint => {
            [Some("artifact_no"), Some("kind"), Some("period"), Some("note")]
        }
        DomainLayerKey::SectionLine => [Some("section_id"), None, None, Some("note")],
        DomainLayerKey::ControlPoints => [Some("point_id"), None, None, None],
        DomainLayerKey::TrialTrench => [Some("name"), None, None, None],
        DomainLayerKey::SurveyArea => [None, None, None, Some("note")],
    }
}

impl StrataAppState {
    fn selected_feature(&self) -> Option<&SurveyFeature> {
        let (key, idx) = self.selected_doc?;
        self.survey_doc.layer(key).features.get(idx)
    }

    /// 지도 좌표 `wxy` 에서 고를 도형을 찾는다. 위에 그려지는 것부터: 점 → 선 → 면(작은 것 먼저).
    #[must_use]
    pub(crate) fn pick_doc_feature(&self, wxy: [f64; 2]) -> Option<(DomainLayerKey, usize)> {
        let tol = PICK_TOLERANCE_PX * self.home_map.viewport.meters_per_pixel;
        let rs = &self.home_map.real_survey;
        let visible = |key: DomainLayerKey| rs.is_layer_visible(key.name_ko());
        let features = |key: DomainLayerKey| self.survey_doc.layer(key).features.iter().enumerate();

        for key in [DomainLayerKey::ControlPoints, DomainLayerKey::ArtifactPoint] {
            if !visible(key) {
                continue;
            }
            let hit = features(key)
                .filter_map(|(i, f)| match f.geom.as_ref()? {
                    WkbGeometry::Point(p) => {
                        let d = ((p[0] - wxy[0]).powi(2) + (p[1] - wxy[1]).powi(2)).sqrt();
                        (d <= tol).then_some((i, d))
                    }
                    _ => None,
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = hit {
                return Some((key, i));
            }
        }
        for key in [DomainLayerKey::SectionLine, DomainLayerKey::FeatureLine] {
            if !visible(key) {
                continue;
            }
            let hit = features(key)
                .filter_map(|(i, f)| match f.geom.as_ref()? {
                    WkbGeometry::LineString(pts) => {
                        let d = dist_point_polyline(wxy, pts);
                        (d <= tol).then_some((i, d))
                    }
                    _ => None,
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = hit {
                return Some((key, i));
            }
        }
        if visible(DomainLayerKey::FeaturePoly) {
            let hit = features(DomainLayerKey::FeaturePoly)
                .filter_map(|(i, f)| match f.geom.as_ref()? {
                    WkbGeometry::Polygon(rings) if point_in_polygon(wxy, rings) => {
                        Some((i, rings.first().map_or(0.0, |r| ring_area_m2(r))))
                    }
                    _ => None,
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = hit {
                return Some((DomainLayerKey::FeaturePoly, i));
            }
        }
        None
    }

    /// 도형을 고르고 그 속성을 오른쪽 카드에 올린다.
    pub(crate) fn select_doc_feature(&mut self, key: DomainLayerKey, idx: usize) {
        self.sync_card_into_selected_feature();
        let Some(feature) = self.survey_doc.layer(key).features.get(idx) else {
            return;
        };
        let cols = card_columns(key);
        let read = |c: Option<&str>| c.map(|name| feature.text(name)).unwrap_or_default();
        let (no, kind, period, memo) = (read(cols[0]), read(cols[1]), read(cols[2]), read(cols[3]));
        let ri = &mut self.right_inspector;
        ri.feature_no = no.clone();
        ri.feature_type = kind;
        ri.feature_period = period;
        ri.feature_memo = memo;
        ri.selected_layer_key = Some(key.name_ko().to_owned());
        self.selected_doc = Some((key, idx));
        self.home_map.selected_feature_idx = None;
        self.home_map.draw_guide_message = Some(if no.is_empty() {
            format!("도형 선택: {} — 오른쪽 카드에서 기록을 고치고, Delete 로 지웁니다.", key.name_ko())
        } else {
            format!("도형 선택: {} {no} — 오른쪽 카드에서 기록을 고치고, Delete 로 지웁니다.", key.name_ko())
        });
    }

    /// 고르기를 푼다(카드에서 고친 값은 먼저 도형에 적는다).
    pub(crate) fn clear_doc_selection(&mut self) {
        self.sync_card_into_selected_feature();
        self.selected_doc = None;
    }

    /// 카드에서 고친 값을 고른 도형의 속성에 적는다. 달라진 것이 있으면 「변경됨」으로 표시한다.
    pub(crate) fn sync_card_into_selected_feature(&mut self) {
        let Some((key, idx)) = self.selected_doc else {
            return;
        };
        let ri = &self.right_inspector;
        let values = [
            ri.feature_no.clone(),
            ri.feature_type.clone(),
            ri.feature_period.clone(),
            ri.feature_memo.clone(),
        ];
        let Some(feature) = self.survey_doc.layer_mut(key).features.get_mut(idx) else {
            self.selected_doc = None;
            return;
        };
        let mut changed = false;
        for (col, value) in card_columns(key).into_iter().zip(values) {
            let Some(col) = col else { continue };
            if feature.text(col) != value {
                feature
                    .attrs
                    .insert(col.to_owned(), FieldValue::Text(value));
                changed = true;
            }
        }
        if changed {
            self.is_dirty = true;
        }
    }

    /// 고른 도형을 지운다(C++ `MainWindowUndo.cpp:486` 선택 도형 삭제). 지웠으면 `true`.
    pub(crate) fn delete_selected_feature(&mut self) -> bool {
        let Some((key, idx)) = self.selected_doc.take() else {
            return false;
        };
        let layer = self.survey_doc.layer_mut(key);
        if idx >= layer.features.len() {
            return false;
        }
        layer.features.remove(idx);
        self.is_dirty = true;
        self.home_map.draw_guide_message = Some(format!(
            "{} 도형 하나를 지웠습니다. 저장(Ctrl+S)해야 파일에서도 지워집니다.",
            key.name_ko()
        ));
        true
    }

    /// 카드 위쪽 두 줄(크기와 좌표계). 고른 것이 없으면 `None`.
    #[must_use]
    pub(crate) fn selection_summary_lines(&self) -> Option<(String, String)> {
        let crs = self.work_crs.auth_id();
        if let Some(feature) = self.selected_feature() {
            return Some(match feature.geom.as_ref() {
                Some(WkbGeometry::Polygon(rings)) => {
                    let outer = rings.first().map_or(&[][..], Vec::as_slice);
                    let area = ring_area_m2(outer);
                    (
                        format!("면적: {area:.2} m² ({:.1} 평)", area / 3.305_785),
                        format!("둘레: {:.2} m · {crs}", ring_perimeter_m(outer)),
                    )
                }
                Some(WkbGeometry::LineString(pts)) => {
                    let len: f64 = pts
                        .windows(2)
                        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
                        .sum();
                    (format!("길이: {len:.2} m"), format!("꼭짓점 {}개 · {crs}", pts.len()))
                }
                Some(WkbGeometry::Point(p)) => {
                    (format!("X {:.3} · Y {:.3}", p[0], p[1]), crs.to_owned())
                }
                None => ("도형 없음".to_owned(), crs.to_owned()),
            });
        }
        let ring = self
            .home_map
            .selected_feature_idx
            .and_then(|idx| self.home_map.real_survey.survey_rings.get(idx))?;
        let area = ring_area_m2(ring);
        Some((
            format!("면적: {area:.2} m² ({:.1} 평)", area / 3.305_785),
            format!("둘레: {:.2} m · {crs}", ring_perimeter_m(ring)),
        ))
    }

    /// 고른 유구의 상면·바닥 표고(속성 `top_el`·`bottom_el`). 둘 다 적혀 있을 때만 돌려준다.
    #[must_use]
    pub(crate) fn selected_elevations(&self) -> Option<(f64, f64)> {
        let feature = self.selected_feature()?;
        let num = |name: &str| match feature.attrs.get(name) {
            Some(FieldValue::Real(v)) => Some(*v),
            Some(FieldValue::Int(v)) => Some(*v as f64),
            _ => None,
        };
        Some((num("top_el")?, num("bottom_el")?))
    }
}

#[cfg(test)]
#[path = "survey_selection_tests.rs"]
mod tests;
