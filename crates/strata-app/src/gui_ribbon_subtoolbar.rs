//! 보조툴바 — 그리기·정합·측정 등 하위 도구 띠.

use eframe::egui;

use crate::app_state::StrataAppState;
use crate::ribbon::SubToolsMode;

use super::gui_ribbon_context_card::{context_card, tool_button};

use super::gui_ribbon_consts::{ALIGN_SUBTOOLBAR_CAPTION, ALIGN_SUBTOOLBAR_ITEMS};

impl StrataAppState {
    /// 보조툴바 한 줄.
    pub(crate) fn paint_ribbon_sub_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(origin) = self.ribbon.sub_toolbar_origin else {
            return;
        };
        let width = match self.ribbon.sub_mode {
            SubToolsMode::Draw => ui.available_width(),
            SubToolsMode::Buffer => 490.0,
            SubToolsMode::Align => 570.0,
            SubToolsMode::None => return,
        };
        if self.ribbon.sub_toolbar_visible {
            ui.push_id("active-context-tools", |ui| {
                context_card(ui, origin, width, |ui| {
                    match self.ribbon.sub_mode {
                        SubToolsMode::Buffer => {
                            ui.label(egui::RichText::new("주변유적경계 ›").size(12.5).strong());
                            // MainWindowRibbon.cpp:798-801 runSiteBuffer500·1000 — 열어 둔 조사 면으로 경계를 (다시) 그린다.
                            if ui.add(tool_button("500m")).clicked() {
                                self.run_site_buffer(500);
                            }
                            if ui.add(tool_button("1000m")).clicked() {
                                self.run_site_buffer(1000);
                            }
                            // MainWindowRibbon.cpp:802-807 exportNearbyHeritageExcel(버퍼 안 유적·조사구역 기준 방향·거리 .xlsx) —
                            // F1: 경주 기본점으로 거리를 재던 CSV 를 없애고, 옮길 때까지 흐리게 둔다.
                            ui.add_enabled(false, tool_button("엑셀 내보내기")).on_disabled_hover_text("아직 Rust 판에서 연결하지 않았습니다 (지시 W-H1)");
                            if ui.add(tool_button("닫기")).clicked() {
                                self.hide_sub_tools();
                            }
                        }
                        SubToolsMode::Align => {
                            // MainWindowAlign.cpp:716-797 showSubToolsAlign — C++ 항목은
                            // 이동(:724) · 점 지우기(:726) · 되돌리기(:730) ·
                            // 맞추기 저장(:745) · 닫기(:793) 다섯 개뿐이다.
                            ui.add(egui::Label::new(egui::RichText::new(ALIGN_SUBTOOLBAR_CAPTION).size(12.5).strong()).selectable(false));
                            if ui.add(tool_button(ALIGN_SUBTOOLBAR_ITEMS[0])).clicked() {
                                // MainWindowAlign.cpp:627-703 applyAlignMove
                                match self.apply_align_move() {
                                    Ok(()) => {}
                                    Err(message) => {
                                        // :632 KaUserError::warn({「이동」, 「도면을 옮기지 못했습니다」, err})
                                        self.show_status_message(format!("도면을 옮기지 못했습니다 — {message}"));
                                    }
                                }
                            }
                            if ui.add(tool_button(ALIGN_SUBTOOLBAR_ITEMS[1])).clicked() {
                                // MainWindowAlign.cpp:726-729 — 지운 뒤 상태줄에 statusText()
                                self.delete_selected_align_point();
                                self.status_bar_hint = self.align.status_text();
                            }
                            if ui.add(tool_button(ALIGN_SUBTOOLBAR_ITEMS[2])).clicked() {
                                // MainWindowAlign.cpp:730-744 — 되돌린 결과를 상태줄(성공)이나
                                // 알림(실패)으로 보여 준다. 문구는 KaAlignMapToolEdit.cpp:132-161.
                                let (restored, message) = self.restore_align_originals();
                                self.status_bar_hint = message.clone();
                                if !restored {
                                    self.home_map.draw_guide_message = Some(format!("맞추기 — {message}"));
                                }
                            }
                            if ui.add(tool_button(ALIGN_SUBTOOLBAR_ITEMS[3])).clicked() {
                                // MainWindowAlign.cpp:745-792 맞추기 저장
                                let saved = self.save_align_result();
                                self.status_bar_hint = saved.1;
                                self.home_map.draw_guide_message = Some(saved.0);
                            }
                            if ui.add(tool_button(ALIGN_SUBTOOLBAR_ITEMS[4])).clicked() {
                                // MainWindowAlign.cpp:793-794 → hideSubTools(:769-777)
                                // → :771-772 stopAlignSession
                                self.hide_sub_tools();
                            }
                        }
                        SubToolsMode::Draw => self.paint_draw_sub_toolbar(ctx, ui),
                        SubToolsMode::None => {}
                    }
                })
            });
        }
    }
}
