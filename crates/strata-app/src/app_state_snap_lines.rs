//! 「가까운 선에 붙이기」 — C++ `SnapTraceOps::snapCanvasShapesToNearestLines`·`snapFeatureToNearestLines`
//! (core/SnapTraceOps.cpp:204-284). 대상: 고른 도형, 없으면 방금 그린 면, 없으면 조사구역→유구 면→유구 선→시굴격자에서
//! 처음 있는 레이어의 마지막 도형. 붙일 선: 보이는 지적·참조 선, 그다음 보이는 조사 레이어(대상 도형 빼고) — eligibleLayers.
//! 바꾼 도형은 그 레이어 편집 버퍼에 되돌리기 한 단계로 들어간다(파일에는 「저장」이 쓴다).

use strata_canvas2d::cad_ring_snap::snap_ring_to_nearest_lines;
use strata_engine::Transformer;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::StrataAppState;
use crate::checklist_state_geometry::{moved, parts_wkt};

const KEYS: [&str; 4] = ["survey_area", "feature_poly", "feature_line", "trial_trench"];

impl StrataAppState {
    fn work_epsg(&self) -> u32 {
        strata_types::crs::CrsCode::from(self.work_crs).epsg_number()
    }

    /// 표 좌표계 ↔ 작업 좌표계 변환기(같으면 None).
    fn table_transformers(&self, srs_id: i32) -> Option<(Transformer, Transformer)> {
        let work = self.work_epsg();
        let src = u32::try_from(srs_id).ok().filter(|s| *s > 0 && *s != work)?;
        Some((Transformer::new(src, work).ok()?, Transformer::new(work, src).ok()?))
    }

    /// 보이는 조사 레이어의 고리·선(작업 좌표계). `exclude` 도형은 뺀다(toCanvasAndFetch 의 ignoreFid).
    fn survey_snap_lines(&self, exclude: Option<(&str, i64)>) -> Vec<Vec<[f64; 2]>> {
        let mut out = Vec::new();
        for e in self.left_layers.entries.iter().filter(|e| self.home_map.real_survey.is_layer_visible(&e.label_ko)) {
            let Some(t) = self.survey_tables.iter().find(|t| t.table == e.layer_key) else { continue };
            let tr = self.table_transformers(t.srs_id);
            for f in t.layer.features.iter().filter(|f| exclude != Some((t.table.as_str(), f.fid))) {
                let Some(parts) = wkt_to_parts(&f.wkt_geom).ok() else { continue };
                let parts = match &tr {
                    Some((to_work, _)) => moved(&parts, to_work).unwrap_or_default(),
                    None => parts,
                };
                for p in parts {
                    match p {
                        WkbGeometry::LineString(l) => out.push(l),
                        WkbGeometry::Polygon(rings) => out.extend(rings),
                        WkbGeometry::Point(_) => {}
                    }
                }
            }
        }
        out
    }

    /// 붙일 대상들(snapCanvasShapesToNearestLines). 고른 것이 없으면 대상을 골라 둔다(selectByIds).
    fn snap_targets(&mut self) -> Vec<(String, i64)> {
        let has_geometry = |app: &Self, (table, fid): &(String, i64)| {
            app.survey_tables.iter().any(|t| &t.table == table && t.layer.features.iter().any(|f| f.fid == *fid && !f.wkt_geom.trim().is_empty()))
        };
        if let Some(sel) = self.home_map.selected_shape.clone().filter(|s| has_geometry(self, s)) {
            return vec![sel];
        }
        if let Some(last) = self.home_map.last_drawn_shape.clone().filter(|s| has_geometry(self, s)) {
            self.home_map.selected_shape = Some(last.clone());
            return vec![last];
        }
        for key in KEYS {
            let last = self.survey_tables.iter().find(|t| t.table == key).and_then(|t| {
                t.layer.features.iter().rev().find(|f| !f.wkt_geom.trim().is_empty()).map(|f| (t.table.clone(), f.fid))
            });
            if let Some(target) = last {
                self.home_map.selected_shape = Some(target.clone());
                return vec![target];
            }
        }
        Vec::new()
    }

