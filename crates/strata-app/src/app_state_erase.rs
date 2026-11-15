//! 「겹친 곳 지우기」 — a shape drawn on top cuts its place out of the shape under it and is removed itself
//! (C++ `MainWindow::eraseOverlapWithShape` · `runErasePlan`, MainWindowErase.cpp:23-118; the plan and its
//! hints, `PolygonErase::plan` · `apply`, core/PolygonErase.cpp:162-345). One shape can be selected in this
//! app, so the plan is the one for a single selected shape; several shapes at once come with the selection
//! of several shapes.

use std::collections::BTreeMap;

use strata_geo::polygon_erase::{erased, overlap_with_area, Rings};
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::app_state_layer_remove::UndoStep;
use crate::checklist_state_geometry::{moved, parts_wkt};

type ShapeId = (String, i64);

/// What one layer gets in the one step (`LayerEdit`).
#[derive(Default)]
struct LayerEdit {
    changes: Vec<(i64, String)>,
    additions: Vec<(String, BTreeMap<String, String>)>,
    deletions: Vec<i64>,
}

fn polygons(wkt: &str) -> Option<Vec<Rings>> {
    let parts = wkt_to_parts(wkt).ok()?;
    let rings: Vec<Rings> = parts.into_iter().filter_map(|part| if let WkbGeometry::Polygon(rings) = part { Some(rings) } else { None }).collect();
    (!rings.is_empty()).then_some(rings)
}

fn area_of(wkt: &str) -> f64 {
    strata_engine::wkt_area(wkt).map_or(0.0, f64::abs)
}

impl StrataAppState {
    fn table(&self, key: &str) -> Option<&SurveyFileTable> {
        self.survey_tables.iter().find(|t| t.table == key)
    }

    /// `PolygonErase::editable`: a polygon layer of the survey's own data that can be changed.
    fn erasable(&self, key: &str) -> bool {
        let polygons = self.table(key).is_some_and(|t| t.geometry_type.to_ascii_uppercase().contains("POLYGON"));
        polygons && self.left_layers.entries.iter().any(|e| e.layer_key == key && e.group == LegendGroup::SurveyData && !e.read_only)
    }

    fn shape_polygons(&self, (key, fid): &ShapeId) -> Option<Vec<Rings>> {
        polygons(&self.table(key)?.layer.features.iter().find(|f| f.fid == *fid)?.wkt_geom)
    }

    /// The cutter as the layer `to` holds its shapes (`moved`): carried into that layer's CRS.
    fn carried(&self, cutter: &[Rings], from: &str, to: &str) -> Option<Vec<Rings>> {
        let (from, to) = (self.table(from)?.srs_id, self.table(to)?.srs_id);
        if from == to {
            return Some(cutter.to_vec());
        }
        let carry = strata_engine::Transformer::new(u32::try_from(from).ok()?, u32::try_from(to).ok()?).ok()?;
        let parts: Vec<WkbGeometry> = cutter.iter().cloned().map(WkbGeometry::Polygon).collect();
        Some(moved(&parts, &carry)?.into_iter().filter_map(|part| if let WkbGeometry::Polygon(rings) = part { Some(rings) } else { None }).collect())
    }

    /// The shapes of a layer under the cutter, each with whether something of it remains (`shapesUnder`).
    fn shapes_under(&self, key: &str, cutter: &[Rings], skip: Option<i64>) -> Vec<(ShapeId, bool)> {
        let Some(table) = self.table(key) else { return Vec::new() };
        let under = table.layer.features.iter().filter(|f| Some(f.fid) != skip).filter_map(|f| Some((f.fid, polygons(&f.wkt_geom)?)));
        under.filter(|(_, shape)| overlap_with_area(shape, cutter)).map(|(fid, shape)| ((key.to_owned(), fid), !erased(&shape, cutter).is_empty())).collect()
    }

