//! 「조사구역 그리기 설정」 창 — C++ `KaSurveyAreaDialog::setupUi`(KaSurveyAreaDialog.cpp:58-178, 새 모양).
//! 최소 너비 440, 바깥 여백 24, 줄 사이 12, 왼쪽 낱말 칸 96: 「이름」, 「외곽선 색」(고른 색 이름이 그 아래), 색 단추
//! 52×32 넷씩 두 줄(고른 것은 먹색 2 px 테두리), 「선 굵기」 붙은 단추 넷, 「직접」 0.5~10.0 mm(0.2씩), 아래에
//! 「그리기 시작 (Enter)」(주 단추)·「취소」. Enter 는 그리기 시작, Esc 는 취소.

use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::survey_area_setup::{PALETTE, SWATCHES_PER_ROW, WIDTH_PRESETS};

const WORD_W: f32 = 96.0;
const INK: Color32 = Color32::from_rgb(20, 20, 19);
const MAIN: Color32 = Color32::from_rgb(181, 87, 58);

fn word(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(Vec2::new(WORD_W, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_width(WORD_W);
        ui.add(egui::Label::new(text).selectable(false));
    });
}

impl StrataAppState {
    /// 열려 있으면 그린다.
    pub(crate) fn render_survey_area_setup(&mut self, ctx: &egui::Context) {
        let Some(mut setup) = self.home_map.survey_area_setup.clone() else { return };
        let (mut start, mut cancel) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        egui::Window::new("조사구역 그리기 설정")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .min_width(440.0)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(24.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.horizontal(|ui| {
                    word(ui, "이름");
                    ui.add(egui::TextEdit::singleline(&mut setup.name).desired_width(280.0));
                });
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(WORD_W);
                        ui.label("외곽선 색");
                        ui.add_space(2.0);
                        ui.label(setup.color_name());
                    });
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        for row in PALETTE.chunks(SWATCHES_PER_ROW).enumerate() {
                            ui.horizontal(|ui| {
                                for (col, (rgb, name)) in row.1.iter().enumerate() {
                                    let idx = row.0 * SWATCHES_PER_ROW + col;
                                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(52.0, 32.0), Sense::click());
                                    let resp = resp.on_hover_text(*name);
                                    let p = ui.painter();
                                    let inner = Rect::from_min_max(rect.min + Vec2::splat(4.0), rect.max - Vec2::splat(4.0));
                                    p.rect_filled(inner, 2.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                                    if idx == setup.color {
                                        p.rect_stroke(rect.shrink(1.0), 4.0, Stroke::new(2.0, INK), egui::StrokeKind::Inside);
                                    }
                                    if resp.clicked() {
                                        setup.color = idx;
                                    }
                                }
                            });
                        }
                    });
                });
                ui.horizontal(|ui| {
                    word(ui, "선 굵기");
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let chosen = setup.chosen_preset();
                    for (i, (label, w)) in WIDTH_PRESETS.iter().enumerate() {
                        let b = egui::Button::new(*label).selected(chosen == Some(i)).min_size(Vec2::new(0.0, 32.0));
                        if ui.add(b).clicked() {
                            setup.width_mm = *w;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    word(ui, "직접");
                    ui.add_sized(
                        [96.0, 28.0],
                        egui::DragValue::new(&mut setup.width_mm).range(0.5..=10.0).speed(0.2).fixed_decimals(1).suffix(" mm"),
                    );
                });
                ui.add_space(12.0);
                // 단추 줄은 높이 36 만 — 남은 높이를 다 차지하면 창이 늘어난다.
                ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 36.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized([80.0, 36.0], egui::Button::new("취소")).clicked() {
                        cancel = true;
                    }
                    let main = egui::Button::new(RichText::new("그리기 시작 (Enter)").color(Color32::WHITE)).fill(MAIN);
                    if ui.add_sized([130.0, 36.0], main).clicked() {
                        start = true;
                    }
                });
            });
        self.home_map.survey_area_setup = Some(setup);
        if cancel {
            self.home_map.survey_area_setup = None;
        } else if start {
            self.accept_survey_area_setup();
        }
    }
}
