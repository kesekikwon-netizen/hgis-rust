//! 도형 편집: 겹친 곳 지우기·폴리곤 묶기와 되돌리기/다시하기.
//!
//! C++ 은 `PolygonErase`(QgsGeometry::difference)·`LayerOps::mergePolygonFeatures` 가 편집
//! 버퍼에 명령을 남기고 Ctrl+Z 로 되돌린다. 여기서는 `geo` 의 불리언 연산으로 `SurveyDoc`
//! 을 고치고, 고치기 전 문서를 통째로 쌓아 두어 되돌린다.
//! 무엇이 「자르는 면」이고 무엇이 「대상」인지의 규칙은 `PolygonErase.h` 의 설명 그대로다.

use geo::{Area, BooleanOps, Intersects, LineString, MultiPolygon, Polygon};

use strata_storage::survey_doc::SurveyDoc;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;

type Pick = (DomainLayerKey, usize);

/// 되돌리기 기록 상한.
const UNDO_LIMIT: usize = 50;
/// 이보다 작은 조각(㎡)은 계산 찌꺼기로 보고 버린다.
const SLIVER_M2: f64 = 1e-6;

fn to_geo(rings: &[Vec<[f64; 2]>]) -> Option<Polygon<f64>> {
    let line = |r: &Vec<[f64; 2]>| LineString::from(r.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>());
    let (outer, holes) = rings.split_first()?;
    Some(Polygon::new(line(outer), holes.iter().map(line).collect()))
}

fn from_geo(poly: &Polygon<f64>) -> WkbGeometry {
    let ring = |l: &LineString<f64>| l.coords().map(|c| [c.x, c.y]).collect::<Vec<_>>();
    let mut rings = vec![ring(poly.exterior())];
    rings.extend(poly.interiors().iter().map(ring));
    WkbGeometry::Polygon(rings)
}

/// 넓은 것부터, 찌꺼기는 빼고.
fn pieces(multi: MultiPolygon<f64>) -> Vec<Polygon<f64>> {
    let mut out: Vec<Polygon<f64>> = multi
        .into_iter()
        .filter(|p| p.unsigned_area() > SLIVER_M2)
        .collect();
    out.sort_by(|a, b| b.unsigned_area().total_cmp(&a.unsigned_area()));
    out
}

fn polygon_of(doc: &SurveyDoc, pick: Pick) -> Option<Polygon<f64>> {
    match doc.layer(pick.0).features.get(pick.1)?.geom.as_ref()? {
        WkbGeometry::Polygon(rings) => to_geo(rings),
        _ => None,
    }
}

/// 면 레이어(고칠 수 있는 조사 데이터).
const POLYGON_LAYERS: [DomainLayerKey; 3] = [
    DomainLayerKey::SurveyArea,
    DomainLayerKey::FeaturePoly,
    DomainLayerKey::TrialTrench,
];

/// 무엇으로 무엇을 지울지. `PolygonErase::plan` 과 같은 규칙.
fn erase_plan(doc: &SurveyDoc, selected: &[Pick]) -> Result<(Vec<Pick>, Pick), String> {
    let polys: Vec<(Pick, Polygon<f64>)> = selected
        .iter()
        .filter_map(|&p| polygon_of(doc, p).map(|g| (p, g)))
        .collect();
    if polys.is_empty() {
        return Err(if selected.is_empty() {
            "위에 그린 도형을 [도형선택]으로 고른 뒤 누르세요.".to_owned()
        } else {
            "고른 도형으로는 지울 수 없습니다. 조사 데이터의 면 도형을 고르세요.".to_owned()
        });
    }
    if let [(cutter, cutter_geom)] = polys.as_slice() {
        // 하나를 골랐다: 그 면이 자르는 면. 같은 레이어에서 겹친 면을 먼저, 없으면 다른 레이어에서 찾는다.
        let overlaps_in = |key: DomainLayerKey| -> Vec<Pick> {
            (0..doc.layer(key).features.len())
                .map(|i| (key, i))
                .filter(|p| p != cutter)
                .filter(|&p| polygon_of(doc, p).is_some_and(|g| g.intersects(cutter_geom)))
                .collect()
        };
        let mut found = overlaps_in(cutter.0);
        if found.is_empty() {
            found = POLYGON_LAYERS
                .into_iter()
                .filter(|k| *k != cutter.0)
                .flat_map(overlaps_in)
                .collect();
        }
        return match found.as_slice() {
            [] => Err("고른 도형 아래에 겹친 면이 없습니다.".to_owned()),
            [target] => {
                let rest = polygon_of(doc, *target).map(|t| pieces(t.difference(cutter_geom)));
                if rest.is_some_and(|r| r.is_empty()) {
                    Err("고른 도형이 아래 도형을 모두 덮고 있어 남는 부분이 없습니다.".to_owned())
                } else {
                    Ok((vec![*cutter], *target))
                }
            }
            many => Err(format!(
                "아래에 겹친 도형이 {}개입니다. Shift를 누른 채 지울 도형도 함께 고른 뒤 누르세요.",
                many.len()
            )),
        };
    }
    // 둘 이상 골랐다: 가장 큰 면이 대상, 나머지가 자르는 면.
    let (target, target_geom) = polys
        .iter()
        .max_by(|a, b| a.1.unsigned_area().total_cmp(&b.1.unsigned_area()))
        .cloned()
        .ok_or_else(|| "지울 도형을 정하지 못했습니다.".to_owned())?;
    let cutters: Vec<(Pick, Polygon<f64>)> = polys.into_iter().filter(|(p, _)| *p != target).collect();
    if !cutters.iter().any(|(_, g)| g.intersects(&target_geom)) {
        return Err("고른 도형들이 서로 겹치지 않습니다.".to_owned());
    }
    let mut rest = MultiPolygon::new(vec![target_geom]);
    for (_, g) in &cutters {
        rest = rest.difference(&MultiPolygon::new(vec![g.clone()]));
    }
    if pieces(rest).is_empty() {
        return Err("고른 작은 도형들이 큰 도형을 모두 덮어 남는 부분이 없습니다.".to_owned());
    }
    Ok((cutters.into_iter().map(|(p, _)| p).collect(), target))
}

