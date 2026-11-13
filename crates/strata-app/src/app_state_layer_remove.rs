//! 「레이어 삭제」와 Ctrl+Z 복원 — C++ `removeLayersFromTree`(MainWindowUndo.cpp:158-288)·되돌리기 `LayersRemoved`(:369-389).
//! 목록과 지도에서만 뺀다: 원본 파일은 그대로, 그 레이어(저장 안 한 편집 포함)는 되돌리기 단계 안에 둔다 — 그동안 그리지도
//! 저장하지도 검수에 세지도 않는다. 되돌리면 같은 자리로. C++ 에는 레이어 삭제의 「다시 실행」이 없다.

use strata_storage::survey_gpkg_read::SurveyFileTable;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;

/// 되돌리기 한 단계 — 그 표의 편집 하나, 또는 레이어 삭제 하나(KaUndoAction::LayersRemoved).
#[derive(Debug, Clone)]
pub(crate) enum UndoStep {
    Table(String),
    LayersRemoved(Vec<RemovedLayer>),
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

#[cfg(test)]
#[path = "app_state_layer_remove_tests.rs"]
mod tests;
