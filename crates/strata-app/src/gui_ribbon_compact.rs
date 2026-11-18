//! Compact groups retain C++ action routing, target/cpp-ref/src/app/MainWindowRibbon.cpp:248-711.
use super::gui_ribbon_consts::UX_RIBBON_GROUPS;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Vec2};

impl StrataAppState {
    pub(super) fn paint_compact_ribbon(&mut self, ui: &mut egui::Ui) {
        let [r, g, b, _] = self.theme.chrome;
        egui::Frame::NONE.fill(egui::Color32::from_rgb(r, g, b)).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
            ui.spacing_mut().button_padding = Vec2::new(8.0, 6.0);
            ui.spacing_mut().interact_size.y = 30.0;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                for (group, items) in UX_RIBBON_GROUPS {
                    let mut paper_opened = false;
                    let group_response = crate::gui_shell::native_menu_button(ui, group, |ui| {
                        ui.set_min_width(190.0);
                        for &label in items {
                            let deferred = super::gui_ribbon_draw::deferred_ribbon_tooltip(label);
                            let enabled = deferred.is_none() && !(self.active_tab == MainTab::Home && group == "기록");
                            let response = ui.add_enabled(enabled, egui::Button::new(label).min_size(Vec2::new(190.0, 30.0)));
                            if response.clicked() {
                                if label == "옛 지도" || label == "더보기" {
                                    self.open_ribbon_context_menu(label, response.rect);
                                } else {
                                    self.handle_ribbon_click(label);
                                }
                                paper_opened = label == "도면" && self.modals.active == Some(crate::dialogs_modal::ActiveModalDialog::PaperSetup);
                                ui.close_menu();
                            } else if response.secondary_clicked() {
                                self.open_ribbon_context_menu(label, response.rect);
                                ui.close_menu();
                            }
                            if let Some(tip) = deferred {
                                response.on_disabled_hover_text(tip);
                            }
                        }
                        if group == "기타" {
                            ui.separator();
                            let region = ui.add_sized([190.0, 30.0], egui::Button::new("지역"));
                            if region.clicked() {
                                let rect = region.rect;
                                self.ribbon.app_bar.region_button_rect = Some([rect.left(), rect.top(), rect.right(), rect.bottom()]);
                                self.ribbon.app_bar.region_popup_open = true;
                                self.ribbon.app_bar.address_popup_open = false;
                                ui.close_menu();
                            }
                            let search = ui.add_sized([190.0, 30.0], egui::TextEdit::singleline(&mut self.ribbon.app_bar.query).hint_text("주소·지번 찾기"));
                            if search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.run_app_bar_search();
                                ui.close_menu();
                            }
                        }
                    });
                    if items.contains(&"도면") {
                        crate::modal_paper_account::note_paper_trigger(ui.ctx(), crate::modal_paper_account::PaperTrigger::Ribbon, group_response.response.rect, paper_opened);
                    }
                    if self.ribbon.sub_tool_label().is_some_and(|label| items.contains(&label)) {
                        self.ribbon.sub_toolbar_origin = Some(group_response.response.rect);
                    }
                }
            });
        });
    }
}
