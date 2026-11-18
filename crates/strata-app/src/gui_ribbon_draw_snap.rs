//! Snap controls, target/cpp-ref/src/app/KaSnapSettingsWidget.cpp:13-33.
use super::gui_ribbon_context_card::{tool_button, CONTROL_HEIGHT};
use crate::app_state::StrataAppState;
use eframe::egui::{self, Stroke, Vec2};

impl StrataAppState {
    pub(super) fn paint_snap_settings(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut changed = ui
            .checkbox(&mut self.home_map.cad.snap_enabled, "자석")
            .on_hover_text("켜면 조사구역·유구·지적 선에 붙습니다. 위성 그림에는 붙지 않습니다")
            .changed();
        changed |= ui
            .checkbox(&mut self.home_map.cad.snap_topological, "공유 경계")
            .on_hover_text("켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮깁니다. QgsProject 위상 편집")
            .changed();
        ui.label("허용치");
        let spin = ui.add_sized([94.0, CONTROL_HEIGHT], tool_button("")).on_hover_text("자석이 붙는 화면 거리");
        let tolerance = &mut self.home_map.cad.snap_tolerance_px;
        if spin.clicked() {
            if let Some(pos) = spin.interact_pointer_pos() {
                *tolerance = (*tolerance + if pos.y < spin.rect.center().y { 1.0 } else { -1.0 }).clamp(4.0, 64.0);
                changed = true;
            }
        }
        if spin.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                *tolerance = (*tolerance + f64::from(scroll.signum())).clamp(4.0, 64.0);
                changed = true;
            }
        }
        let ink = ui.visuals().text_color();
        ui.painter()
            .text(spin.rect.left_center() + Vec2::new(10.0, 0.0), egui::Align2::LEFT_CENTER, format!("{tolerance:.0} px"), egui::FontId::proportional(13.0), ink);
        for (y, down) in [(-4.0, false), (4.0, true)] {
            let c = spin.rect.right_center() + Vec2::new(-12.0, y);
            let h = if down { 2.0 } else { -2.0 };
            ui.painter().add(egui::Shape::line(vec![c + Vec2::new(-3.0, -h), c + Vec2::new(0.0, h), c + Vec2::new(3.0, -h)], Stroke::new(1.2, ink)));
        }
        let current_only = self.home_map.cad.snap_target_current_only;
        let text = if current_only { "현재 레이어" } else { "모든 조사 레이어" };
        let combo = ui.add(tool_button(text)).on_hover_text("조사 도형과 지적 선에 붙습니다. 위성 그림은 제외");
        let popup_id = ui.id().with("ka_draw_combo_popup");
        if combo.clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup_id));
        }
        crate::gui_shell::native_popup_below_widget(ui, popup_id, &combo, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(combo.rect.width());
            for (value, label) in [(false, "모든 조사 레이어"), (true, "현재 레이어")] {
                if ui.selectable_label(current_only == value, label).clicked() && current_only != value {
                    self.home_map.cad.snap_target_current_only = value;
                    changed = true;
                }
            }
        });
        if changed {
            self.show_status_message(if self.home_map.cad.snap_enabled {
                "자석 켜짐 — 조사 도형·지적 선에 붙습니다. 위성 그림은 제외"
            } else {
                "자석 꺼짐"
            });
        }
        let assist = ui
            .add_sized([CONTROL_HEIGHT, CONTROL_HEIGHT], tool_button(""))
            .on_hover_text("자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기");
        let c = assist.rect.center();
        ui.painter().add(egui::Shape::line(vec![c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.0), c + Vec2::new(4.0, -2.0)], Stroke::new(1.5, ink)));
        if assist.clicked() {
            self.ribbon.snap_assist_menu_open = !self.ribbon.snap_assist_menu_open;
        }
        self.paint_snap_assist_popup(ctx, ui, assist.rect);
    }
}
