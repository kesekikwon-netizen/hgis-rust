//! 「도형 속성 — <레이어>」 창 — C++ `editFeatureAttributes`(MainWindowEditing.cpp:642-699) + 새 모양 `KaRecordPlate::arrange`
//! (KaRecordPlate.cpp:62-266): 폭 560(KaWindowSize Medium), 여백 24·24·8·16(+판 오른쪽 16), 안내, 구역마다 위 16·낱말+가는 선·
//! 아래 8·줄 사이 12, 반 칸 = 낱말 96 + 칸 148(둘이면 사이 24), 한 줄 칸, 표고 칸은 고정폭 숫자 오른쪽 맞춤 + 「m」,
//! 상면·바닥 옆에 단면 그림, 비고 72 높이 여러 줄, 판은 넘치면 굴림, 아래 24 에 「저장」「취소」. Enter 저장(비고 안에선 줄바꿈),
//! Esc 취소.

use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Vec2};

use crate::app_state::StrataAppState;
use crate::dlg_feature_record_figure::elevation_figure;
use crate::feature_record::{placeholder_for, RecordEditor, RecordRow, ELEVATION_SECTION, RECORD_BUTTONS, RECORD_SECTIONS, RECORD_TIP};

const CONTENT_W: f32 = 512.0;
const WORD_W: f32 = 96.0;
const HALF_BOX: f32 = 148.0;
/// 칸 틀의 안 여백 6·6 + 선 1·1(FRAME_PAD), 그리고 글 상자 자체 여백 4·4(BOX_PAD 에 더함) — 칸 전체가 QLineEdit 폭이 되게.
const FRAME_PAD: f32 = 14.0;
const BOX_PAD: f32 = FRAME_PAD + 8.0;
const MUTED: Color32 = Color32::from_rgb(105, 103, 96);
const RULE: Color32 = Color32::from_rgb(0xDE, 0xDC, 0xD1);
const EDGE: Color32 = Color32::from_rgb(135, 134, 127);
const MAIN: Color32 = Color32::from_rgb(181, 87, 58);

/// 낱말 96 — 길면 줄이고 마우스를 올리면 다 보인다.
fn word(ui: &mut egui::Ui, text: &str, top: bool) {
    let layout = egui::Layout::left_to_right(if top { egui::Align::Min } else { egui::Align::Center });
    ui.allocate_ui_with_layout(Vec2::new(WORD_W, 26.0), layout, |ui| {
        ui.set_width(WORD_W);
        if top {
            ui.add_space(0.0);
        }
        let label = egui::Label::new(RichText::new(text).size(12.0).color(MUTED)).truncate().selectable(false);
        ui.add(label).on_hover_text(text);
    });
}

/// 칸의 글 상자 id(시험·글쇠 옮기기용).
pub(crate) fn record_box_id(field: &str) -> egui::Id {
    egui::Id::new(("ka_record_box", field))
}

/// 칸 하나의 편집기. 비고(여러 줄)에 글쇠가 있으면 `lines_focus`.
fn editor(ui: &mut egui::Ui, row: &mut RecordRow, width: f32, items: &[String], lines_focus: &mut bool) {
    let metres = row.section == ELEVATION_SECTION;
    let id = record_box_id(&row.field);
    let frame = egui::Frame::NONE.stroke(egui::Stroke::new(1.0, EDGE)).corner_radius(4.0).inner_margin(egui::Margin::symmetric(6, 3)).fill(Color32::WHITE);
    match row.editor {
        RecordEditor::Lines => {
            let r = frame.show(ui, |ui| {
                ui.add_sized([width - FRAME_PAD, 72.0 - 8.0], egui::TextEdit::multiline(&mut row.text).id(id).frame(false))
            });
            *lines_focus |= r.inner.has_focus();
        }
        RecordEditor::Choice => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                frame.show(ui, |ui| {
                    let edit = egui::TextEdit::singleline(&mut row.text).id(id).hint_text(placeholder_for(&row.field)).desired_width(width - BOX_PAD - 26.0).frame(false);
                    ui.add(edit).on_hover_text(placeholder_for(&row.field));
                });
                let button = ui.add(egui::Button::new("").min_size(Vec2::new(24.0, 24.0)));
                let c = button.rect.center();
                let ink = egui::Stroke::new(1.5, Color32::from_rgb(61, 61, 58));
                ui.painter().line_segment([c + Vec2::new(-4.0, -2.0), c + Vec2::new(0.0, 2.0)], ink);
                ui.painter().line_segment([c + Vec2::new(0.0, 2.0), c + Vec2::new(4.0, -2.0)], ink);
                let popup = ui.id().with(("ka_record_choice", row.field.as_str()));
                if button.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(popup));
                }
                egui::popup::popup_below_widget(ui, popup, &button, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                    ui.set_min_width(width);
                    egui::ScrollArea::vertical().max_height(14.0 * 22.0).show(ui, |ui| {
                        for item in items {
                            if ui.selectable_label(*item == row.text, item).clicked() {
                                row.text = item.clone();
                            }
                        }
                    });
                });
            });
        }
        _ => {
            frame.show(ui, |ui| {
                let mut edit = egui::TextEdit::singleline(&mut row.text).id(id).hint_text(placeholder_for(&row.field)).frame(false);
                if metres {
                    // 표고 칸(metreBox): 고정폭 숫자, 오른쪽 맞춤, 칸 안 오른쪽에 「m」.
                    edit = edit.font(FontId::new(13.0, FontFamily::Monospace)).horizontal_align(egui::Align::Max).desired_width(width - BOX_PAD - 18.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.add(edit);
                        ui.label(RichText::new("m").size(12.0).color(MUTED));
                    });
                } else {
                    ui.add(edit.desired_width(width - BOX_PAD));
                }
            });
        }
    }
}

