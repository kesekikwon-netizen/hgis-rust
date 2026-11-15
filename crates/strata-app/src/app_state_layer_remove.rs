//! 「레이어 삭제」와 Ctrl+Z 복원 — C++ `removeLayersFromTree`(MainWindowUndo.cpp:158-288)·되돌리기 `LayersRemoved`(:369-389).
//! 목록과 지도에서만 뺀다: 원본 파일은 그대로, 그 레이어(저장 안 한 편집 포함)는 되돌리기 단계 안에 둔다 — 그동안 그리지도
//! 저장하지도 검수에 세지도 않는다. 되돌리면 같은 자리로. C++ 에는 레이어 삭제의 「다시 실행」이 없다.

use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::schema::LegendGroup;

use crate::app_state::{MainTab, StrataAppState};
use crate::left_layer_panel::LayerTreeEntry;

/// 되돌리기 한 단계 — 그 표의 편집 하나, 또는 레이어 삭제 하나(KaUndoAction::LayersRemoved).
#[derive(Debug, Clone)]
pub(crate) enum UndoStep {
    Table(String),
    LayersRemoved(Vec<RemovedLayer>),
    /// A row of the legend was dragged: the keys of the rows in the order they had (KaUndoAction::LayerTreeChanged).
    RowsMoved(Vec<String>),
    /// One edit that changed several tables at once (겹친 곳 지우기 across layers): they go back together
    /// (`undoLinkedEdit`, MainWindowUndo.cpp:343-344).
    Linked(Vec<String>),
}

/// 뺀 레이어 하나와 그 자리.
#[derive(Debug, Clone)]
pub(crate) struct RemovedLayer {
    index: usize,
    entry: LayerTreeEntry,
    table: Option<(usize, SurveyFileTable)>,
}

impl StrataAppState {
    /// 레이어 삭제 — `keys` 의 레이어를 목록·지도에서 뺀다(묶음 머리면 그 묶음의 레이어 모두).
    pub(crate) fn remove_layers(&mut self, keys: &[String]) -> bool {
        // 자리는 빼기 전에 모두 적어 둔다(목록 차례대로).
        let picked: Vec<usize> = self.left_layers.entries.iter().enumerate().filter(|(_, e)| keys.contains(&e.layer_key)).map(|(i, _)| i).collect();
        if picked.is_empty() {
            self.show_status_message("제거할 레이어를 먼저 클릭하세요.".to_owned());
            return false;
        }
        let mut removed = Vec::with_capacity(picked.len());
        for &index in picked.iter().rev() {
            let entry = self.left_layers.entries.remove(index);
            let key = entry.layer_key.clone();
            if self.edit_layer_key.as_deref() == Some(key.as_str()) {
                self.stop_capture_tool();
                self.edit_layer_key = None;
            }
            if self.home_map.selected_shape.as_ref().is_some_and(|(t, _)| *t == key) {
                self.home_map.selected_shape = None;
            }
            if self.right_inspector.selected_layer_key.as_deref() == Some(entry.label_ko.as_str()) {
                self.right_inspector.selected_layer_key = None;
            }
            let table = self.survey_tables.iter().position(|t| t.table == key).map(|i| (i, self.survey_tables.remove(i)));
            removed.push(RemovedLayer { index, entry, table });
        }
        removed.reverse();
        self.drop_card_if_gone();
        self.redo_tables.clear();
        self.undo_tables.push(UndoStep::LayersRemoved(removed));
        self.is_dirty = true;
        self.show_status_message("레이어를 목록에서 제거했습니다. Ctrl+Z로 복원할 수 있습니다. 원본 파일은 그대로입니다.".to_owned());
        true
    }

    /// 되돌리기 — 뺀 자리에 다시 넣고 마지막 레이어를 현재로.
    pub(crate) fn restore_removed_layers(&mut self, removed: Vec<RemovedLayer>) {
        for r in removed {
            // A reference map that was asked for again in the meantime already has its row: one row a map.
            if (crate::ref_tile_layers::ref_tile_layer(&r.entry.layer_key).is_some() || self.ref_vectors.has(&r.entry.layer_key))
                && self.left_layers.entries.iter().any(|e| e.layer_key == r.entry.layer_key)
            {
                continue;
            }
            let at = r.index.min(self.left_layers.entries.len());
            self.right_inspector.selected_layer_key = Some(r.entry.label_ko.clone());
            self.left_layers.entries.insert(at, r.entry);
            if let Some((i, t)) = r.table {
                let at = i.min(self.survey_tables.len());
                self.survey_tables.insert(at, t);
            }
        }
        self.is_dirty = true;
    }

