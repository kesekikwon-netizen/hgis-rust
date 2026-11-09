//! `dialogs_modal` 에서 옮긴 대화상자 묶음: SurveyArea.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use crate::dialogs_modal::render_strata_modal_header;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};


pub(crate) fn render_survey_area(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // KaSurveyAreaDialog.cpp:45-160 — 조사구역 그리기 설정
    let full = ctx.screen_rect();
    // screen_rect is already logical points (Task D7)
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 500.0_f32.min(vis_w - 40.0);
    let modal_h = 440.0_f32.min(vis_h - 40.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_survey_area")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 20.0);
            ui.set_max_width(modal_w - 20.0);
            render_strata_modal_header(ui, "조사구역 그리기 설정", open);

            // 1) 조사구역 레이어 이름 & 이어 그릴 레이어 선택 (KaSurveyAreaDialog.cpp:54-69)
            ui.label(
                egui::RichText::new("조사구역 레이어 이름")
                    .size(13.0)
                    .strong()
                    .color(Color32::from_rgb(0x37, 0x41, 0x51)),
            );
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.modals.survey_area_layer_name)
                        .desired_width(240.0),
                );
                for preset_name in ["조사구역", "시굴구역", "발굴구역"] {
                    if ui
                        .selectable_label(
                            app.modals.survey_area_layer_name == preset_name,
                            preset_name,
                        )
                        .clicked()
                    {
                        app.modals.survey_area_layer_name = preset_name.to_owned();
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("이어 그릴 레이어 선택")
                    .size(13.0)
                    .strong()
                    .color(Color32::from_rgb(0x37, 0x41, 0x51)),
            );
            let ring_cnt = app.home_map.real_survey.survey_rings.len();
            let area_m2 = app.home_map.real_survey.survey_area_m2;
            let pyeong = area_m2 / 3.305_785;
            let continue_opts = [
                format!(
                    "지금 선택한 레이어 ({} · {}개 도형)",
                    app.modals.survey_area_layer_name, ring_cnt
                ),
                "+ 새 조사구역 레이어 만들기".to_owned(),
            ];
            let sel_continue = continue_opts
                .get(app.modals.survey_area_continue_idx)
                .cloned()
                .unwrap_or_else(|| continue_opts[0].clone());
            egui::ComboBox::from_id_salt("ka_survey_area_continue_combo")
                .selected_text(sel_continue)
                .width(modal_w - 32.0)
                .show_ui(ui, |ui| {
                    for (idx, opt) in continue_opts.iter().enumerate() {
                        ui.selectable_value(
                            &mut app.modals.survey_area_continue_idx,
                            idx,
                            opt,
                        );
                    }
                });
            ui.label(
                egui::RichText::new(format!(
                    "현재 조사구역 면적: {area_m2:.2} m² (약 {pyeong:.1} 평)"
                ))
                .size(12.0)
                .color(Color32::from_rgb(0x47, 0x55, 0x69)),
            );

            ui.add_space(8.0);
            // 2) 외곽선 색 — 8색 스와치 그리드 (KaSurveyAreaDialog.cpp:74-103)
            let (cr, cg, cb, cname) = crate::right_inspector::KA_STYLE_PALETTE
                [app.modals.survey_area_color_idx.min(7)];
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("외곽선 색")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(0x37, 0x41, 0x51)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "선택: {cname} (#{cr:02X}{cg:02X}{cb:02X})"
                    ))
                    .size(12.0)
                    .color(Color32::from_rgb(cr, cg, cb))
                    .strong(),
                );
            });
            ui.horizontal(|ui| {
                for (idx, &(r, g, b, name)) in
                    crate::right_inspector::KA_STYLE_PALETTE.iter().enumerate()
                {
                    let selected = app.modals.survey_area_color_idx == idx;
                    let btn = egui::Button::new(
                        egui::RichText::new(if selected { "✓" } else { "" })
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(r, g, b))
                    .stroke(if selected {
                        Stroke::new(2.5, Color32::from_rgb(0x11, 0x18, 0x27))
                    } else {
                        Stroke::new(1.0, Color32::from_rgb(0x9C, 0xA3, 0xAF))
                    })
                    .corner_radius(6.0)
                    .min_size(Vec2::new(48.0, 32.0));
                    if ui
                        .add(btn)
                        .on_hover_text(format!("{name} (#{r:02X}{g:02X}{b:02X})"))
                        .clicked()
                    {
                        app.modals.survey_area_color_idx = idx;
                    }
                }
            });

            ui.add_space(8.0);
            // 3) 선 굵기 — 4종 프리셋 + 직접 스핀박스 (KaSurveyAreaDialog.cpp:105-143)
            ui.label(
                egui::RichText::new("선 굵기")
                    .size(13.0)
                    .strong()
                    .color(Color32::from_rgb(0x37, 0x41, 0x51)),
            );
            ui.horizontal(|ui| {
                for (label, w_mm) in [
                    ("보통 1.0", 1.0_f32),
                    ("기본 1.5", 1.5_f32),
                    ("굵게 2.0", 2.0_f32),
                    ("강조 3.0", 3.0_f32),
                ] {
                    let active = (app.modals.survey_area_width_mm - w_mm).abs() < 0.05;
                    let btn = egui::Button::new(
                        egui::RichText::new(label)
                            .size(12.5)
                            .strong()
                            .color(if active {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(0x37, 0x41, 0x51)
                            }),
                    )
                    .fill(if active {
                        Color32::from_rgb(0x25, 0x63, 0xEB)
                    } else {
                        Color32::from_rgb(0xF3, 0xF4, 0xF6)
                    })
                    .corner_radius(6.0)
                    .min_size(Vec2::new(72.0, 30.0));
                    if ui.add(btn).clicked() {
                        app.modals.survey_area_width_mm = w_mm;
                    }
                }
                ui.label("직접");
                ui.add(
                    egui::DragValue::new(&mut app.modals.survey_area_width_mm)
                        .range(0.5..=10.0)
                        .speed(0.1)
                        .suffix(" mm"),
                );
            });

            ui.add_space(12.0);
            ui.separator();
            // 4) 하단 확인/취소 버튼 (KaSurveyAreaDialog.cpp:146-158)
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("취소").clicked() {
                        app.modals.active = None;
                    }
                    let ok_lbl = if app.modals.survey_area_continue_idx == 0 {
                        "계속 그리기 (확인)"
                    } else {
                        "그리기 시작 (Enter)"
                    };
                    let ok_btn = egui::Button::new(
                        egui::RichText::new(ok_lbl)
                            .color(Color32::WHITE)
                            .strong()
                            .size(13.5),
                    )
                    .fill(Color32::from_rgb(0x25, 0x63, 0xEB))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(148.0, 34.0));
                    if ui.add(ok_btn).clicked() {
                        app.right_inspector.style_color_index =
                            app.modals.survey_area_color_idx.min(7);
                        app.right_inspector.stroke_width_mm =
                            app.modals.survey_area_width_mm;
                        app.right_inspector.selected_layer_key =
                            Some(app.modals.survey_area_layer_name.clone());
                        app. modals.applied_line_color_rgb = Some([cr, cg, cb]);
                        app.ribbon.sub_toolbar_visible = true;
                        app.ribbon.sub_mode = crate::ribbon::SubToolsMode::Draw;
                        app.active_tool =
                            strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
                        app.home_map.draw_guide_message = Some(format!(
                            "「{}」 그리기 시작 — 외곽선 색 {cname}(#{cr:02X}{cg:02X}{cb:02X}), 굵기 {:.1} mm",
                            app.modals.survey_area_layer_name,
                            app.modals.survey_area_width_mm
                        ));
                        app.modals.active = None;
                    }
                });
            });
        });
}
