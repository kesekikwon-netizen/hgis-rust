//! Named layer sets, target/cpp-ref/src/app/KaDrawingLayerSets.cpp:19-60.
use crate::app_state::StrataAppState;
use std::collections::BTreeMap;

pub(super) type Catalog = BTreeMap<String, BTreeMap<String, bool>>;

#[derive(Debug, Clone, Default)]
pub(crate) struct LayerSetState {
    catalog: Catalog,
    question: Option<Question>,
    remove_expanded: bool,
}

#[derive(Debug, Clone)]
struct Question {
    request: Request,
    focus: bool,
}

#[derive(Debug, Clone)]
enum Request {
    Name(String),
    Replace(String),
    Remove(String),
}

#[path = "layer_sets_io.rs"]
pub(crate) mod persistence;
#[path = "layer_sets_ui.rs"]
mod ui;

impl StrataAppState {
    pub(crate) fn store_layer_set(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        let sets = &mut self.layer_sets.catalog;
        // target/cpp-ref/src/app/KaDrawingLayerSets.cpp:35-36; read the state painted by gui_layer_panel_rows.rs:121.
        let checks = self.left_layers.entries.iter().map(|row| (row.layer_key.clone(), self.home_map.real_survey.is_layer_visible(&row.label_ko))).collect();
        sets.insert(name.to_owned(), checks);
        self.is_dirty = true;
        true
    }
    pub(crate) fn apply_layer_set(&mut self, name: &str) -> bool {
        let Some(checks) = self.layer_sets.catalog.get(name) else { return false };
        for row in &mut self.left_layers.entries {
            if let Some(checked) = checks.get(&row.layer_key) {
                row.visible = *checked;
                self.home_map.real_survey.layer_visible.insert(row.label_ko.clone(), *checked);
            }
        }
        self.is_dirty = true;
        true
    }
    pub(crate) fn remove_layer_set(&mut self, name: &str) -> bool {
        let sets = &mut self.layer_sets.catalog;
        if sets.remove(name).is_none() {
            return false;
        }
        self.is_dirty = true;
        true
    }
    pub(crate) fn layer_set_names(&self) -> Vec<String> {
        let mut names = self.layer_sets.catalog.keys().cloned().collect::<Vec<_>>();
        // target/cpp-ref/src/app/KaDrawingLayerSets.cpp:19-23. O(total name bytes) for keys, then O(n log n) string comparisons while the menu is open.
        names.sort_by_cached_key(|name| name.to_lowercase());
        names
    }
    pub(crate) fn layer_set_question_open(&self) -> bool {
        self.layer_sets.question.is_some()
    }
}

#[cfg(test)]
#[path = "layer_sets_io_tests.rs"]
mod persistence_tests;
#[cfg(test)]
#[path = "layer_sets_tests.rs"]
mod tests;
