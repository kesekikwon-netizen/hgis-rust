//! 「이름·번호」 창 — C++ `KaFeatureFormDialog` 생성자(KaFeatureFormDialog.cpp:36-118, 새 모양): 여백 24·24·24·16,
//! 줄 사이 12, 낱말 칸 96. 안내 두 줄, 「종류」(목록 + 직접 적기) 또는 「이름」, 「시대」(시대 칸이 있을 때), 「번호」와
//! 그 옆 「다음 번호를 미리 넣었습니다」, 같은 번호면 주황(#B45309) 글, 「저장」·「건너뛰기」. Enter 는 저장, Esc 는 건너뛰기.

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::app_state::StrataAppState;

const WORD_W: f32 = 96.0;
const NOTE: Color32 = Color32::from_rgb(0xB4, 0x53, 0x09);
const MAIN: Color32 = Color32::from_rgb(181, 87, 58);
const EDGE: Color32 = Color32::from_rgb(135, 134, 127);

/// 테두리 있는 한 줄 글칸(QLineEdit) — 지금 테마의 egui 글칸은 테두리가 보이지 않는다.
fn boxed(ui: &mut egui::Ui, edit: egui::TextEdit<'_>) -> egui::Response {
    egui::Frame::NONE
        .stroke(egui::Stroke::new(1.0, EDGE))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .fill(Color32::WHITE)
        .show(ui, |ui| ui.add(edit.frame(false)))
        .inner
}

fn word(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(Vec2::new(WORD_W, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_width(WORD_W);
        ui.add(egui::Label::new(text).selectable(false));
    });
}

/// 직접 적을 수 있는 고르기(makeChoiceCombo: 편집 가능, 목록은 ▾). 바뀌면 true.
fn choice(ui: &mut egui::Ui, id: &str, text: &mut String, items: &[String], placeholder: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        changed |= boxed(ui, egui::TextEdit::singleline(text).hint_text(placeholder).desired_width(224.0)).changed();
        // 목록 단추 — 글꼴에 ▾ 가 없어 그려 넣는다.
        let button = ui.add(egui::Button::new("").min_size(Vec2::new(24.0, 24.0)));
        let c = button.rect.center();
        let ink = egui::Stroke::new(1.5, Color32::from_rgb(61, 61, 58));
        ui.painter().line_segment([c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.0)], ink);
        ui.painter().line_segment([c + Vec2::new(0.0, 2.0), c + Vec2::new(4.0, -2.0)], ink);
        let popup = ui.id().with(("ka_form_choice", id));
        if button.clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup));
        }
        egui::popup::popup_below_widget(ui, popup, &button, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(240.0);
            egui::ScrollArea::vertical().max_height(14.0 * 22.0).show(ui, |ui| {
                for item in items {
                    if ui.selectable_label(item == text, item).clicked() {
                        *text = item.clone();
                        changed = true;
                    }
                }
            });
        });
    });
    changed
}

impl StrataAppState {
    /// 열려 있으면 그린다.
    pub(crate) fn render_feature_form(&mut self, ctx: &egui::Context) {
        let Some(mut form) = self.home_map.feature_form.clone() else { return };
        let (mut save, mut skip) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        // 연 프레임의 키는 그리기의 것(Enter 로 마침), 목록(▾)이 열려 있으면 Esc 는 목록만 닫는다.
        let list_open = ctx.memory(|m| m.any_popup_open());
        save &= form.keys_armed && !list_open;
        skip &= form.keys_armed && !list_open;
        form.keys_armed = true;
        egui::Window::new("이름·번호")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(380.0)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin { left: 24, right: 24, top: 24, bottom: 16 }))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.label("이름과 번호는 나중에 넣어도 됩니다.\n건너뛰어도 그린 도형은 그대로 남습니다.");
                if form.name_is_kind {
                    ui.horizontal(|ui| {
                        word(ui, "종류");
                        let items = form.kind_choices.clone();
                        if choice(ui, "kind", &mut form.name, &items, "예: 주거지, 수혈 — 목록에 없으면 직접 적으세요") {
                            form.update_number_hint();
                        }
                    });
                } else if form.fields.name_field.is_some() {
                    ui.horizontal(|ui| {
                        word(ui, "이름");
                        boxed(ui, egui::TextEdit::singleline(&mut form.name).hint_text("예: 조사명").desired_width(250.0));
                    });
                }
                if form.has_period {
                    ui.horizontal(|ui| {
                        word(ui, "시대");
                        let items = form.period_choices.clone();
                        choice(ui, "period", &mut form.period, &items, "예: 청동기, 원삼국");
                    });
                }
                if form.fields.number_field.is_some() {
                    ui.horizontal(|ui| {
                        word(ui, "번호");
                        let edit = egui::TextEdit::singleline(&mut form.number).hint_text("예: 1호").desired_width(120.0);
                        if boxed(ui, edit).changed() {
                            form.number_typed = true;
                            form.update_number_hint();
                        }
                        if form.number_filled_visible() {
                            ui.add_space(8.0);
                            ui.label("다음 번호를 미리 넣었습니다");
                        }
                    });
                    if let Some(note) = form.number_note() {
                        ui.horizontal(|ui| {
                            ui.add_space(WORD_W);
                            ui.add(egui::Label::new(RichText::new(note).color(NOTE)).wrap());
                        });
                    }
                }
                ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 32.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized([88.0, 30.0], egui::Button::new("건너뛰기")).clicked() {
                        skip = true;
                    }
                    let main = egui::Button::new(RichText::new("저장").color(Color32::WHITE)).fill(MAIN);
                    if ui.add_sized([88.0, 30.0], main).clicked() {
                        save = true;
                    }
                });
            });
        self.home_map.feature_form = Some(form);
        if skip {
            self.skip_feature_form();
        } else if save {
            self.save_feature_form();
        }
    }
}
