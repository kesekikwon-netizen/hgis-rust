//! 「폴리곤 묶기」 — C++ `MainWindow::mergeFeaturePolygons` (MainWindowExport.cpp:442-510) with
//! `GeometryEditOps::mergeKeeperId`, `mergeConflicts`, `mergeConflictSummary` and
//! `LayerOps::mergePolygonFeatures` (GeometryEditMerge.cpp:26-152): the shapes selected with Shift in one
//! layer become one shape that keeps the values of the first drawn, as one step; differing values ask first;
//! shapes apart are refused in a layer of single polygons.

use std::collections::BTreeMap;

use strata_geo::polygon_erase::{merged, Rings};
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::StrataAppState;
use crate::app_state_layer_remove::UndoStep;
use crate::checklist_state_geometry::parts_wkt;

/// The order the shapes were drawn in: saved ones by number, then unsaved ones -1, -2 … (`drawnBefore`, :27-30).
pub(crate) fn draw_order(ids: &[i64]) -> Vec<i64> {
    let mut order = ids.to_vec();
    order.sort_by(|a, b| match (*a >= 0, *b >= 0) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (true, true) => a.cmp(b),
        (false, false) => b.cmp(a),
    });
    order
}

/// The fields whose values differ from the first row (the first drawn), with every value that is there, by
/// the field's Korean name (`mergeConflicts`, :44-78). The identifier and the area label do not count.
pub(crate) fn merge_conflicts(rows: &[BTreeMap<String, String>]) -> Vec<(String, Vec<String>)> {
    let Some(first) = rows.first() else { return Vec::new() };
    let fields: std::collections::BTreeSet<&String> = rows.iter().flat_map(BTreeMap::keys).collect();
    let mut out = Vec::new();
    for field in fields.into_iter().filter(|field| !field.eq_ignore_ascii_case("uid") && field.as_str() != "area_m2") {
        let kept = first.get(field).map_or("", |value| value.trim());
        let mut values: Vec<String> = Vec::new();
        let mut lost = false;
        for value in rows.iter().map(|row| row.get(field).map_or("", |value| value.trim())).filter(|value| !value.is_empty()) {
            lost |= value != kept;
            if !values.iter().any(|known| known == value) {
                values.push(value.to_owned());
            }
        }
        if lost {
            out.push((crate::feature_record::field_label_ko(field), values));
        }
    }
    out
}

/// The words of the question (`mergeConflictSummary`, :80-97: six fields at most, four values each).
pub(crate) fn merge_conflict_summary(conflicts: &[(String, Vec<String>)]) -> String {
    let mut lines = vec!["고른 도형의 속성이 서로 다릅니다. 가장 먼저 그린 도형의 값만 남습니다.".to_owned()];
    for (field, values) in conflicts.iter().take(6) {
        let mut shown: Vec<&str> = values.iter().take(4).map(String::as_str).collect();
        if values.len() > 4 {
            shown.push("…");
        }
        lines.push(format!("· {field}: {}", shown.join(", ")));
    }
    if conflicts.len() > 6 {
        lines.push(format!("· 그 밖에 {}개 항목", conflicts.len() - 6));
    }
    lines.push(String::new());
    lines.push("묶은 뒤에도 Ctrl+Z로 되돌릴 수 있습니다.".to_owned());
    lines.join("\n")
}

pub(super) fn polygons(wkt: &str) -> Option<Vec<Rings>> {
    let parts = wkt_to_parts(wkt).ok()?;
    let rings: Vec<Rings> = parts.into_iter().filter_map(|part| if let WkbGeometry::Polygon(rings) = part { Some(rings) } else { None }).collect();
    (!rings.is_empty()).then_some(rings)
}

impl StrataAppState {
    /// Every selected shape that is still there: those taken with Shift first, the last one taken at the end.
    pub(crate) fn all_selected(&self) -> Vec<(String, i64)> {
        let Some(last) = self.home_map.selected_shape.clone() else { return Vec::new() };
        let there = |(table, fid): &(String, i64)| self.survey_tables.iter().any(|t| t.table == *table && t.layer.features.iter().any(|f| f.fid == *fid));
        self.home_map.also_selected.iter().cloned().chain(std::iter::once(last)).filter(there).collect()
    }

    /// 「폴리곤 묶기」.
    pub(crate) fn merge_selected_polygons(&mut self) {
        self.merge_selected(false);
    }

    /// 「예」 to the question about differing values.
    pub(crate) fn merge_selected_polygons_confirmed(&mut self) {
        self.home_map.merge_ask = None;
        self.merge_selected(true);
    }

