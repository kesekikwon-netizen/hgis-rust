//! 앱바 — [📍 지역] 단추 + [🔍 주소·지번 찾기] 입력칸 (C++ `KaAppBar.cpp:25-58`).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_ribbon_top::RibbonFrame;

impl StrataAppState {
    /// 지역 단추와 검색칸.
    pub(crate) fn paint_ribbon_app_bar(
        &mut self,
        ui: &mut egui::Ui,
        f: &RibbonFrame,
    ) {
        let ribbon_rect = f.ribbon_rect;
        let h_scale = f.h_scale;
        // KaAppBar.cpp:25-58 — [📍 지역] 버튼 + [🔍 주소·지번 찾기] 입력칸
        let right_left = (ribbon_rect.left() + 1334.0 * h_scale).max(ribbon_rect.right() - 264.0);
        let right_rect = Rect::from_min_max(
            Pos2::new(right_left, ribbon_rect.top() + 1.0),
            Pos2::new(ribbon_rect.right() - 8.0, ribbon_rect.bottom() - 1.0),
        );
        let mut right_ui = ui.new_child(egui::UiBuilder::new().max_rect(right_rect));
        right_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let search_placeholder = if self.display_options.paper_look {
                "주소·지번 찾기"
            } else {
                "도로명·지번 (예: 하회종가길 40)  Ctrl+F"
            };
            let resp = ui
                .add_sized(
                    [156.0, 32.0],
                    egui::TextEdit::singleline(&mut self.ribbon.app_bar.query)
                        .hint_text(format!("     {search_placeholder}"))
                        .vertical_align(egui::Align::Center),
                )
                .on_hover_text("도로명·지번으로 찾습니다 (예: 하회종가길 40) · Ctrl+F");
            // 검색칸 안쪽 왼쪽 돋보기 아이콘(16px, KaAppBar.cpp:41)
            {
                let sp = ui.painter();
                let c = Pos2::new(resp.rect.left() + 12.0, resp.rect.center().y - 1.0);
                let ink = Color32::from_rgb(94, 93, 89);
                sp.circle_stroke(c, 4.8, Stroke::new(1.5, ink));
                sp.line_segment(
                    [
                        Pos2::new(c.x + 3.4, c.y + 3.4),
                        Pos2::new(c.x + 7.2, c.y + 7.2),
                    ],
                    Stroke::new(1.6, ink),
                );
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.run_app_bar_search();
            }
            let (reg_rect, reg_resp) =
                ui.allocate_exact_size(Vec2::new(76.0, 36.0), egui::Sense::click());
            self.ribbon.app_bar.region_button_rect = Some([
                reg_rect.min.x,
                reg_rect.min.y,
                reg_rect.max.x,
                reg_rect.max.y,
            ]);
            {
                let rp = ui.painter_at(reg_rect);
                let active_or_hover = reg_resp.hovered()
                    || self.ribbon.app_bar.region_popup_open
                    || self.ribbon.app_bar.address_popup_open;
                let bg = if active_or_hover {
                    Color32::from_rgb(240, 238, 230)
                } else {
                    Color32::WHITE
                };
                let border_col = if active_or_hover {
                    Color32::from_rgb(181, 87, 58)
                } else {
                    Color32::from_rgb(194, 192, 182)
                };
                rp.rect_filled(reg_rect, 8.0, bg);
                rp.rect_stroke(
                    reg_rect,
                    8.0,
                    Stroke::new(1.0, border_col),
                    egui::StrokeKind::Inside,
                );
                // 지도 핀 아이콘 (KaIcons "gps", 16px, KaAppBar.cpp:32)
                let pin_c = Pos2::new(reg_rect.left() + 19.0, reg_rect.center().y - 2.0);
                let pin_ink = Color32::from_rgb(94, 93, 89);
                rp.circle_stroke(pin_c, 4.5, Stroke::new(1.5, pin_ink));
                rp.circle_filled(pin_c, 1.6, pin_ink);
                rp.line_segment(
                    [
                        Pos2::new(pin_c.x - 3.2, pin_c.y + 3.2),
                        Pos2::new(pin_c.x, pin_c.y + 8.0),
                    ],
                    Stroke::new(1.5, pin_ink),
                );
                rp.line_segment(
                    [
                        Pos2::new(pin_c.x + 3.2, pin_c.y + 3.2),
                        Pos2::new(pin_c.x, pin_c.y + 8.0),
                    ],
                    Stroke::new(1.5, pin_ink),
                );
                rp.text(
                    Pos2::new(reg_rect.left() + 31.0, reg_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "지역",
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(20, 20, 19),
                );
            }
            if reg_resp
                .on_hover_text("시·도를 고른 뒤 시·군·동과 지번으로 찾아갑니다")
                .clicked()
            {
                self.ribbon.app_bar.region_popup_open = !self.ribbon.app_bar.region_popup_open;
                if self.ribbon.app_bar.region_popup_open {
                    self.ribbon.app_bar.address_popup_open = false;
                }
            }
        });
    }
}
