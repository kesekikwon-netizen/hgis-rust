//! 왼쪽 레이어 목록 우클릭 메뉴 — C++ `MainWindow::showLayerTreeContextMenu`(MainWindowContextMenus.cpp:176-448). 줄은
//! layer_menu_model(가지·줄·구분선·하위 메뉴·체크·흐림), 그리기는 menu_popup. 줄을 누르면 그 레이어를 현재로 한 뒤 한다(:240-247).
//! 창(모달)이 열려 있으면 열지 않는다.

use eframe::egui;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::layer_menu_model::{blank_menu_entries, layer_menu_entries, LayerMenuKind};
use crate::menu_popup::{show_menu, MenuPick};

impl StrataAppState {
    pub(crate) fn render_layer_context_menu(&mut self, ctx: &egui::Context) {
        let shown_id = egui::Id::new("ka_layer_menu_shown_at");
        if !self.layer_context_menu_open || self.modal_window_open() {
            self.layer_context_menu_open = false;
            ctx.data_mut(|d| d.remove::<[f32; 2]>(shown_id));
            return;
        }
        let fresh = ctx.data(|d| d.get_temp::<[f32; 2]>(shown_id)) != Some(self.layer_context_menu_pos);
        if fresh {
            self.home_map.layer_menu_path.clear();
            ctx.data_mut(|d| d.insert_temp(shown_id, self.layer_context_menu_pos));
        }
        let key = self.layer_context_menu_key.clone();
        let entries = match &key {
            Some(k) => match self.layer_menu_input(k) {
                Some(input) => layer_menu_entries(&input),
                None => {
                    self.layer_context_menu_open = false;
                    return;
                }
            },
            None => {
                let group = self.home_map.layer_menu_group;
                let removable = group.is_some_and(|g| self.left_layers.entries.iter().any(|e| e.group == g));
                blank_menu_entries(removable, group.is_some())
            }
        };
        let pos = egui::Pos2::new(self.layer_context_menu_pos[0], self.layer_context_menu_pos[1]);
        let mut path = std::mem::take(&mut self.home_map.layer_menu_path);
        let (pick, rects) = show_menu(ctx, egui::Id::new("ka_layer_context_menu"), &entries, pos, &mut path, 0);
        self.home_map.layer_menu_path = path;
        let outside = !fresh
            && ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rects.iter().any(|r| r.contains(p))));
        if outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.layer_context_menu_open = false;
        }
        if let Some(pick) = pick {
            self.layer_context_menu_open = false;
            self.run_layer_menu_action(key.as_deref(), &pick);
        }
    }

    /// 고른 줄을 한다 — 먼저 그 레이어를 현재 레이어로(:240-247).
    pub(crate) fn run_layer_menu_action(&mut self, key: Option<&str>, pick: &MenuPick) {
        let entry = key.and_then(|k| self.left_layers.entries.iter().find(|e| e.layer_key == k).cloned());
        if let Some(e) = &entry {
            self.right_inspector.selected_layer_key = Some(e.label_ko.clone());
        }
        let label = entry.as_ref().map_or_else(String::new, |e| e.label_ko.clone());
        match pick.id {
            // editCurrentLayerStyle — 지금 있는 선 색 창(C++ 창은 CM2a-3).
            "layer.style" => self.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::LineColor),
            // beginEdit(vector) — 그 레이어를 편집 레이어로.
            "layer.draw" => {
                self.active_tab = crate::app_state::MainTab::Map;
                self.start_edit_from_legend(&label);
            }
            // showLayerAreaSummary — C++ 은 알림 창이다. 창이 생기기 전까지 상태줄에 이어 쓴다.
            "layer.area" => {
                let trench = entry.as_ref().is_some_and(|e| crate::layer_menu_model::menu_kind(&e.layer_key, e.group != LegendGroup::Basemap, e.group != LegendGroup::SurveyData) == LayerMenuKind::Trench);
                if let Some(text) = self.layer_area_summary(&label, trench) {
                    self.show_status_message(text.replace('\n', " — "));
                }
            }
            "layer.zoom" => self.zoom_to_current_legend_layer(),
            // zoomMapToFullMax(MainWindowSession.cpp:1800).
            "layer.fullExtent" => self.home_map.zoom_to_full_max(self.work_crs),
            "layer.labelsVisible" => {
                // setVisible → refreshViews(프로젝트 고침).
                self.home_map.real_survey.toggle_label_visible(&label);
                self.is_dirty = true;
            }
            "layer.rename" => self.open_rename_layer(key.unwrap_or_default()),
            "layer.labelFieldValue" => {
                self.layer_label_field(key.unwrap_or_default(), &pick.value);
            }
            "layer.labelArea" => {
                self.layer_label_area(key.unwrap_or_default());
            }
            "layer.labelSizeValue" => {
                self.layer_label_size(key.unwrap_or_default(), pick.value.parse().unwrap_or(0.0));
            }
            "layer.opacityValue" => {
                self.layer_set_transparency(key.unwrap_or_default(), pick.value.parse().unwrap_or(0));
            }
            "layer.bottom" => {
                self.layer_to_group_bottom(key.unwrap_or_default());
            }
            // 레이어 삭제는 CM2a-1d. 깨진 한글 바로잡기는 .shp 레이어에만 보인다(조사 GPKG 에는 없다).
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "gui_layer_panel_context_menu_tests.rs"]
mod tests;
