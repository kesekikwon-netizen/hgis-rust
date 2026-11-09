//! `dialogs_modal` 에서 옮긴 대화상자 묶음: LineColor, DemClass.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use crate::dialogs_modal::render_strata_modal_header;
use crate::modal_types::ModalDialogController;
use crate::app_state::StrataAppState;


pub(crate) fn render_line_color(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // KaLineColorDialog.cpp:25-75 — 선 색 및 선 굵기 다이얼로그
    let full = ctx.screen_rect();
    // screen_rect is already logical points (Task D7)
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 430.0_f32.min(vis_w - 40.0);
    let modal_h = 310.0_f32.min(vis_h - 40.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_line_color")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 20.0);
            ui.set_max_width(modal_w - 20.0);
            let target_layer = app
                .right_inspector
                .selected_layer_key
                .clone()
                .unwrap_or_else(|| "조사구역".to_owned());
            render_strata_modal_header(ui, "선 색", open);
            ui.label(
                egui::RichText::new(format!("대상 레이어: {target_layer}"))
                    .size(12.0)
                    .color(Color32::from_rgb(0x47, 0x55, 0x69)),
            );
            ui.add_space(6.0);
            // KaLineColorDialog.cpp:48-51 — 선이 아닌 레이어 경고
            if !ModalDialogController::is_line_map_layer(&target_layer) {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "「{target_layer}」은 색을 바꾸지 않습니다.\n면을 색으로 채운 지도(토양도·지질도)와 점·글자는 공식 범례·도면 표시 그대로 둡니다.\n도면 선·받은 지적도·등고선처럼 선으로 된 지도나 조사 데이터 레이어를 고르세요."
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(0x9A, 0x34, 0x12)),
                );
            }
            ui.add_space(6.0);

            let [r, g, b] = app.modals.line_color_rgb;
            let fg = if i32::from(r) + i32::from(g) + i32::from(b) > 380 {
                Color32::from_rgb(0x11, 0x18, 0x27)
            } else {
                Color32::WHITE
            };
            egui::Grid::new("ka_line_color_form")
                .num_columns(2)
                .spacing([14.0, 10.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("선 색").size(13.0).strong());
                    let hex_btn = egui::Button::new(
                        egui::RichText::new(format!(
                            "#{r:02X}{g:02X}{b:02X} · 클릭해서 색 고르기"
                        ))
                        .color(fg)
                        .strong(),
                    )
                    .fill(Color32::from_rgb(r, g, b))
                    .stroke(Stroke::new(2.0, Color32::from_rgb(0x9C, 0xA3, 0xAF)))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(290.0, 32.0));
                    if ui.add(hex_btn).clicked() {
                        let next_idx = (app.modals.line_color_idx + 1) % 8;
                        app.modals.line_color_idx = next_idx;
                        let (nr, ng, nb, _) = crate::right_inspector::KA_STYLE_PALETTE[next_idx];
                        app.modals.line_color_rgb = [nr, ng, nb];
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("색 팔레트").size(12.0));
                    ui.horizontal(|ui| {
                        for (idx, &(sr, sg, sb, sname)) in
                            crate::right_inspector::KA_STYLE_PALETTE.iter().enumerate()
                        {
                            let active = app.modals.line_color_rgb == [sr, sg, sb];
                            let sw_btn = egui::Button::new(
                                egui::RichText::new(if active { "✓" } else { "" })
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(sr, sg, sb))
                            .stroke(if active {
                                Stroke::new(2.0, Color32::from_rgb(0x11, 0x18, 0x27))
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(0x9C, 0xA3, 0xAF))
                            })
                            .corner_radius(4.0)
                            .min_size(Vec2::new(30.0, 26.0));
                            if ui
                                .add(sw_btn)
                                .on_hover_text(format!("{sname} (#{sr:02X}{sg:02X}{sb:02X})"))
                                .clicked()
                            {
                                app.modals.line_color_idx = idx;
                                app.modals.line_color_rgb = [sr, sg, sb];
                            }
                        }
                    });
                    ui.end_row();

                    ui.label(egui::RichText::new("선 굵기").size(13.0).strong());
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut app.modals.line_width_mm)
                                .range(0.1..=5.0)
                                .speed(0.1)
                                .fixed_decimals(2)
                                .suffix(" mm"),
                        );
                        for w_mm in [0.5_f32, 1.0, 1.5, 2.0, 3.0] {
                            if ui
                                .selectable_label(
                                    (app.modals.line_width_mm - w_mm).abs() < 0.05,
                                    format!("{w_mm:.1}mm"),
                                )
                                .clicked()
                            {
                                app.modals.line_width_mm = w_mm;
                            }
                        }
                    });
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.separator();
            // 하단 버튼 행: [원래 색으로] ··· [확인] [취소] (KaLineColorDialog.cpp:54-67)
            ui.horizontal(|ui| {
                if ui.button("원래 색으로").clicked() {
                    app.modals.line_color_idx = 1;
                    app.modals.line_color_rgb = [0xEA, 0x58, 0x0C];
                    app.modals.line_width_mm = 1.5;
                    app.modals.applied_line_color_rgb = None;
                    app.right_inspector.style_color_index = 1;
                    app.right_inspector.stroke_width_mm = 1.5;
                    app.home_map.draw_guide_message =
                        Some(format!("「{target_layer}」 선 색을 원래 색(#EA580C, 1.5 mm)으로 되돌렸습니다."));
                    app.modals.active = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("취소").clicked() {
                        app.modals.active = None;
                    }
                    let ok_btn = egui::Button::new(
                        egui::RichText::new("적용")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(33, 33, 31))
                    .min_size(Vec2::new(72.0, 30.0));
                    if ui.add(ok_btn).clicked() {
                        app.right_inspector.style_color_index =
                            app.modals.line_color_idx.min(7);
                        app.right_inspector.stroke_width_mm = app.modals.line_width_mm;
                        app.modals.applied_line_color_rgb =
                            Some(app.modals.line_color_rgb);
                        app.home_map.draw_guide_message = Some(format!(
                            "「{target_layer}」 선 색 #{r:02X}{g:02X}{b:02X} · 굵기 {:.2} mm 적용됨",
                            app.modals.line_width_mm
                        ));
                        app.modals.active = None;
                    }
                });
            });
        });
}
