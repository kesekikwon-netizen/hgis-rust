//! 「폴리곤 나누기」 — C++ `MainWindow::startSplitPolygonTool` (MainWindowEditing.cpp:1193-1280) and what it
//! starts: two selected shapes give their shared place as a new shape (`LayerOps::splitTwoOverlappingFeatures`,
//! LayerOps.cpp:876-1019); a selected shape of several polygons comes apart (`explodeMultipartFeatures`,
//! :599-674); else a line is drawn across a polygon and right click cuts along it (`splitPolygonWithLine`,
//! :800-874; MainWindowEditing.cpp:794-816). Every cut is one step Ctrl+Z takes back.

use std::collections::BTreeMap;

use strata_canvas2d::map_tools::MapToolKind;
use strata_geo::polygon_erase::Rings;
use strata_geo::polygon_split::{overlap, sliced};
use strata_storage::wkb_codec::WkbGeometry;

use super::app_state_draw_commit::TOO_FEW_TEXT;
use super::app_state_merge::polygons;
use crate::app_state::StrataAppState;
use crate::app_state_layer_remove::UndoStep;
use crate::checklist_state_geometry::parts_wkt;

type ShapeId = (String, i64);

fn polygon_wkt(rings: Rings) -> String {
    parts_wkt(&[WkbGeometry::Polygon(rings)])
}

fn without_uid(mut values: BTreeMap<String, String>) -> BTreeMap<String, String> {
    values.retain(|name, _| !name.eq_ignore_ascii_case("uid"));
    values
}

impl StrataAppState {
    /// The polygons of a shape in the CRS of the table `into` (its own when they are the same).
    fn shape_into(&self, shape: &ShapeId, into: &str) -> Option<Vec<Rings>> {
        let from = self.survey_tables.iter().find(|t| t.table == shape.0)?;
        let to = self.survey_tables.iter().find(|t| t.table == into)?;
        let rings = polygons(&from.layer.features.iter().find(|f| f.fid == shape.1)?.wkt_geom)?;
        if from.srs_id == to.srs_id || from.srs_id <= 0 || to.srs_id <= 0 {
            return Some(rings);
        }
        let carry = strata_engine::Transformer::new(u32::try_from(from.srs_id).ok()?, u32::try_from(to.srs_id).ok()?).ok()?;
        rings.into_iter().map(|polygon| polygon.into_iter().map(|ring| ring.into_iter().map(|p| carry.transform(p).ok()).collect()).collect()).collect()
    }

    /// One layer's changes as one step.
    fn apply_split(&mut self, key: &str, changes: &[(i64, String)], additions: Vec<(String, BTreeMap<String, String>)>) -> bool {
        let table = self.survey_tables.iter_mut().find(|t| t.table == key);
        if table.and_then(|t| t.layer.change_add_delete(changes, additions, &[])).is_none() {
            return false;
        }
        // target/cpp-ref/src/app/MainWindowSession.cpp:1225-1228: modified split buffers are unsaved work.
        self.is_dirty = true;
        self.redo_tables.clear();
        self.after_layer_edit(key);
        self.undo_tables.push(UndoStep::Table(key.to_owned()));
        true
    }

    /// 「폴리곤 나누기」.
    pub(crate) fn start_split_polygon_tool(&mut self) {
        let selected = self.all_selected();
        if let [first, second] = &selected[..] {
            return self.split_two_selected(first, second);
        }
        if self.explode_selected(&selected) {
            return;
        }
        let target = selected.first().map(|(table, _)| table.clone()).or_else(|| self.current_legend_layer().map(|e| e.layer_key.clone()));
        let polygon_layer = target.filter(|key| self.survey_tables.iter().any(|t| t.table == *key && t.geometry_type.to_ascii_uppercase().contains("POLYGON")));
        let Some(table) = polygon_layer else {
            return self.show_status_message("도형을 Shift+클릭으로 선택하거나, 나눌 폴리곤 레이어를 좌측 목록에서 선택해 주세요.");
        };
        self.edit_layer_key = Some(table.clone());
        self.active_tool = MapToolKind::DrawLine;
        self.draw_sketch = crate::draw_sketch::DrawSketch::default();
        self.home_map.splitting = Some(table);
        self.show_status_message("폴리곤 나누기 모드 — 폴리곤을 가로지르는 선을 클릭하여 그리고 우클릭으로 분할 (또는 Shift로 2개 도형 선택 후 실행).");
    }

