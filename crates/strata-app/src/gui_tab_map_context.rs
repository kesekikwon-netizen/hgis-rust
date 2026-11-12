//! 지도 캔버스 우클릭 메뉴 — 항목은 `map_context_menu_model.rs`(C++ `populateMapContextMenu`,
//! `MainWindowContextMenus.cpp:112-161`), 여기서는 그리기와 고른 항목의 동작만 한다.
//!
//! 여는 곳은 `gui_tab_map_canvas.rs`(그리기·측거 도구가 아닐 때 우클릭, `onMapContextMenu` :163-170).
//! 우클릭한 자리는 메뉴가 처음 뜨는 프레임의 마우스 지도 좌표(상태줄과 같은 변환)로 잡는다.
//! 선택 도구는 이미 고른 도형이 있으면 자기 메뉴를 띄우므로(`KaFeatureSelectTool.cpp:276-288`,
//! 지시 CM3) 이 메뉴를 열지 않는다. 클릭 자리의 도형 집기는 아직 없다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::map_context_menu_model::{map_menu_entries, MapMenuAction, MapMenuEntry, MapMenuInput};

const ROW_W: f32 = 216.0;
const ROW_H: f32 = 22.0;

/// 메뉴 한 줄(왼쪽 맞춤). 흐린 줄은 이유를, 켜진 줄은 도움말을 툴팁으로(`setToolTipsVisible(true)`).
fn menu_row(ui: &mut egui::Ui, text: &str, enabled: bool, tooltip: &str, arrow: bool) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(text).size(12.0))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(ROW_W, ROW_H));
    let mut resp = ui.add_enabled(enabled, button);
    if arrow {
        let at = Pos2::new(resp.rect.right() - 10.0, resp.rect.center().y);
        let ink = ui.visuals().text_color();
        ui.painter().text(at, egui::Align2::CENTER_CENTER, "⏵", egui::FontId::proportional(11.0), ink);
    }
    if !tooltip.is_empty() {
        resp = if enabled { resp.on_hover_text(tooltip) } else { resp.on_disabled_hover_text(tooltip) };
    }
    resp
}

fn action_row(ui: &mut egui::Ui, a: &MapMenuAction, picked: &mut Option<&'static str>) -> egui::Response {
    let resp = menu_row(ui, a.text, a.enabled, &a.tooltip, false);
    if a.enabled && resp.clicked() {
        *picked = Some(a.id);
    }
    resp
}

impl StrataAppState {
    /// 범례의 현재 레이어(`m_layerTree->currentLayer()`).
    fn current_legend_layer(&self) -> Option<&crate::left_layer_panel::LayerTreeEntry> {
        let name = self.right_inspector.selected_layer_key.as_deref()?;
        self.left_layers.entries.iter().find(|e| e.label_ko == name)
    }

    /// 「선택한 레이어로 이동」 — 조사 표는 도형 범위로, 배경지도·지적·참조 그림은 1:50,000 로.
    fn zoom_to_current_legend_layer(&mut self) {
        let Some(entry) = self.current_legend_layer().cloned() else { return };
        let is_raster = matches!(entry.group, LegendGroup::Basemap | LegendGroup::Cadastral | LegendGroup::ReferenceMap);
        let ext = self
            .survey_tables
            .iter()
            .find(|t| t.table == entry.layer_key)
            .and_then(|t| crate::survey_paint_layers::table_extent_in(t, self.work_crs));
        self.home_map.zoom_to_layer_max(ext, is_raster, self.work_crs);
    }

    fn run_map_menu_action(&mut self, ctx: &egui::Context, id: &str, coords: &str) {
        match id {
            "map.drawArea" => self.start_edit_survey_area(),
            "map.drawFeature" => self.start_edit_feature_poly(),
            "map.drawLine" => self.start_edit_feature_line(),
            "map.drawSection" => self.start_edit_section_line(),
            "map.previous" => {
                if let Some(r) = self.home_map.extent_history.previous() {
                    self.home_map.show_history_extent(r);
                }
            }
            "map.next" => {
                if let Some(r) = self.home_map.extent_history.next() {
                    self.home_map.show_history_extent(r);
                }
            }
            "map.layerZoom" => self.zoom_to_current_legend_layer(),
            "map.full" => {
                // MainWindowSession.cpp:1800-1807 zoomMapToFullMax.
                self.home_map.zoom_to_full_max(self.work_crs);
                self.home_map.draw_guide_message = Some("한국 전체 범위".to_owned());
            }
            "map.refresh" => {
                // m_canvas->refresh() — 다시 그린다. 화면 타일은 디스크 캐시에서 다시 읽는다(새로 받지 않음).
                self.basemap_textures = Default::default();
                self.cadastral_cells = Default::default();
                ctx.request_repaint();
            }
            "map.copyCoordinates" => ctx.copy_text(coords.to_owned()),
            _ => {}
        }
    }