    fn merge_selected(&mut self, confirmed: bool) {
        let all = self.all_selected();
        let Some((target, _)) = all.first().cloned() else { return self.show_status_message(FEWER) };
        let ids: Vec<i64> = all.iter().filter(|(table, _)| *table == target).map(|(_, fid)| *fid).collect();
        let others: Vec<String> = all.iter().filter(|(table, _)| *table != target).map(|(table, _)| self.layer_label_of(table)).collect();
        if ids.len() < 2 {
            return self.show_status_message(FEWER);
        }
        let Some(table) = self.survey_tables.iter().find(|t| t.table == target) else { return };
        if !table.geometry_type.to_ascii_uppercase().contains("POLYGON") {
            return self.show_status_message("폴리곤 레이어만 묶을 수 있습니다");
        }
        let order = draw_order(&ids);
        let rows: Vec<BTreeMap<String, String>> = order.iter().filter_map(|fid| table.layer.features.iter().find(|f| f.fid == *fid)).map(|f| f.attributes.clone()).collect();
        let conflicts = merge_conflicts(&rows);
        if !confirmed && !conflicts.is_empty() {
            self.home_map.merge_ask = Some(format!("{}\n\n묶을까요?", merge_conflict_summary(&conflicts)));
            return;
        }
        let shapes: Option<Vec<Vec<Rings>>> = order.iter().map(|fid| table.layer.features.iter().find(|f| f.fid == *fid).and_then(|f| polygons(&f.wkt_geom))).collect();
        let pieces = shapes.map(|shapes| merged(&shapes)).unwrap_or_default();
        if pieces.is_empty() {
            return self.show_status_message("폴리곤 결합 실패");
        }
        let multi = table.geometry_type.to_ascii_uppercase().starts_with("MULTI");
        if pieces.len() > 1 && !multi {
            return self.show_status_message("떨어진 구역은 하나로 묶을 수 없습니다. 두 구역은 따로 저장되고 제출에도 함께 들어갑니다. 맞닿거나 겹친 구역만 하나로 묶습니다.");
        }
        let wkt = parts_wkt(&pieces.into_iter().map(WkbGeometry::Polygon).collect::<Vec<_>>());
        // The first drawn keeps its number and its values and takes the merged shape; the others go.
        let (keeper, gone) = (order[0], order[1..].to_vec());
        let table = self.survey_tables.iter_mut().find(|t| t.table == target);
        if table.and_then(|t| t.layer.change_add_delete(&[(keeper, wkt)], Vec::new(), &gone)).is_none() {
            return self.show_status_message("면을 묶지 못했습니다");
        }
        self.redo_tables.clear();
        self.after_layer_edit(&target);
        self.undo_tables.push(UndoStep::Table(target.clone()));
        (self.home_map.selected_shape, self.home_map.also_selected, self.home_map.active_vertex) = (None, Vec::new(), None);
        let label = self.layer_label_of(&target);
        let done = format!("「{label}」의 폴리곤 {}개를 1개로 묶었습니다. Ctrl+Z로 되돌릴 수 있습니다.", order.len());
        if others.is_empty() {
            self.show_status_message(done);
        } else {
            // KaUserError::warn (:459-465), in the status line until a window is there; the merge goes on.
            self.show_status_message(format!("한 번에 한 레이어의 면만 묶을 수 있습니다. 「{label}」의 면만 묶고 다음 레이어의 선택은 쓰지 않습니다: {}. {done}", others.join(", ")));
        }
    }

    /// The question of 「폴리곤 묶기」 (QMessageBox::question, default 아니요, :484-489).
    pub(crate) fn render_merge_ask(&mut self, ctx: &eframe::egui::Context) {
        use eframe::egui;
        let Some(words) = self.home_map.merge_ask.clone() else { return };
        let (mut yes, mut no) = (false, false);
        egui::Window::new("폴리곤 묶기").id(egui::Id::new("ka_merge_ask")).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(ctx, |ui| {
            ui.set_max_width(440.0);
            ui.label(words);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                yes = ui.button("예").clicked();
                no = ui.button("아니요").clicked();
            });
            no |= ui.input(|i| i.key_pressed(egui::Key::Escape));
        });
        if yes {
            self.merge_selected_polygons_confirmed();
        } else if no {
            self.home_map.merge_ask = None;
        }
    }
}

/// `QMessageBox::information` when fewer than two shapes of a layer are selected (:468-474).
const FEWER: &str = "묶을 면을 2개 이상 고른 뒤 누르세요. [도형선택]으로 면을 클릭하고, Shift를 누른 채 다른 면을 더 고릅니다.";

#[cfg(test)]
#[path = "app_state_merge_tests.rs"]
mod tests;
