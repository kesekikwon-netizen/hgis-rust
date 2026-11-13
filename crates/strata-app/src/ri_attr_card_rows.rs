//! 오른쪽 카드의 줄 — C++ `KaFeatureCard::buildEditors`(KaFeatureCardEditors.cpp:140-223) 새 모양: 기준점 줄, 「자동 계산」
//! 수치 대장(면적·둘레/길이, 고정폭 16 px 오른쪽, 아래 가는 선), 번호 + 「다음 번호」, 이름·종류·시대(+시대 띠)·조사 줄, 메모.
//! 글 상자는 적는 동안 draft, Enter·칸을 떠나면 쓴다(editingFinished), 메모는 떠날 때만(FocusOut), 목록 고르기는 바로(activated).

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::feature_card::{CardView, ControlRows};
use crate::feature_form::period_key;
use crate::feature_record::placeholder_for;
use crate::right_inspector::KA_BUILTIN_PERIODS;

pub(crate) const MUTED: Color32 = Color32::from_rgb(105, 103, 96);
pub(crate) const INK: Color32 = Color32::from_rgb(31, 35, 40);
const RULE: Color32 = Color32::from_rgb(0xDE, 0xDC, 0xD1);
const EDGE: Color32 = Color32::from_rgb(135, 134, 127);
const WARN: Color32 = Color32::from_rgb(0xB4, 0x53, 0x09);
const LABEL_W: f32 = 58.0;

fn label(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(Vec2::new(LABEL_W, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_width(LABEL_W);
        ui.add(egui::Label::new(RichText::new(text).size(12.0).color(INK)).truncate().selectable(false)).on_hover_text(text);
    });
}

fn frame() -> egui::Frame {
    egui::Frame::NONE.stroke(Stroke::new(1.0, EDGE)).corner_radius(8.0).inner_margin(egui::Margin::symmetric(6, 2)).fill(Color32::WHITE)
}

fn box_id(field: &str) -> egui::Id {
    egui::Id::new(("ka_card_box", field))
}

impl StrataAppState {
    /// 한 줄 글칸 — draft 가 있으면 그것(포커스 있는 칸은 새로 고침이 덮지 않는다).
    pub(crate) fn card_line(&mut self, ui: &mut egui::Ui, field: &str, value: &str, placeholder: &str, width: f32) {
        let mut text = self.home_map.card_drafts.get(field).cloned().unwrap_or_else(|| value.to_owned());
        let r = frame().show(ui, |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(box_id(field)).hint_text(placeholder).desired_width(width - 22.0).frame(false))
        });
        if r.inner.changed() {
            self.card_draft(field, text);
        }
        if r.inner.lost_focus() {
            self.commit_card_field(field);
        }
    }

    /// 낱말 + 한 줄 글칸.
    pub(crate) fn card_text_row(&mut self, ui: &mut egui::Ui, field: &str, word: &str, value: &str, placeholder: &str) {
        ui.horizontal(|ui| {
            label(ui, word);
            let w = ui.available_width();
            self.card_line(ui, field, value, placeholder, w);
        });
    }

    /// 종류·시대 — 직접 적기 + ▾ 목록(목록에서 고르면 바로 쓴다).
    pub(crate) fn card_choice_row(&mut self, ui: &mut egui::Ui, field: &str, word: &str, value: &str, items: &[String]) {
        ui.horizontal(|ui| {
            label(ui, word);
            ui.spacing_mut().item_spacing.x = 2.0;
            let w = ui.available_width() - 26.0;
            self.card_line(ui, field, value, placeholder_for(field), w);
            let button = ui.add(egui::Button::new("").min_size(Vec2::new(24.0, 24.0)));
            let c = button.rect.center();
            let ink = Stroke::new(1.5, Color32::from_rgb(61, 61, 58));
            ui.painter().line_segment([c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.0)], ink);
            ui.painter().line_segment([c + Vec2::new(0.0, 2.0), c + Vec2::new(4.0, -2.0)], ink);
            let popup = ui.id().with(("ka_card_choice", field));
            if button.clicked() {
                ui.memory_mut(|m| m.toggle_popup(popup));
            }
            let mut picked = None;
            egui::popup::popup_below_widget(ui, popup, &button, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                ui.set_min_width(160.0);
                egui::ScrollArea::vertical().max_height(14.0 * 22.0).show(ui, |ui| {
                    for item in items {
                        if ui.selectable_label(item == value, item).clicked() {
                            picked = Some(item.clone());
                        }
                    }
                });
            });
            if let Some(item) = picked {
                self.home_map.card_drafts.remove(field);
                self.card_set_value(field, &item);
            }
        });
    }

    /// 번호 + 「다음 번호」(그 종류의 다음 번호를 넣는다), 같은 번호 글(주황).
    pub(crate) fn card_number_row(&mut self, ui: &mut egui::Ui, v: &CardView) {
        let Some(field) = v.number_field.clone() else { return };
        ui.horizontal(|ui| {
            label(ui, v.number_label);
            let w = ui.available_width() - 70.0;
            self.card_line(ui, &field, &v.number, "", w);
            let next = egui::Button::new(RichText::new("다음 번호").size(11.5));
            let r = ui.add_enabled(!v.next_number.is_empty(), next);
            let r = if v.next_number.is_empty() { r } else { r.on_hover_text(format!("{} 넣기", v.next_number)) };
            if r.clicked() {
                self.home_map.card_drafts.remove(&field);
                self.card_set_value(&field, &v.next_number);
            }
        });
        if let Some(note) = &v.note_number {
            ui.horizontal(|ui| {
                ui.add_space(LABEL_W);
                ui.add(egui::Label::new(RichText::new(note).size(11.5).color(WARN)).wrap());
            });
        }
    }

    /// 메모 — 두 줄(56–72), 칸을 떠날 때만 쓴다.
    pub(crate) fn card_memo(&mut self, ui: &mut egui::Ui, value: &str) {
        let mut text = self.home_map.card_drafts.get("note").cloned().unwrap_or_else(|| value.to_owned());
        ui.horizontal_top(|ui| {
            label(ui, "메모");
            let w = ui.available_width();
            let r = frame().show(ui, |ui| {
                let edit = egui::TextEdit::multiline(&mut text).id(box_id("note")).hint_text("현장 메모 (두 줄)").frame(false);
                ui.add_sized([w - 14.0, 56.0], edit)
            });
            if r.inner.changed() {
                self.card_draft("note", text.clone());
            }
            if r.inner.lost_focus() {
                self.commit_card_field("note");
            }
        });
    }
}