    /// `PolygonErase::plan` for one selected shape: the shapes to cut, or the words that say why not.
    fn erase_plan(&self, picked: Option<&ShapeId>) -> Result<(ShapeId, Vec<ShapeId>), String> {
        let Some(cutter) = picked else { return Err("위에 그린 도형을 [도형선택]으로 고른 뒤 누르세요.".to_owned()) };
        let Some(shape) = self.shape_polygons(cutter).filter(|_| self.erasable(&cutter.0)) else {
            return Err("고른 도형으로는 지울 수 없습니다. 조사 데이터의 면 도형을 고르세요.".to_owned());
        };
        let mut under = self.shapes_under(&cutter.0, &shape, Some(cutter.1));
        // Other layers are looked at only when nothing of the cutter's own layer lies under it; with several
        // shapes under it there, nobody can tell which one to cut: the user chooses.
        if under.is_empty() {
            let shown = self.left_layers.entries.iter().filter(|e| e.layer_key != cutter.0 && self.home_map.real_survey.is_layer_visible(&e.label_ko));
            for layer in shown.filter(|e| self.erasable(&e.layer_key)) {
                if let Some(there) = self.carried(&shape, &cutter.0, &layer.layer_key) {
                    under.extend(self.shapes_under(&layer.layer_key, &there, None));
                }
            }
            let remaining = under.iter().filter(|(_, remains)| *remains).count();
            if remaining > 1 {
                return Err(format!("아래에 겹친 도형이 {remaining}개입니다. Shift를 누른 채 지울 도형도 함께 고른 뒤 다시 누르세요."));
            }
        }
        let targets: Vec<ShapeId> = under.iter().filter(|(_, remains)| *remains).map(|(id, _)| id.clone()).collect();
        match (targets.is_empty(), under.is_empty()) {
            (false, _) => Ok((cutter.clone(), targets)),
            (true, false) => Err("고른 도형이 아래 도형을 모두 덮고 있어 남는 부분이 없습니다. 위에 그린 작은 도형을 고르세요.".to_owned()),
            (true, true) => Err("고른 도형 아래에 겹친 면이 없습니다.".to_owned()),
        }
    }

    /// `PolygonErase::apply`: everything is worked out first, then each layer gets its changes as one step.
    /// Returns the pieces that became shapes of their own, or the words that say what went wrong.
    fn apply_erase(&mut self, cutter: &ShapeId, targets: &[ShapeId]) -> Result<usize, String> {
        let shape = self.shape_polygons(cutter).ok_or("위에 그린 도형을 찾지 못했습니다. 도형을 다시 고르세요.")?;
        let mut edits: Vec<(String, LayerEdit)> = Vec::new();
        let mut added = 0;
        for target in targets {
            let table = self.table(&target.0).ok_or("고칠 수 없는 레이어의 도형입니다.")?;
            let feature = table.layer.features.iter().find(|f| f.fid == target.1).ok_or("지울 도형을 찾지 못했습니다. 도형을 다시 고르세요.")?;
            let from = polygons(&feature.wkt_geom).ok_or("지울 도형을 찾지 못했습니다. 도형을 다시 고르세요.")?;
            let with = self.carried(&shape, &cutter.0, &target.0).ok_or("위에 그린 도형을 찾지 못했습니다. 도형을 다시 고르세요.")?;
            let rest = erased(&from, &with);
            if rest.is_empty() {
                return Err("남는 부분이 없어 지우지 않았습니다.".to_owned());
            }
            // A layer of single polygons cannot keep several pieces as one shape: the largest piece stays the
            // shape and every other piece becomes a shape of its own (`partsFor`), without the uid.
            let pieces: Vec<String> = if table.geometry_type.to_ascii_uppercase().starts_with("MULTI") {
                vec![parts_wkt(&rest.into_iter().map(WkbGeometry::Polygon).collect::<Vec<_>>())]
            } else {
                rest.into_iter().map(|rings| parts_wkt(&[WkbGeometry::Polygon(rings)])).collect()
            };
            let mut attributes = feature.attributes.clone();
            attributes.retain(|name, _| !name.eq_ignore_ascii_case("uid"));
            let at = edits.iter().position(|(key, _)| *key == target.0).unwrap_or_else(|| {
                edits.push((target.0.clone(), LayerEdit::default()));
                edits.len() - 1
            });
            added += pieces.len() - 1;
            let mut pieces = pieces.into_iter();
            edits[at].1.changes.extend(pieces.next().map(|largest| (target.1, largest)));
            edits[at].1.additions.extend(pieces.map(|piece| (piece, attributes.clone())));
        }
        match edits.iter_mut().find(|(key, _)| *key == cutter.0) {
            Some((_, edit)) => edit.deletions.push(cutter.1),
            None => edits.push((cutter.0.clone(), LayerEdit { deletions: vec![cutter.1], ..LayerEdit::default() })),
        }
        let mut done: Vec<String> = Vec::new();
        for (key, edit) in edits {
            let table = self.survey_tables.iter_mut().find(|t| t.table == key);
            if table.and_then(|t| t.layer.change_add_delete(&edit.changes, edit.additions, &edit.deletions)).is_none() {
                // No half-erased state: the layers already changed go back.
                for table in self.survey_tables.iter_mut().filter(|t| done.contains(&t.table)) {
                    table.layer.undo();
                }
                return Err("지울 도형을 찾지 못했습니다. 도형을 다시 고르세요.".to_owned());
            }
            done.push(key);
        }
        self.redo_tables.clear();
        for key in &done {
            self.after_layer_edit(key);
        }
        self.undo_tables.push(if done.len() == 1 { UndoStep::Table(done.remove(0)) } else { UndoStep::Linked(done) });
        Ok(added)
    }