    /// The line drawn (work CRS) cuts the selected polygons of the layer, else every polygon it crosses.
    pub(crate) fn split_polygons_with_line(&mut self, key: &str, line: Vec<[f64; 2]>) {
        const FAILED: &str = "선으로 면을 나누지 못했습니다.";
        if line.len() < 2 {
            return self.show_status_message(format!("{FAILED} 분할선은 최소 2개 이상의 점이어야 합니다."));
        }
        let chosen: Vec<i64> = self.all_selected().into_iter().filter(|(table, _)| table == key).map(|(_, fid)| fid).collect();
        let Some(t) = self.survey_tables.iter().find(|t| t.table == key) else { return };
        let work = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        // Owner CRS rule and target/cpp-ref/src/app/KaCaptureMapTool.cpp:437-448: unknown is not identity.
        let line = match u32::try_from(t.srs_id) {
            Ok(table_srs) if table_srs == work => Some(line),
            Ok(table_srs) => strata_engine::Transformer::new(work, table_srs).ok().and_then(|carry| carry.transform_all(&line).ok()),
            Err(_) => None,
        };
        let Some(line) = line else { return self.show_status_message(TOO_FEW_TEXT) };
        // target/cpp-ref/src/app/KaCaptureMapTool.cpp:403-408,451-455: consume only an admitted capture.
        self.draw_sketch = crate::draw_sketch::DrawSketch::default();
        let (mut changes, mut additions) = (Vec::new(), Vec::new());
        for f in t.layer.features.iter().filter(|f| chosen.is_empty() || chosen.contains(&f.fid)) {
            let Some(shape) = polygons(&f.wkt_geom) else { continue };
            let pieces = sliced(&shape, &line);
            if pieces.len() <= shape.len() {
                continue;
            }
            let mut pieces = pieces.into_iter();
            changes.extend(pieces.next().map(|largest| (f.fid, polygon_wkt(largest))));
            additions.extend(pieces.map(|piece| (polygon_wkt(piece), without_uid(f.attributes.clone()))));
        }
        if changes.is_empty() {
            return self.show_status_message(format!("{FAILED} 분할선이 관통하는 폴리곤이 없습니다. 폴리곤 양쪽 경계를 완전히 가로질러야 합니다."));
        }
        if self.apply_split(key, &changes, additions) {
            self.show_status_message("폴리곤을 성공적으로 나누었습니다.");
        }
    }

    /// Two selected shapes: their shared place becomes a new shape with the values of the first — of the
    /// second when the first is the survey area — and both stay as they were (`:952-988`).
    fn split_two_selected(&mut self, first: &ShapeId, second: &ShapeId) {
        let boundary = first.0 == "survey_area" || self.layer_label_of(&first.0).contains("바운더리");
        let target = if boundary { second } else { first };
        let shared = match (self.shape_into(first, &target.0), self.shape_into(second, &target.0)) {
            (Some(a), Some(b)) => overlap(&a, &b),
            _ => Vec::new(),
        };
        if shared.is_empty() {
            return self.show_status_message("두 도형의 겹친 구간을 나누지 못했습니다. 선택한 두 도형이 서로 겹치지 않습니다. 겹치는 도형 두 개를 Shift+클릭으로 다시 고른 뒤 누르세요.");
        }
        let Some(values) = self.survey_tables.iter().find(|t| t.table == target.0).and_then(|t| t.layer.features.iter().find(|f| f.fid == target.1)).map(|f| without_uid(f.attributes.clone())) else { return };
        // A layer of single polygons keeps each piece as a shape of its own.
        let additions = shared.into_iter().map(|piece| (polygon_wkt(piece), values.clone())).collect();
        if self.apply_split(&target.0, &[], additions) {
            self.show_status_message("선택한 두 도형의 겹치는 구간을 잘라 분할했습니다! (A, B, 중첩부 3개로 분할됨)");
        }
    }

    /// The selected shapes of the first one's layer that hold several polygons come apart, one shape each;
    /// `false` when none does.
    fn explode_selected(&mut self, selected: &[ShapeId]) -> bool {
        let Some((key, _)) = selected.first() else { return false };
        let Some(t) = self.survey_tables.iter().find(|t| t.table == *key) else { return false };
        let (mut changes, mut additions) = (Vec::new(), Vec::new());
        for (_, fid) in selected.iter().filter(|(table, _)| table == key) {
            let Some(f) = t.layer.features.iter().find(|f| f.fid == *fid) else { continue };
            let Some(shape) = polygons(&f.wkt_geom).filter(|shape| shape.len() > 1) else { continue };
            let mut parts = shape.into_iter();
            changes.extend(parts.next().map(|first| (f.fid, polygon_wkt(first))));
            additions.extend(parts.map(|part| (polygon_wkt(part), without_uid(f.attributes.clone()))));
        }
        let key = key.clone();
        if changes.is_empty() || !self.apply_split(&key, &changes, additions) {
            return false;
        }
        self.show_status_message("그룹으로 묶여 있던 폴리곤을 개별 폴리곤들로 분리했습니다!");
        true
    }
}

#[cfg(test)]
#[path = "app_state_split_tests.rs"]
mod tests;