/// 「자동 계산」 수치 대장 — 머리(오른쪽 「자동 계산」, 아래 먹선), 줄마다 낱말·고정폭 16 px 값(오른쪽)·아래 가는 선.
pub(crate) fn ledger(ui: &mut egui::Ui, v: &CardView) {
    if v.area.is_none() && v.perimeter.is_none() {
        return;
    }
    let head = ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("자동 계산").size(11.0).color(MUTED)).on_hover_text("도형에서 바로 잰 값이라 고칠 수 없습니다");
        });
    });
    let y = head.response.rect.bottom() + 2.0;
    ui.painter().line_segment([Pos2::new(head.response.rect.left(), y), Pos2::new(head.response.rect.right(), y)], Stroke::new(1.0, INK));
    ui.add_space(4.0);
    let rows: Vec<(&str, String)> = v.area.iter().map(|a| ("면적", a.clone())).chain(v.perimeter.iter().map(|(w, s)| (*w, s.clone()))).collect();
    for (word, value) in rows {
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::hover());
        let p = ui.painter_at(r);
        p.text(Pos2::new(r.left() + 8.0, r.center().y), egui::Align2::LEFT_CENTER, word, FontId::proportional(12.0), INK);
        p.text(Pos2::new(r.right() - 8.0, r.center().y), egui::Align2::RIGHT_CENTER, value, FontId::new(16.0, FontFamily::Monospace), INK);
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, RULE));
        ui.interact(r, egui::Id::new(("ka_card_measure", word)), Sense::hover()).on_hover_text("자동 계산 — 도형에서 바로 잰 값이라 고칠 수 없습니다");
    }
}

/// 기준점 줄 — 측지 글, X (동)·Y (북)·Z 를 점선 끝에 고정폭으로(보기만).
pub(crate) fn control_rows(ui: &mut egui::Ui, c: &ControlRows) {
    ui.label(RichText::new(&c.meta).size(11.5).color(MUTED));
    for (word, value) in [("X (동)", &c.x), ("Y (북)", &c.y), ("Z", &c.z)] {
        ui.horizontal(|ui| {
            label(ui, word);
            let value_w = 7.5 * value.chars().count() as f32 + 4.0;
            let (r, _) = ui.allocate_exact_size(Vec2::new((ui.available_width() - value_w - 6.0).max(8.0), 10.0), Sense::hover());
            ui.painter().extend(egui::Shape::dotted_line(&[Pos2::new(r.left(), r.center().y), Pos2::new(r.right(), r.center().y)], MUTED, 3.0, 0.6));
            ui.label(RichText::new(value).font(FontId::new(12.5, FontFamily::Monospace)).color(INK));
        });
    }
}

/// KaPeriodBand — 168×13, 시대 색 칸(옛것부터, 미정은 빈 칸), 사이 1 px 선, 적힌 시대는 먹 테두리 + 아래 눈금(보기만).
pub(crate) fn period_band(ui: &mut egui::Ui, period: &str) {
    ui.horizontal(|ui| {
        ui.add_space(LABEL_W);
        let (r, _) = ui.allocate_exact_size(Vec2::new(168.0, 13.0), Sense::hover());
        let p = ui.painter_at(r);
        let cell = 168.0 / KA_BUILTIN_PERIODS.len() as f32;
        let at = |i: usize| Rect::from_min_size(Pos2::new(r.left() + i as f32 * cell, r.top()), Vec2::new(cell, 8.0));
        for (i, (_, rgb)) in KA_BUILTIN_PERIODS.iter().enumerate() {
            match rgb {
                Some((red, g, b)) => p.rect_filled(at(i), 0.0, Color32::from_rgb(*red, *g, *b)),
                None => p.rect_stroke(at(i).shrink(0.5), 0.0, Stroke::new(1.0, EDGE), egui::StrokeKind::Middle),
            };
            if i > 0 {
                p.line_segment([at(i).left_top(), at(i).left_bottom()], Stroke::new(1.0, RULE));
            }
        }
        let key = period_key(period);
        if let Some(i) = KA_BUILTIN_PERIODS.iter().position(|(n, _)| !key.is_empty() && period_key(n) == key) {
            let c = at(i);
            p.rect_stroke(c.shrink(0.5), 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Middle);
            p.line_segment([Pos2::new(c.center().x, r.top() + 9.0), Pos2::new(c.center().x, r.top() + 13.0)], Stroke::new(1.0, INK));
        }
    });
}
