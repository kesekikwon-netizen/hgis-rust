//! 보조툴바 — 그리기·정합·측정 등 하위 도구 띠.

use eframe::egui::{self, Color32, Stroke};

use crate::app_state::StrataAppState;
use crate::ribbon::SubToolsMode;

use super::gui_ribbon_draw::draw_buffer_pill;

use super::gui_ribbon_consts::{ALIGN_SUBTOOLBAR_CAPTION, ALIGN_SUBTOOLBAR_ITEMS};

impl StrataAppState {
    /// 보조툴바 한 줄.
    pub(crate) fn paint_ribbon_sub_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.draw_row_open() {
            self.paint_draw_sub_toolbar(ctx, ui);
            return;
        }
        if self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode != SubToolsMode::None {
            egui::Frame::NONE
                .fill(Color32::from_rgb(245, 243, 236))
                .stroke(Stroke::new(1.0, Color32::from_rgb(218, 215, 205)))
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        match self.ribbon.sub_mode {
                            SubToolsMode::Buffer => {
                                ui.label(egui::RichText::new("주변유적경계 ›").size(12.5).strong());
                                // MainWindowRibbon.cpp:798-801 runSiteBuffer500·1000 — 열어 둔 조사 면으로 경계를 (다시) 그린다.
                                if draw_buffer_pill(ui, "버퍼", true, "500m", 78.0) {
                                    self.run_site_buffer(500);
                                }
                                if draw_buffer_pill(ui, "버퍼", true, "1000m", 84.0) {
                                    self.run_site_buffer(1000);
                                }
                                if draw_buffer_pill(ui, "수치지형", false, "엑셀 내보내기", 118.0) {
                                    if let Some(out) = self.home_map.real_survey.export_nearby_heritage_csv() {
                                        self.home_map.draw_guide_message = Some(format!("주변유적 목록을 내보냈습니다: {}", out.display()));
                                    }
                                }
                                if draw_buffer_pill(ui, "", false, "닫기", 48.0) { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Align => {
                                // MainWindowAlign.cpp:716-797 showSubToolsAlign — C++ 항목은
                                // 이동(:724) · 점 지우기(:726) · 되돌리기(:730) ·
                                // 맞추기 저장(:745) · 닫기(:793) 다섯 개뿐이다.
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(ALIGN_SUBTOOLBAR_CAPTION)
                                            .size(12.5)
                                            .strong(),
                                    )
                                    .selectable(false),
                                );
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[0]).clicked() {
                                    // MainWindowAlign.cpp:627-703 applyAlignMove
                                    match self.apply_align_move() {
                                        Ok(()) => {}
                                        Err(message) => {
                                            // :632 KaUserError::warn({「이동」, 「도면을 옮기지 못했습니다」, err})
                                            self.home_map.draw_guide_message =
                                                Some(format!("도면을 옮기지 못했습니다 — {message}"));
                                        }
                                    }
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[1]).clicked() {
                                    // MainWindowAlign.cpp:726-729 — 지운 뒤 상태줄에 statusText()
                                    self.align.remove_last_pair();
                                    self.status_bar_hint = self.align.status_text();
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[2]).clicked() {
                                    // MainWindowAlign.cpp:730-744 — 되돌린 결과를 상태줄(성공)이나
                                    // 알림(실패)으로 보여 준다. 문구는 KaAlignMapToolEdit.cpp:132-161.
                                    let (restored, message) = self.restore_align_originals();
                                    self.status_bar_hint = message.clone();
                                    if !restored {
                                        self.home_map.draw_guide_message =
                                            Some(format!("맞추기 — {message}"));
                                    }
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[3]).clicked() {
                                    // MainWindowAlign.cpp:745-792 맞추기 저장
                                    let saved = self.save_align_result();
                                    self.status_bar_hint = saved.1;
                                    self.home_map.draw_guide_message = Some(saved.0);
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[4]).clicked() {
                                    // MainWindowAlign.cpp:793-794 → hideSubTools(:769-777)
                                    // → :771-772 stopAlignSession
                                    self.hide_sub_tools();
                                }
                            }
                            SubToolsMode::Draw | SubToolsMode::None => {}
                        }
                    });
                });
        }
    }
}
