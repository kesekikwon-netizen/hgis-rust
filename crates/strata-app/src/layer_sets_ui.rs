//! In-app questions and menu, target/cpp-ref/src/app/KaDrawingLayerSets.cpp:63-108.
use super::{Question, Request};
use crate::app_state::StrataAppState;
use eframe::egui::{self, Vec2};

impl StrataAppState {
    pub(crate) fn paint_layer_set_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("세트 ▾", |ui| {
            ui.set_max_width(340.0);
            let height = (ui.ctx().screen_rect().height() - 48.0).max(80.0);
            ui.set_max_height(height);
            egui::ScrollArea::vertical().id_salt("named_layer_set_menu").drag_to_scroll(false).max_height(height).show(ui, |ui| {
                let save = ui.add(egui::Button::new("지금 켜진 레이어를 세트로 저장…"));
                if save.clicked() {
                    self.layer_sets.question = Some(Question { request: Request::Name(format!("세트 {}", self.layer_set_names().len() + 1)), focus: true });
                    ui.close_menu();
                }
                let names = self.layer_set_names();
                ui.separator();
                if names.is_empty() {
                    ui.add_enabled(false, egui::Button::new("저장한 세트가 없습니다"));
                } else {
                    for name in &names {
                        if ui.button(format!("적용: {name}")).on_hover_text("이 세트의 켜짐 상태로 레이어를 켜고 끕니다. 지도 화면에도 같이 적용됩니다.").clicked()
                        {
                            if self.apply_layer_set(name) {
                                self.show_status_message(format!("레이어 세트 「{name}」을 적용했습니다."));
                            }
                            ui.close_menu();
                        }
                    }
                    // target/cpp-ref/src/app/KaDrawingLayerSets.cpp:95-105; owner monitor refinement keeps children in the same popup.
                    let remove = egui::CollapsingHeader::new("세트 지우기").open(Some(self.layer_sets.remove_expanded)).show(ui, |ui| {
                        for name in &names {
                            if ui.button(name).clicked() {
                                self.layer_sets.remove_expanded = false;
                                self.layer_sets.question = Some(Question { request: Request::Remove(name.clone()), focus: true });
                                ui.close_menu();
                            }
                        }
                    });
                    if remove.header_response.clicked() {
                        self.layer_sets.remove_expanded = !self.layer_sets.remove_expanded;
                    }
                }
            });
        });
    }

    pub(crate) fn render_layer_set_question(&mut self, ctx: &egui::Context) {
        let Some(mut question) = self.layer_sets.question.take() else { return };
        let (mut accept, mut cancel) = (false, false);
        let name_stage = matches!(&question.request, Request::Name(_));
        let shown = egui::Modal::new(egui::Id::new("named_layer_set_question")).show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 64.0).min(400.0));
            ui.spacing_mut().interact_size.y = 30.0;
            ui.heading("레이어 세트");
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            match &mut question.request {
                Request::Name(text) => {
                    ui.label("세트 이름 (예: 유구 배치, 지형·지적 중첩)");
                    let edit = ui.add_sized([ui.available_width(), 32.0], egui::TextEdit::singleline(text));
                    if question.focus {
                        edit.request_focus();
                    }
                    accept |= enter && (edit.has_focus() || edit.lost_focus());
                }
                Request::Replace(name) => {
                    ui.label(format!("「{name}」 세트가 이미 있습니다. 지금 켜짐 상태로 바꿀까요?"));
                }
                Request::Remove(name) => {
                    ui.label(format!("「{name}」 세트를 지울까요? 레이어는 그대로입니다."));
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let yes = ui.add_sized(Vec2::new(88.0, 32.0), egui::Button::new(if name_stage { "확인" } else { "예" }));
                let no = ui.add_sized(Vec2::new(88.0, 32.0), egui::Button::new(if name_stage { "취소" } else { "아니요" }));
                if !name_stage && question.focus {
                    no.request_focus();
                }
                accept |= yes.clicked() || (!name_stage && enter && yes.has_focus());
                cancel |= no.clicked() || (!name_stage && enter && !yes.has_focus());
            });
        });
        let close = !cancel && shown.should_close();
        if cancel || close {
            return;
        }
        if !accept {
            question.focus = false;
            self.layer_sets.question = Some(question);
            return;
        }
        match question.request {
            Request::Name(text) => {
                let name = text.trim().to_owned();
                if name.is_empty() {
                    return;
                }
                if self.layer_sets.catalog.contains_key(&name) {
                    self.layer_sets.question = Some(Question { request: Request::Replace(name), focus: true });
                } else {
                    self.finish_layer_set_store(&name);
                }
            }
            Request::Replace(name) => self.finish_layer_set_store(&name),
            Request::Remove(name) => {
                if self.remove_layer_set(&name) {
                    self.show_status_message(format!("레이어 세트 「{name}」을 지웠습니다."));
                }
            }
        }
    }

    fn finish_layer_set_store(&mut self, name: &str) {
        if self.store_layer_set(name) {
            self.show_status_message(format!("레이어 세트 「{name}」을 저장했습니다. 고를 때만 적용됩니다."));
        }
    }
}