    /// 도형 하나를 붙인다(snapFeatureToNearestLines) — (옮긴 꼭짓점, 채운 사이점), 안 바뀌면 None.
    fn snap_one(&mut self, table: &str, fid: i64) -> Option<(usize, usize)> {
        let t = self.survey_tables.iter().find(|t| t.table == table)?;
        let f = t.layer.features.iter().find(|f| f.fid == fid)?;
        let parts = wkt_to_parts(&f.wkt_geom).ok().filter(|p| !p.is_empty())?;
        let tr = self.table_transformers(t.srs_id);
        let in_work = match &tr {
            Some((to_work, _)) => moved(&parts, to_work)?,
            None => parts,
        };
        let mut refs: Vec<Vec<[f64; 2]>> = self.home_map.real_survey.cadastral_parcels.iter().map(|p| p.ring.clone()).collect();
        refs.extend(self.home_map.real_survey.heritage_polys.iter().map(|h| h.ring.clone()));
        refs.extend(self.survey_snap_lines(Some((table, fid))));
        let mupp = self.home_map.viewport.meters_per_pixel;
        // 면은 첫 조각의 바깥 고리, 선은 첫 조각(C++ asPolygon·asMultiPolygon().value(0)).
        let (snapped, moved_n, inserted) = match in_work.first()? {
            WkbGeometry::Polygon(rings) => {
                let (ring, m, i) = snap_ring_to_nearest_lines(rings.first()?, true, &refs, mupp);
                let mut out = rings.clone();
                out[0] = ring;
                (WkbGeometry::Polygon(out), m, i)
            }
            WkbGeometry::LineString(l) => {
                let (line, m, i) = snap_ring_to_nearest_lines(l, false, &refs, mupp);
                (WkbGeometry::LineString(line), m, i)
            }
            WkbGeometry::Point(_) => return None,
        };
        if moved_n == 0 && inserted == 0 {
            return None;
        }
        let back = match &tr {
            Some((_, to_table)) => moved(&[snapped], to_table)?,
            None => vec![snapped],
        };
        let wkt = parts_wkt(&back);
        let table = table.to_owned();
        let t = self.survey_tables.iter_mut().find(|t| t.table == table)?;
        t.layer.change_geometry(fid, wkt).then_some(())?;
        self.redo_tables.clear();
        self.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table(table));
        self.is_dirty = true;
        Some((moved_n, inserted))
    }

    /// 「가까운 선에 붙이기」 — 바꾼 도형 수. 상태줄에 C++ 글.
    pub(crate) fn snap_shapes_to_nearest_lines(&mut self) -> usize {
        let targets = self.snap_targets();
        if targets.is_empty() {
            self.show_status_message("먼저 지적도·도면 위에 도형을 그리거나 도형선택으로 도형을 고르세요.");
            return 0;
        }
        let (mut changed, mut total_moved, mut total_inserted) = (0usize, 0usize, 0usize);
        for (table, fid) in targets {
            if let Some((m, i)) = self.snap_one(&table, fid) {
                changed += 1;
                total_moved += m;
                total_inserted += i;
            }
        }
        self.show_status_message(if changed > 0 {
            format!("도형을 가장 가까운 선에 붙였습니다 (꼭짓점 {total_moved}개 맞춤 · 사이점 {total_inserted}개 채움). Ctrl+Z로 되돌릴 수 있습니다.")
        } else {
            "이미 가장 가까운 선에 붙어 있거나 주변에 붙일 기준 선(지적·도면)이 없습니다.".to_owned()
        });
        changed
    }
}

#[cfg(test)]
#[path = "app_state_snap_lines_tests.rs"]
mod tests;
