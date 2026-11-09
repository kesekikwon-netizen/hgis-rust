//! 지역 팝업 뒷부분 — 검색 결과 선택 모달과 검색 실패 안내.
//!
//! `gui_ribbon_region` 에서 300줄을 지키려고 나눴다. 본문은 원문 그대로다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_ribbon_consts::{
    REGION_BOTTOM_HINT, REGION_LOT_HINT, REGION_LOT_PLACEHOLDER,
};

impl StrataAppState {
    /// 검색 결과 선택과 실패 안내 모달.
    pub(crate) fn render_region_locator_popup_b(&mut self, ctx: &egui::Context) {
        if !self.ribbon.app_bar.search_results_modal.is_empty() {
            let mut close_results = false;
            let mut confirm_idx: Option<usize> = None;
            let hits = self.ribbon.app_bar.search_results_modal.clone();
            egui::Window::new("위치 선택")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(460.0)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "검색 결과 {}건 — 이동할 위치를 선택하세요",
                        hits.len()
                    ));
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            for (idx, h) in hits.iter().enumerate() {
                                let line = if h.detail.is_empty() {
                                    h.title.clone()
                                } else {
                                    format!("{}  —  {}", h.title, h.detail)
                                };
                                let sel = self.ribbon.app_bar.selected_hit_idx == idx;
                                if ui.selectable_label(sel, line).clicked() {
                                    self.ribbon.app_bar.selected_hit_idx = idx;
                                }
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("확인").color(Color32::WHITE).strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .min_size(Vec2::new(72.0, 28.0));
                        if ui.add(ok_btn).clicked() {
                            confirm_idx = Some(self.ribbon.app_bar.selected_hit_idx);
                        }
                        if ui
                            .add(egui::Button::new("취소").min_size(Vec2::new(72.0, 28.0)))
                            .clicked()
                        {
                            close_results = true;
                        }
                    });
                });
            if let Some(idx) = confirm_idx {
                if let Some(hit) = hits.get(idx) {
                    self.zoom_to_location_hit(hit);
                }
                self.ribbon.app_bar.search_results_modal.clear();
            } else if close_results {
                self.ribbon.app_bar.search_results_modal.clear();
            }
        }

        // 검색 실패 안내 모달 (`MainWindow::onLocationFailed`, MainWindow.cpp:2801-2806)
        if let Some(err_msg) = self.ribbon.app_bar.search_error_modal.clone() {
            let mut close_err = false;
            egui::Window::new("위치 검색")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(380.0)
                .show(ctx, |ui| {
                    ui.label(&err_msg);
                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok_btn = egui::Button::new(
                            egui::RichText::new("확인").color(Color32::WHITE).strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .min_size(Vec2::new(72.0, 28.0));
                        if ui.add(ok_btn).clicked() {
                            close_err = true;
                        }
                    });
                });
            if close_err {
                self.ribbon.app_bar.search_error_modal = None;
            }
        }
    }

    /// 2단계 주소 팝업 (`#regionAddressPopup`) — 원문 `render_region_locator_popup` 뒷부분.
    pub(crate) fn paint_region_address_popup(
        &mut self,
        ctx: &egui::Context,
        screen_r: Rect,
        btn_min_x: f32,
        btn_max_y: f32,
    ) {
        // 2단계: 시·도 칩을 누른 뒤 열리는 주소 팝업 (`#regionAddressPopup`, KaRegionLocator.cpp:176-286)
        if self.ribbon.app_bar.address_popup_open {
            let popup_w = 570.0_f32;
            let popup_x = btn_min_x.clamp(8.0, (screen_r.right() - popup_w - 8.0).max(8.0));
            let popup_pos = Pos2::new(popup_x, btn_max_y + 2.0);
            let mut run_search = false;
            let mut close_popup = false;
            let canon_sido = strata_net::address_search::KoreaRegionCatalog::canonical_sido(
                &self.ribbon.app_bar.selected_sido,
            );
            let cities = strata_net::address_search::KoreaRegionCatalog::cities_of(&canon_sido);
            let dongs = strata_net::address_search::KoreaRegionCatalog::dongs_of(
                &canon_sido,
                &self.ribbon.app_bar.city_input,
            );

            egui::Area::new(egui::Id::new("ka_region_address_popup"))
                .order(egui::Order::Foreground)
                .fixed_pos(popup_pos)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(popup_w);
                            // Row 1: m_sidoLabel (#regionSidoTitle) + closeX (✕)
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&canon_sido)
                                        .size(13.0)
                                        .strong()
                                        .color(Color32::from_rgb(140, 54, 22)),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("✕")
                                            .on_hover_text("닫기 (Esc)")
                                            .clicked()
                                        {
                                            close_popup = true;
                                        }
                                    },
                                );
                            });
                            ui.add_space(6.0);
                            // Row 2: 시·군·구 ComboBox | 동·읍·면(법정동·읍·면·리) | 산 12-3 | 찾기 | 취소
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let city_label = if self.ribbon.app_bar.city_input.trim().is_empty()
                                {
                                    "시·군·구".to_owned()
                                } else {
                                    self.ribbon.app_bar.city_input.clone()
                                };
                                egui::ComboBox::from_id_salt("ka_region_city_combo")
                                    .width(120.0)
                                    .selected_text(city_label)
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                self.ribbon.app_bar.city_input.is_empty(),
                                                "시·군·구",
                                            )
                                            .clicked()
                                        {
                                            self.ribbon.app_bar.city_input.clear();
                                            self.ribbon.app_bar.dong_input.clear();
                                        }
                                        for c in &cities {
                                            let sel = self.ribbon.app_bar.city_input == *c;
                                            if ui.selectable_label(sel, c).clicked() {
                                                self.ribbon.app_bar.city_input = c.clone();
                                                self.ribbon.app_bar.dong_input.clear();
                                            }
                                        }
                                    });

                                let dong_resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.ribbon.app_bar.dong_input)
                                        .hint_text("법정동·읍·면·리")
                                        .desired_width(112.0),
                                );
                                if dong_resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    run_search = true;
                                }
                                if !dongs.is_empty() {
                                    egui::ComboBox::from_id_salt("ka_region_dong_combo")
                                        .width(24.0)
                                        .selected_text("")
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    self.ribbon.app_bar.dong_input.is_empty(),
                                                    "동·읍·면",
                                                )
                                                .clicked()
                                            {
                                                self.ribbon.app_bar.dong_input.clear();
                                            }
                                            for d in &dongs {
                                                let sel = self.ribbon.app_bar.dong_input == *d;
                                                if ui.selectable_label(sel, d).clicked() {
                                                    self.ribbon.app_bar.dong_input = d.clone();
                                                }
                                            }
                                        });
                                }

                                let lot_resp = ui
                                    .add(
                                        egui::TextEdit::singleline(
                                            &mut self.ribbon.app_bar.lot_input,
                                        )
                                        .hint_text(REGION_LOT_PLACEHOLDER)
                                        .desired_width(96.0),
                                    )
                                    .on_hover_text(REGION_LOT_HINT);
                                if lot_resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    run_search = true;
                                }

                                let find_btn = egui::Button::new(
                                    egui::RichText::new("찾기")
                                        .color(Color32::WHITE)
                                        .strong()
                                        .size(12.5),
                                )
                                .fill(Color32::from_rgb(181, 87, 58))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(56.0, 28.0));
                                if ui.add(find_btn).clicked() {
                                    run_search = true;
                                }

                                let cancel_btn = egui::Button::new(
                                    egui::RichText::new("취소")
                                        .color(Color32::from_rgb(35, 35, 32))
                                        .size(12.5),
                                )
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(54.0, 28.0));
                                if ui.add(cancel_btn).clicked() {
                                    close_popup = true;
                                }
                            });
                            ui.add_space(6.0);
                            // Row 3: 힌트 문구
                            ui.label(
                                egui::RichText::new(REGION_BOTTOM_HINT)
                                    .size(11.5)
                                    .color(Color32::from_rgb(94, 93, 89)),
                            );
                        });
                });

            if close_popup {
                self.ribbon.app_bar.address_popup_open = false;
            }
            if run_search {
                self.run_region_popup_search();
            }
        }
    }
}