    /// 지금 빼 둔(되돌리기 단계 안의) 표 이름들 — 저장 뒤 파일에서 다시 읽은 표에서 뺀다.
    pub(crate) fn removed_table_keys(&self) -> Vec<String> {
        self.undo_tables
            .iter()
            .filter_map(|s| if let UndoStep::LayersRemoved(r) = s { Some(r) } else { None })
            .flatten()
            .filter_map(|r| r.table.as_ref().map(|(_, t)| t.table.clone()))
            .collect()
    }

    /// 묶음 머리 「레이어 삭제」 — 그 묶음의 레이어 모두(removableLegendLayersFromNode).
    pub(crate) fn remove_group_layers(&mut self, group: strata_types::schema::LegendGroup) -> bool {
        let keys: Vec<String> = self.left_layers.entries.iter().filter(|e| e.group == group).map(|e| e.layer_key.clone()).collect();
        self.remove_layers(&keys)
    }
}

impl StrataAppState {
    /// The Delete key (`MainWindow.cpp:256-268`). On the sheet the key is the sheet's (`:1197-1203`). In the
    /// legend it takes the selected row off the list (`removeSelectedLayers`). On the map it is
    /// `deleteFeaturesOrSelectedReferenceLayers` (`MainWindowUndo.cpp:486-536`): while drawing the last point,
    /// else the selected shape of a survey layer, else a selected reference map or base map. A survey layer
    /// never leaves the list by a Delete pressed on the map.
    pub(crate) fn delete_selected(&mut self) {
        if self.active_tab != MainTab::Map {
            return;
        }
        let selected = self.right_inspector.selected_layer_key.as_deref();
        let row = selected.and_then(|label| self.left_layers.entries.iter().find(|e| e.label_ko == label)).map(|e| (e.layer_key.clone(), e.group));
        if self.home_map.legend_focus {
            let keys: Vec<String> = row.into_iter().map(|(key, _)| key).collect();
            self.remove_layers(&keys);
            return;
        }
        if self.capture_active() {
            self.sketch_undo_vertex();
            return;
        }
        if self.home_map.selected_shape.is_some() {
            let all = self.all_selected();
            self.delete_shapes(&all);
        } else if let Some((key, _)) = row.filter(|(_, group)| *group != LegendGroup::SurveyData) {
            self.remove_layers(&[key]);
        }
    }

    /// `deleteSelectedFeatures` (`MainWindowUndo.cpp:538-575`): every selected shape, grouped by layer, as one
    /// undo step; the file changes at the next 저장.
    fn delete_shapes(&mut self, shapes: &[(String, i64)]) {
        let mut keys: Vec<&String> = Vec::new();
        for (key, _) in shapes {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        let (mut done, mut count): (Vec<String>, usize) = (Vec::new(), 0);
        for key in keys {
            let fids: Vec<i64> = shapes.iter().filter(|(table, _)| table == key).map(|(_, fid)| *fid).collect();
            let table = self.survey_tables.iter_mut().find(|t| t.table == *key);
            if table.and_then(|t| t.layer.change_add_delete(&[], Vec::new(), &fids)).is_some() {
                done.push(key.clone());
                count += fids.len();
            }
        }
        if done.is_empty() {
            return;
        }
        self.redo_tables.clear();
        for key in &done {
            self.after_layer_edit(key);
        }
        self.undo_tables.push(if done.len() == 1 { UndoStep::Table(done.remove(0)) } else { UndoStep::Linked(done) });
        self.is_dirty = true;
        (self.home_map.selected_shape, self.home_map.also_selected) = (None, Vec::new());
        self.show_status_message(format!("도형 {count}개를 지웠습니다. Ctrl+Z로 복원할 수 있습니다."));
    }
}

#[path = "app_state_layer_order.rs"]
pub(crate) mod order;

#[cfg(test)]
#[path = "app_state_layer_remove_tests.rs"]
mod tests;