/// 낱말 + 칸.
fn cell(ui: &mut egui::Ui, row: &mut RecordRow, box_w: f32, items: &[String], lines_focus: &mut bool) {
    ui.push_id(row.field.clone(), |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            word(ui, &row.label.clone(), row.editor == RecordEditor::Lines);
            editor(ui, row, box_w, items, lines_focus);
        });
    });
}

impl StrataAppState {
    /// 열려 있으면 그린다.
    pub(crate) fn render_feature_record(&mut self, ctx: &egui::Context) {
        let Some(mut form) = self.home_map.feature_record.clone() else { return };
        let (mut save, mut cancel) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        let list_open = ctx.memory(|m| m.any_popup_open());
        save &= form.keys_armed && !list_open;
        cancel &= form.keys_armed && !list_open;
        form.keys_armed = true;
        let mut lines_focus = false;
        let room = (ctx.screen_rect().height() - 180.0).max(200.0);
        egui::Window::new(form.title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(CONTENT_W + 16.0)
            .max_width(CONTENT_W + 16.0)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin { left: 24, right: 8, top: 24, bottom: 16 }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(room).show(ui, |ui| {
                    ui.set_width(CONTENT_W);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(RECORD_TIP).size(12.0).color(MUTED));
                    let kinds = form.kind_choices.clone();
                    let periods = form.period_choices.clone();
                    for title in form.section_words() {
                        let s = RECORD_SECTIONS.iter().position(|t| *t == title).unwrap_or(0);
                        let idx: Vec<usize> = (0..form.rows.len()).filter(|i| form.rows[*i].section == s).collect();
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(RichText::new(title).size(11.0).color(MUTED));
                            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 0.0, RULE);
                        });
                        ui.add_space(8.0);
                        record_rows(ui, &mut form.rows, &idx, s, &kinds, &periods, &mut lines_focus);
                    }
                });
                ui.add_space(24.0);
                ui.allocate_ui_with_layout(Vec2::new(CONTENT_W, 32.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized([88.0, 30.0], egui::Button::new(RECORD_BUTTONS[1])).clicked() {
                        cancel = true;
                    }
                    if ui.add_sized([88.0, 30.0], egui::Button::new(RichText::new(RECORD_BUTTONS[0]).color(Color32::WHITE)).fill(MAIN)).clicked() {
                        save = true;
                    }
                });
            });
        // 비고(여러 줄) 안의 Enter 는 줄바꿈이다(QPlainTextEdit 이 먹는다).
        save &= !lines_focus || !ctx.input(|i| i.key_pressed(egui::Key::Enter));
        self.home_map.feature_record = Some(form);
        if cancel {
            self.cancel_feature_record();
        } else if save {
            self.save_feature_record();
        }
    }
}

/// 한 구역의 줄 — 반 칸은 둘씩(addRows), 표고는 상면·바닥 옆에 그림(addElevations).
#[allow(clippy::too_many_arguments)]
fn record_rows(ui: &mut egui::Ui, rows: &mut [RecordRow], idx: &[usize], section: usize, kinds: &[String], periods: &[String], focus: &mut bool) {
    let items = |r: &RecordRow| if r.field == "kind" { kinds.to_vec() } else if r.field == "period" { periods.to_vec() } else { Vec::new() };
    let mut rest: Vec<usize> = idx.to_vec();
    if section == ELEVATION_SECTION {
        let top = idx.iter().copied().find(|i| rows[*i].field == "top_el");
        let bottom = idx.iter().copied().find(|i| rows[*i].field == "bottom_el");
        if let (Some(t), Some(b)) = (top, bottom) {
            let depth = idx.iter().copied().find(|i| rows[*i].field == "depth_m").map(|i| rows[i].text.clone()).unwrap_or_default();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 24.0;
                ui.vertical(|ui| {
                    ui.set_width(WORD_W + HALF_BOX);
                    ui.spacing_mut().item_spacing.y = 12.0;
                    for i in [t, b] {
                        cell(ui, &mut rows[i], HALF_BOX, &[], focus);
                    }
                });
                elevation_figure(ui, &rows[t].text, &rows[b].text, &depth);
            });
            ui.add_space(12.0);
            rest.retain(|i| *i != t && *i != b);
        }
    }
    let mut k = 0;
    while k < rest.len() {
        let first = rest[k];
        if rows[first].across {
            let list = items(&rows[first]);
            cell(ui, &mut rows[first], CONTENT_W - WORD_W, &list, focus);
            k += 1;
        } else {
            let second = rest.get(k + 1).copied().filter(|i| !rows[*i].across);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 24.0;
                for i in std::iter::once(first).chain(second) {
                    let list = items(&rows[i]);
                    cell(ui, &mut rows[i], HALF_BOX, &list, focus);
                }
            });
            k += if second.is_some() { 2 } else { 1 };
        }
        if k < rest.len() {
            ui.add_space(12.0);
        }
    }
}

#[cfg(test)]
#[path = "dlg_feature_record_tests.rs"]
mod tests;