    /// The button 「겹친 곳 지우기」. With nothing selected the first press only selects the shape drawn last,
    /// so that the user sees which shape it is; the next press erases.
    pub(crate) fn erase_overlap_with_shape(&mut self) {
        let drawn = self.home_map.last_drawn_shape.clone().filter(|shape| self.erasable(&shape.0) && self.shape_polygons(shape).is_some());
        if let (None, Some(drawn)) = (&self.home_map.selected_shape, drawn) {
            self.home_map.selected_shape = Some(drawn);
            return self.show_status_message("방금 그린 도형을 골랐습니다. 이 모양대로 아래 도형에서 지우려면 [겹친 곳 지우기]를 한 번 더 누르세요.");
        }
        let (cutter, targets) = match self.erase_plan(self.home_map.selected_shape.as_ref()) {
            Ok(plan) => plan,
            Err(hint) => return self.show_status_message(hint),
        };
        let first = targets[0].clone();
        let area = |app: &Self| app.table(&first.0).and_then(|t| t.layer.features.iter().find(|f| f.fid == first.1)).map_or(0.0, |f| area_of(&f.wkt_geom));
        let (layer, before) = (self.left_layers.entries.iter().find(|e| e.layer_key == first.0).map(|e| e.label_ko.clone()).unwrap_or_default(), area(self));
        let added = match self.apply_erase(&cutter, &targets) {
            Ok(added) => added,
            Err(why) => return self.show_status_message(format!("지우지 못했습니다. {why}")),
        };
        self.is_dirty = true;
        self.home_map.selected_shape = None;
        // Ctrl+Z starts with what was just done: the cut layer is the current one.
        self.right_inspector.selected_layer_key = Some(layer.clone());
        let mut text = format!("「{layer}」 도형 {}개에서 겹친 자리를 지웠습니다.", targets.len());
        if targets.len() == 1 && added == 0 {
            text.push_str(&format!(" 면적 {} ㎡ → {} ㎡.", crate::survey_measure::grouped2(before), crate::survey_measure::grouped2(area(self))));
        }
        if added > 0 {
            text.push_str(&format!(" 떨어진 조각 {added}개는 따로 도형이 되었습니다."));
        }
        self.show_status_message(text + " 위에 그린 도형은 없앴습니다. Ctrl+Z로 되돌립니다.");
    }
}

#[cfg(test)]
#[path = "app_state_erase_tests.rs"]
mod tests;