    pub(crate) fn render_map_context_menu(&mut self, ctx: &egui::Context) {
        let shown_id = egui::Id::new("ka_map_menu_shown_at");
        if !self.map_context_menu_open {
            ctx.data_mut(|d| d.remove::<[f32; 2]>(shown_id));
            self.map_menu_sub_open = false;
            return;
        }
        let fresh = ctx.data(|d| d.get_temp::<[f32; 2]>(shown_id)) != Some(self.map_context_menu_pos);
        if fresh {
            if self.active_tool == MapToolKind::Select && self.home_map.selected_feature_idx.is_some() {
                self.map_context_menu_open = false;
                return;
            }
            self.map_context_menu_world = self.home_map.cursor_world_xy;
            self.map_menu_sub_open = false;
            ctx.data_mut(|d| d.insert_temp(shown_id, self.map_context_menu_pos));
        }
        let input = MapMenuInput {
            survey_open: self.survey_path.is_some(),
            busy: false,
            current_layer: self.current_legend_layer().is_some(),
            can_previous: self.home_map.extent_history.can_previous(),
            can_next: self.home_map.extent_history.can_next(),
            click_xy: self.map_context_menu_world,
            crs_auth: self.work_crs.auth_id(),
        };
        let entries = map_menu_entries(&input);
        let coords = crate::map_context_menu_model::coordinate_text(input.click_xy, input.crs_auth);
        let pos = Pos2::new(self.map_context_menu_pos[0], self.map_context_menu_pos[1]);
        let mut picked: Option<&'static str> = None;
        let mut sub_row: Option<Rect> = None;
        let mut sub_items: &[MapMenuAction] = &[];
        let mut sub_open = self.map_menu_sub_open;
        // 메뉴 본체·하위 메뉴 모두 줄 폭(ROW_W) 그대로의 팝업 틀 — 화살표가 메뉴 오른쪽 끝에,
        // 하위 메뉴는 메뉴 바로 옆에 붙는다(QMenu 처럼).
        let main = egui::Area::new(egui::Id::new("ka_map_context_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(ROW_W);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for e in &entries {
                        match e {
                            MapMenuEntry::Separator => {
                                ui.separator();
                            }
                            MapMenuEntry::Action(a) => {
                                if action_row(ui, a, &mut picked).hovered() {
                                    sub_open = false;
                                }
                            }
                            MapMenuEntry::Submenu { title, items, .. } => {
                                let r = menu_row(ui, title, true, "", true);
                                if r.hovered() || r.clicked() {
                                    sub_open = true;
                                }
                                sub_row = Some(r.rect);
                                sub_items = items;
                            }
                        }
                    }
                });
            });
        let main_rect = Some(main.response.rect);
        let mut sub_rect = None;
        if let (true, Some(row)) = (sub_open, sub_row) {
            let frame_pad = main.response.rect.right() - row.right();
            let area = egui::Area::new(egui::Id::new("ka_map_context_submenu"))
                .order(egui::Order::Foreground)
                .fixed_pos(Pos2::new(main.response.rect.right() - 2.0, row.top() - frame_pad))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_width(ROW_W);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for a in sub_items {
                            action_row(ui, a, &mut picked);
                        }
                    });
                });
            sub_rect = Some(area.response.rect);
        }
        self.map_menu_sub_open = sub_open;
        let mut close = false;
        if let Some(id) = picked {
            self.run_map_menu_action(ctx, id, &coords);
            close = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if !fresh {
            let click = ctx.input(|i| if i.pointer.any_click() { i.pointer.interact_pos() } else { None });
            let inside = |p: Pos2| main_rect.is_some_and(|r| r.contains(p)) || sub_rect.is_some_and(|r| r.contains(p));
            if click.is_some_and(|p| !inside(p)) {
                close = true;
            }
        }
        if close {
            self.map_context_menu_open = false;
            self.map_menu_sub_open = false;
            ctx.data_mut(|d| d.remove::<[f32; 2]>(shown_id));
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_map_context_tests.rs"]
mod tests;
