//! 도형선택 — C++ `FeaturePick::candidates`(core/FeaturePick.cpp)와 `KaFeatureSelectTool::selectAtPoint`
//! (KaFeatureSelectPick.cpp:129-211). 열어 둔 조사 표(저장 안 한 도형 포함)를 범례 차례(위가 먼저)로 본다.
//! 아직 없는 것: 안쪽 조각(구멍) 고르기, Shift 여러 개, 수정점(꼭짓점 편집), 지적·참조 레이어.

use crate::app_state::StrataAppState;

/// 고를 레이어 하나(점·허용치와 같은 좌표계의 도형 WKT).
pub(crate) struct PickLayer {
    pub(crate) key: String,
    /// 조사 레이어인가(지적·참조가 아니면) — 조사 도형이 있으면 지적·참조는 고르지 않는다.
    pub(crate) survey: bool,
    pub(crate) polygons: bool,
    pub(crate) features: Vec<(i64, String)>,
}

/// `kFeaturePickPx`·`kClickSlopPx`.
const PICK_PX: f64 = 10.0;
const CLICK_SLOP_PX: f32 = 4.0;

type Rank = (u8, u8, f64, usize);

/// 레이어마다 (레이어, 그 좌표계의 점, 허용치)로 고른다 — 차례: 조사 먼저, 점·선 → 점을 품은 면(작은 것 먼저) →
/// 가까운 면 테두리, 그 안에서 거리·넓이, 레이어 차례(Rank). 조사 도형이 하나라도 있으면 지적·참조는 뺀다.
fn ranked_hits<'a>(layers: impl Iterator<Item = (&'a PickLayer, [f64; 2], f64)>) -> Vec<(String, i64)> {
    let mut ranked: Vec<(Rank, (String, i64))> = Vec::new();
    for (order, (layer, xy, tol)) in layers.enumerate() {
        if tol.is_nan() || tol <= 0.0 {
            continue;
        }
        let tier = u8::from(!layer.survey);
        for (fid, wkt) in &layer.features {
            let Ok((contains, distance)) = strata_engine::wkt_point_relation(wkt, xy) else { continue };
            let rank = if layer.polygons && contains {
                (tier, 1, strata_engine::wkt_area(wkt).unwrap_or(0.0).abs() / (tol * tol), order)
            } else if (0.0..=tol).contains(&distance) {
                (tier, if layer.polygons { 2 } else { 0 }, distance / tol, order)
            } else {
                continue;
            };
            ranked.push((rank, (layer.key.clone(), *fid)));
        }
    }
    if ranked.iter().any(|(r, _)| r.0 == 0) {
        ranked.retain(|(r, _)| r.0 == 0);
    }
    ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().map(|(_, hit)| hit).collect()
}

/// candidates — 모든 레이어가 점과 같은 좌표계일 때(옮긴 C++ 시험용).
#[cfg(test)]
pub(crate) fn pick_candidates(layers: &[PickLayer], xy: [f64; 2], tolerance: f64) -> Vec<(String, i64)> {
    ranked_hits(layers.iter().map(|l| (l, xy, tolerance)))
}

impl StrataAppState {
    /// 보이는 조사 레이어를 범례 차례로, 점·허용치는 그 표의 좌표계로 옮겨(C++ toLayer.transform) 고른다.
    pub(crate) fn pick_at_work_xy(&self, xy: [f64; 2]) -> Vec<(String, i64)> {
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let tol = self.home_map.viewport.meters_per_pixel * PICK_PX;
        let mut layers: Vec<(PickLayer, [f64; 2], f64)> = Vec::new();
        for e in self.left_layers.entries.iter().filter(|e| self.home_map.real_survey.is_layer_visible(&e.label_ko)) {
            let Some(t) = self.survey_tables.iter().find(|t| t.table == e.layer_key) else { continue };
            let (p, t_tol) = match u32::try_from(t.srs_id).ok().filter(|s| *s > 0 && *s != work) {
                Some(src) => {
                    let Ok(tr) = strata_engine::Transformer::new(work, src) else { continue };
                    let (Ok(p), Ok(edge)) = (tr.transform(xy), tr.transform([xy[0] + tol, xy[1]])) else { continue };
                    (p, (edge[0] - p[0]).hypot(edge[1] - p[1]))
                }
                None => (xy, tol),
            };
            let features = t.layer.features.iter().filter(|f| !f.wkt_geom.trim().is_empty()).map(|f| (f.fid, f.wkt_geom.clone())).collect();
            let polygons = t.geometry_type.to_ascii_uppercase().contains("POLYGON");
            layers.push((PickLayer { key: t.table.clone(), survey: true, polygons, features }, p, t_tol));
        }
        ranked_hits(layers.iter().map(|(l, p, t)| (l, *p, *t)))
    }

    /// selectAtPoint — 같은 자리(4 px 안, 같은 후보)를 다시 누르면 다음 겹친 도형. 상태줄에 C++ 글.
    pub(crate) fn select_at(&mut self, xy: [f64; 2], screen: [f32; 2]) -> bool {
        let candidates = self.pick_at_work_xy(xy);
        let pick = match &self.home_map.pick_cycle {
            Some((last, idx, pos))
                if !candidates.is_empty()
                    && *last == candidates
                    && (screen[0] - pos[0]).abs() + (screen[1] - pos[1]).abs() <= CLICK_SLOP_PX =>
            {
                (idx + 1) % candidates.len()
            }
            _ => 0,
        };
        self.home_map.selected_shape = candidates.get(pick).cloned();
        self.home_map.pick_cycle = (!candidates.is_empty()).then(|| (candidates.clone(), pick, screen));
        // canvas selectionChanged → syncFeatureCard(MainWindowFeatureCard.cpp:32).
        self.sync_card_with_selection();
        if candidates.is_empty() {
            self.show_status_message("선택된 도형 없음");
        } else if candidates.len() > 1 {
            self.show_status_message(format!("겹친 도형 {}/{} — 같은 자리를 다시 누르면 다음 도형을 고릅니다.", pick + 1, candidates.len()));
        }
        // 한 개만 골랐을 때 C++ 은 「수정점이 나왔습니다…」라고 하지만 꼭짓점 편집이 아직 없어 말하지 않는다.
        !candidates.is_empty()
    }
}

#[cfg(test)]
#[path = "feature_pick_tests.rs"]
mod tests;
