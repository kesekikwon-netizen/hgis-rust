//! 지도 캔버스·레이어 트리 우클릭 메뉴.

use eframe::egui::{self, Color32, Stroke};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
/// `KaShellGridControls.cpp:41-193` — 좌표격자 토글 + 2줄 `gridDetail`
/// (미터·경위도, 간격, 회전 / 굵기, 선 모양, 색 견본 셋과 직접 고르기)
/// MainWindowContextMenus.cpp:112-161 `populateMapContextMenu` — 지도 캔버스
/// 우클릭 팝업. 도형 위에서 열면 「이 도형 기록 입력」이 먼저 나온다.
impl StrataAppState {
    pub(crate) fn render_map_context_menu(&mut self, ctx: &egui::Context) {
    if !self.map_context_menu_open {
        return;
    }
    let pos = egui::Pos2::new(self.map_context_menu_pos[0], self.map_context_menu_pos[1]);
    let mut close = false;
    let mut picked: Option<&'static str> = None;
    egui::Window::new("ka_map_context_menu")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            ui.set_min_width(228.0);
            let has_hit = self.home_map.selected_feature_idx.is_some();
            if has_hit {
                if ui
                    .add_sized(
                        [216.0, 24.0],
                        egui::Button::new(egui::RichText::new("이 도형 기록 입력").size(12.0))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    picked = Some("attr");
                }
                ui.separator();
            }
            ui.label(
                egui::RichText::new("조사 도형 그리기")
                    .size(10.5)
                    .strong()
                    .color(Color32::from_rgb(120, 118, 110)),
            );
            for item in ["조사구역", "유구 면", "유구 선", "단면선 그리기"] {
                if ui
                    .add_sized(
                        [216.0, 22.0],
                        egui::Button::new(egui::RichText::new(item).size(12.0))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    picked = Some(match item {
                        "조사구역" => "draw_area",
                        "유구 면" => "draw_poly",
                        "유구 선" => "draw_line",
                        _ => "draw_section",
                    });
                }
            }
            ui.separator();
            for item in ["이전 지도 범위", "다음 지도 범위"] {
                if ui
                    .add_sized(
                        [216.0, 22.0],
                        egui::Button::new(egui::RichText::new(item).size(12.0))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    picked = Some(if item.starts_with("이전") {
                        "zoom_prev"
                    } else {
                        "zoom_next"
                    });
                }
            }
            for (label, key) in [
                ("선택한 레이어로 이동", "layer_zoom"),
                ("전체 지도 보기", "full"),
                ("지도 새로고침", "refresh"),
            ] {
                if ui
                    .add_sized(
                        [216.0, 22.0],
                        egui::Button::new(egui::RichText::new(label).size(12.0))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    picked = Some(key);
                }
            }
            ui.separator();
            let coord_txt = format!(
                "{:.3}, {:.3} ({})",
                self.map_context_menu_world[1],
                self.map_context_menu_world[0],
                self.work_crs.auth_id()
            );
            if ui
                .add_sized(
                    [216.0, 22.0],
                    egui::Button::new(egui::RichText::new("이 위치 좌표 복사").size(12.0))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE),
                )
                .on_hover_text(&coord_txt)
                .clicked()
            {
                picked = Some("copy_coords");
            }
        });
    let coord_txt = format!(
        "{:.3}, {:.3} ({})",
        self.map_context_menu_world[1],
        self.map_context_menu_world[0],
        self.work_crs.auth_id()
    );
    if let Some(key) = picked {
        match key {
            "attr" => {
                self.right_inspector.active_card =
                    crate::right_inspector::InspectorCardTab::LayerStyleCard;
                self.home_map.draw_guide_message =
                    Some("이 도형 기록을 오른쪽 카드에서 고칩니다.".to_owned());
            }
            "draw_area" => {
                self.active_tool = MapToolKind::DrawPolygon;
            }
            "draw_poly" => {
                self.active_tool = MapToolKind::DrawPolygon;
            }
            "draw_line" => {
                self.active_tool = MapToolKind::DrawLine;
            }
            "draw_section" => {
                self.active_tool = MapToolKind::DrawLine;
            }
            "zoom_prev" => {
                self.home_map.draw_guide_message =
                    Some("이전에 보던 지도 범위로 돌아갈 이력이 없습니다.".to_owned());
            }
            "zoom_next" => {
                self.home_map.draw_guide_message =
                    Some("이전 범위로 돌아간 뒤에 쓸 수 있습니다.".to_owned());
            }
            "layer_zoom" => {
                if self.right_inspector.selected_layer_key.is_some() {
                    self.home_map.fit_to_survey();
                } else {
                    self.home_map.draw_guide_message =
                        Some("왼쪽 목록에서 레이어를 선택하세요.".to_owned());
                }
            }
            "full" => {
                self.home_map.fit_to_survey();
            }
            "refresh" => {
                self.home_map.draw_guide_message = Some("지도를 새로 그렸습니다.".to_owned());
            }
            "copy_coords" => {
                ctx.copy_text(coord_txt.clone());
                self.home_map.draw_guide_message =
                    Some(format!("좌표를 복사했습니다: {coord_txt}"));
            }
            _ => {}
        }
        close = true;
    }
    // 바깥을 누르면 닫는다
    if close || ctx.input(|i| i.pointer.any_click() && !i.pointer.any_down()) {
        self.map_context_menu_open = false;
    }
    }
}