/// 여러 도형을 지운다. 순번이 밀리지 않게 레이어마다 뒤에서부터 지운다.
fn remove_features(doc: &mut SurveyDoc, mut picks: Vec<Pick>) {
    picks.sort_by(|a, b| b.1.cmp(&a.1));
    for (key, idx) in picks {
        let features = &mut doc.layer_mut(key).features;
        if idx < features.len() {
            features.remove(idx);
        }
    }
}

impl StrataAppState {
    /// 고른 도형 전부(처음 고른 것이 맨 앞).
    #[must_use]
    pub(crate) fn all_selected(&self) -> Vec<Pick> {
        let mut out: Vec<Pick> = self.selected_doc.into_iter().collect();
        for p in &self.extra_selected {
            if !out.contains(p) {
                out.push(*p);
            }
        }
        out
    }

    /// 편집 직전에 부른다: 지금 문서를 되돌리기 기록에 쌓는다.
    pub(crate) fn push_undo(&mut self) {
        self.sync_view_into_survey_doc();
        self.undo_stack.push(self.survey_doc.clone());
        if self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn after_doc_edit(&mut self) {
        self.selected_doc = None;
        self.extra_selected.clear();
        self.home_map.selected_feature_idx = None;
        self.refresh_view_from_doc();
        for key in self.survey_doc.non_empty_keys() {
            self.left_layers.ensure_domain_layer(key);
        }
        self.is_dirty = true;
    }

    /// Ctrl+Z — 마지막 편집을 되돌린다. 되돌릴 것이 없으면 `false`.
    pub(crate) fn undo_edit(&mut self) -> bool {
        self.sync_view_into_survey_doc();
        let Some(prev) = self.undo_stack.pop() else {
            self.home_map.draw_guide_message = Some("되돌릴 편집이 없습니다.".to_owned());
            return false;
        };
        self.redo_stack.push(std::mem::replace(&mut self.survey_doc, prev));
        self.after_doc_edit();
        self.home_map.draw_guide_message = Some("마지막 편집을 되돌렸습니다 (Ctrl+Z).".to_owned());
        true
    }

    /// Ctrl+Y — 되돌린 편집을 다시 한다.
    pub(crate) fn redo_edit(&mut self) -> bool {
        self.sync_view_into_survey_doc();
        let Some(next) = self.redo_stack.pop() else {
            self.home_map.draw_guide_message = Some("다시 할 편집이 없습니다.".to_owned());
            return false;
        };
        self.undo_stack.push(std::mem::replace(&mut self.survey_doc, next));
        self.after_doc_edit();
        self.home_map.draw_guide_message = Some("되돌린 편집을 다시 적용했습니다 (Ctrl+Y).".to_owned());
        true
    }

    /// 「겹친 곳 지우기」 — 위에 그린 면 모양대로 아래 면에서 그 자리만 지우고, 위에 그린 면은 없앤다.
    /// C++ `MainWindow::eraseOverlapWithShape` + `PolygonErase::plan/apply`.
    pub(crate) fn erase_overlap_with_selection(&mut self) -> bool {
        self.sync_view_into_survey_doc();
        let selected = self.all_selected();
        let (cutters, target) = match erase_plan(&self.survey_doc, &selected) {
            Ok(plan) => plan,
            Err(hint) => {
                self.home_map.draw_guide_message = Some(hint);
                return false;
            }
        };
        let Some(target_geom) = polygon_of(&self.survey_doc, target) else {
            return false;
        };
        let mut rest = MultiPolygon::new(vec![target_geom]);
        for c in &cutters {
            if let Some(g) = polygon_of(&self.survey_doc, *c) {
                rest = rest.difference(&MultiPolygon::new(vec![g]));
            }
        }
        let parts = pieces(rest);
        let Some((first, others)) = parts.split_first() else {
            self.home_map.draw_guide_message = Some("남는 부분이 없어 지우지 않았습니다.".to_owned());
            return false;
        };

        self.push_undo();
        let layer_name = target.0.name_ko();
        let layer = self.survey_doc.layer_mut(target.0);
        layer.features[target.1].set_geom(from_geo(first));
        // 둘 이상으로 갈라지면 나머지 조각은 같은 기록을 가진 새 도형이 된다(uid 는 비운다).
        let template = layer.features[target.1].clone();
        for part in others {
            let mut extra = template.clone();
            extra.fid = None;
            extra.attrs.remove("uid");
            extra.set_geom(from_geo(part));
            layer.features.push(extra);
        }
        remove_features(&mut self.survey_doc, cutters);
        self.after_doc_edit();
        self.home_map.draw_guide_message = Some(if others.is_empty() {
            format!("「{layer_name}」 도형 1개에서 겹친 자리를 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다.")
        } else {
            format!(
                "「{layer_name}」 도형 1개에서 겹친 자리를 지웠습니다. 도형이 {}개로 갈라졌습니다. Ctrl+Z로 되돌릴 수 있습니다.",
                others.len() + 1
            )
        });
        true
    }

    /// 「폴리곤 묶기」 — 고른 면 2개 이상을 하나로 합친다. 먼저 그린 도형의 기록이 남는다.
    /// C++ `MainWindowExport.cpp:466-506` + `LayerOps::mergePolygonFeatures`.
    pub(crate) fn merge_selected_polygons(&mut self) -> bool {
        self.sync_view_into_survey_doc();
        let mut picks: Vec<Pick> = self
            .all_selected()
            .into_iter()
            .filter(|p| polygon_of(&self.survey_doc, *p).is_some())
            .collect();
        if picks.len() < 2 {
            self.home_map.draw_guide_message = Some(
                "묶을 면을 2개 이상 고른 뒤 누르세요. [도형선택]으로 면을 클릭하고, Shift를 누른 채 다른 면을 더 고릅니다."
                    .to_owned(),
            );
            return false;
        }
        if picks.iter().any(|p| p.0 != picks[0].0) {
            self.home_map.draw_guide_message =
                Some("같은 레이어의 면끼리만 묶을 수 있습니다.".to_owned());
            return false;
        }
        picks.sort_by_key(|p| p.1);
        let mut merged = MultiPolygon::new(Vec::new());
        for p in &picks {
            if let Some(g) = polygon_of(&self.survey_doc, *p) {
                merged = merged.union(&MultiPolygon::new(vec![g]));
            }
        }
        let parts = pieces(merged);
        let [one] = parts.as_slice() else {
            // 조사 파일의 면 표는 한 도형에 면 하나만 담는다. 떨어진 면은 하나로 묶을 수 없다.
            self.home_map.draw_guide_message = Some(
                "고른 면들이 서로 닿지 않아 하나로 묶을 수 없습니다. 닿거나 겹친 면을 고르세요.".to_owned(),
            );
            return false;
        };

        self.push_undo();
        let keep = picks[0];
        let n = picks.len();
        self.survey_doc.layer_mut(keep.0).features[keep.1].set_geom(from_geo(one));
        remove_features(&mut self.survey_doc, picks[1..].to_vec());
        self.after_doc_edit();
        self.home_map.draw_guide_message = Some(format!(
            "「{}」의 폴리곤 {n}개를 1개로 묶었습니다. Ctrl+Z로 되돌릴 수 있습니다.",
            keep.0.name_ko()
        ));
        true
    }
}

#[cfg(test)]
#[path = "survey_edit_ops_tests.rs"]
mod tests;
