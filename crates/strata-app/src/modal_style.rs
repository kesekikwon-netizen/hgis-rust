//! `dialogs_modal` 에서 옮긴 대화상자 묶음: LineColor, DemClass.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use crate::dialogs_modal::render_strata_modal_header;
use crate::modal_types::ModalDialogController;
use crate::app_state::StrataAppState;


pub(crate) fn render_line_color(app: &mut StrataAppState, ctx: &egui::Context, mut mut open: &mut bool) {
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

pub(crate) fn render_dem_class(app: &mut StrataAppState, ctx: &egui::Context, mut mut open: &mut bool) {
    // KaDemClassDialog.cpp:91-170 — DEM 표현 다이얼로그
    let full = ctx.screen_rect();
    // screen_rect is already logical points (Task D7)
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 500.0_f32.min(vis_w - 40.0);
    let modal_h = 430.0_f32.min(vis_h - 40.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_dem_class")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 20.0);
            ui.set_max_width(modal_w - 20.0);
            render_strata_modal_header(ui, "DEM 표현", open);

            // KaDemClassDialog.cpp:95-100 — 안내 문구
            ui.label(
                egui::RichText::new(
                    "표고에 따라 색이 부드럽게 이어집니다. 유적처럼 높이차가 작은 곳은 「현재 화면에 색 맞추기」로 화면 안의 높이만으로 색을 나누세요.",
                )
                .size(12.0)
                .color(Color32::from_rgb(0x47, 0x55, 0x69)),
            );
            ui.add_space(8.0);

            egui::Grid::new("ka_dem_class_form")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    // 등고 / 음영 / 고도 모드 선택
                    ui.label(egui::RichText::new("DEM 모드").size(12.5).strong());
                    ui.horizontal(|ui| {
                        for (idx, lbl) in ["등고", "음영", "고도"].into_iter().enumerate() {
                            if ui
                                .radio_value(&mut app.modals.dem_mode_idx, idx, lbl)
                                .clicked()
                            {
                                app.modals.dem_applied_mode_idx = idx;
                            }
                        }
                    });
                    ui.end_row();

                    // 색 표현 프리셋 콤보 + [현재 화면에 색 맞추기] 버튼 (KaDemClassDialog.cpp:104-121)
                    ui.label(egui::RichText::new("색 표현").size(12.5).strong());
                    ui.horizontal(|ui| {
                        let presets =
                            ["전국 표준 (0–2000m)", "저지대 강조", "현재 화면 맞춤"];
                        let cur_preset = presets
                            .get(app.modals.dem_preset_idx)
                            .copied()
                            .unwrap_or(presets[0]);
                        egui::ComboBox::from_id_salt("ka_dem_preset_combo")
                            .selected_text(cur_preset)
                            .width(175.0)
                            .show_ui(ui, |ui| {
                                for (idx, p_lbl) in presets.into_iter().enumerate() {
                                    ui.selectable_value(
                                        &mut app.modals.dem_preset_idx,
                                        idx,
                                        p_lbl,
                                    );
                                }
                            });
                        if ui.button("현재 화면에 색 맞추기").clicked() {
                            app.modals.dem_preset_idx = 2;
                            app.modals.dem_mode_idx = 2;
                            app.modals.dem_status_text =
                                "현재 화면 범위(32.4 m ~ 74.8 m)에 맞춰 고도 색 띠를 재조정했습니다."
                                    .to_owned();
                        }
                    });
                    ui.end_row();

                    // 색 띠 미리보기 (KaDemClassDialog.cpp:36-80, 123-131)
                    ui.label(egui::RichText::new("색 띠 미리보기").size(12.5).strong());
                    let (ramp_r, _) = ui.allocate_exact_size(
                        Vec2::new(350.0, 38.0),
                        egui::Sense::hover(),
                    );
                    let rp = ui.painter_at(ramp_r);
                    let bar_h = 20.0;
                    let bar_r = Rect::from_min_size(
                        ramp_r.min,
                        Vec2::new(ramp_r.width(), bar_h),
                    );
                    let steps = 24usize;
                    let sw = bar_r.width() / (steps as f32);
                    for si in 0..steps {
                        let t = (si as f32) / ((steps - 1) as f32);
                        let col = match app.modals.dem_preset_idx {
                            0 => {
                                // 전국 표준 (0-2000m)
                                let r = (74.0 + t * 145.0) as u8;
                                let g = (158.0 + (1.0 - (t - 0.4).abs()) * 45.0)
                                    .clamp(95.0, 215.0) as u8;
                                let b = (82.0 + t * 110.0) as u8;
                                Color32::from_rgb(r, g, b)
                            }
                            1 => {
                                // 저지대 강조
                                let r = (46.0 + t * 165.0) as u8;
                                let g = (144.0 - t * 52.0) as u8;
                                let b = (114.0 - t * 68.0) as u8;
                                Color32::from_rgb(r, g, b)
                            }
                            _ => {
                                // 현재 화면 맞춤 (32.4m ~ 74.8m 고대비)
                                let r = (42.0 + t * 195.0) as u8;
                                let g = if t < 0.5 {
                                    (135.0 + t * 120.0) as u8
                                } else {
                                    (195.0 - (t - 0.5) * 240.0) as u8
                                };
                                let b = (68.0 - t * 38.0) as u8;
                                Color32::from_rgb(r, g, b)
                            }
                        };
                        let seg = Rect::from_min_size(
                            Pos2::new(bar_r.left() + (si as f32) * sw, bar_r.top()),
                            Vec2::new(sw + 1.0, bar_h),
                        );
                        rp.rect_filled(seg, 0.0, col);
                    }
                    rp.rect_stroke(
                        bar_r,
                        3.0,
                        Stroke::new(1.0, Color32::from_rgb(0xCB, 0xD5, 0xE1)),
                        egui::StrokeKind::Inside,
                    );
                    let (min_lbl, max_lbl) = match app.modals.dem_preset_idx {
                        0 => ("0 m", "2000 m"),
                        1 => ("0 m", "300 m"),
                        _ => ("32.4 m", "74.8 m"),
                    };
                    rp.text(
                        Pos2::new(ramp_r.left() + 2.0, ramp_r.bottom() - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        min_lbl,
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(0x47, 0x55, 0x69),
                    );
                    rp.text(
                        Pos2::new(ramp_r.right() - 2.0, ramp_r.bottom() - 2.0),
                        egui::Align2::RIGHT_BOTTOM,
                        max_lbl,
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(0x47, 0x55, 0x69),
                    );
                    ui.end_row();

                    // 지형 음영 합성 체크박스 (KaDemClassDialog.cpp:133-138)
                    ui.label("");
                    ui.checkbox(&mut app.modals.dem_relief_enabled, "지형 음영 합성");
                    ui.end_row();

                    // 수직과장 (KaDemClassDialog.cpp:140-145)
                    ui.label(egui::RichText::new("수직과장").size(12.5).strong());
                    ui.add(
                        egui::DragValue::new(&mut app.modals.dem_exaggeration)
                            .range(0.1..=5.0)
                            .speed(0.1)
                            .suffix(" 배"),
                    );
                    ui.end_row();

                    // 음영 세기 (KaDemClassDialog.cpp:147-152)
                    ui.label(egui::RichText::new("음영 세기").size(12.5).strong());
                    ui.add(
                        egui::DragValue::new(&mut app.modals.dem_relief_strength_pct)
                            .range(0..=80)
                            .speed(5.0)
                            .suffix(" %"),
                    );
                    ui.end_row();
                });

            ui.add_space(6.0);
            // 상태 메시지 (KaDemClassDialog.cpp:154-158)
            ui.colored_label(
                Color32::from_rgb(0x7A, 0x5A, 0x00),
                egui::RichText::new(&app.modals.dem_status_text).size(11.5),
            );

            ui.add_space(8.0);
            ui.separator();
            // 하단 버튼 행: [적용] [확인] [취소] (KaDemClassDialog.cpp:160-165)
            ui.horizontal(|ui| {
                if ui.button("적용").clicked() {
                    app.modals.dem_applied_mode_idx = app.modals.dem_mode_idx;
                    app.modals.dem_applied_preset_idx = app.modals.dem_preset_idx;
                    app.modals.dem_applied_relief_enabled = app.modals.dem_relief_enabled;
                    app.modals.dem_applied_exaggeration = app.modals.dem_exaggeration;
                    app.modals.dem_applied_relief_strength_pct =
                        app.modals.dem_relief_strength_pct;
                    app.home_map
                        .real_survey
                        .layer_visible
                        .insert("지형 음영".to_owned(), true);
                    app.home_map.draw_guide_message = Some(format!(
                        "DEM 표현 적용됨 (모드: {}, 수직과장: {:.1}배, 음영 세기: {}%)",
                        ["등고", "음영", "고도"][app.modals.dem_mode_idx.min(2)],
                        app.modals.dem_exaggeration,
                        app.modals.dem_relief_strength_pct
                    ));
                    app.modals.dem_status_text = "적용했습니다. 색띠로 표고를 확인하세요. 음영은 지형 방향에 따라 밝기를 바꿉니다.".to_owned();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("취소").clicked() {
                        app.modals.active = None;
                    }
                    let ok_btn = egui::Button::new(
                        egui::RichText::new("확인")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(33, 33, 31))
                    .min_size(Vec2::new(72.0, 30.0));
                    if ui.add(ok_btn).clicked() {
                        app.modals.dem_applied_mode_idx = app.modals.dem_mode_idx;
                        app.modals.dem_applied_preset_idx = app.modals.dem_preset_idx;
                        app.modals.dem_applied_relief_enabled =
                            app.modals.dem_relief_enabled;
                        app.modals.dem_applied_exaggeration = app.modals.dem_exaggeration;
                        app.modals.dem_applied_relief_strength_pct =
                            app.modals.dem_relief_strength_pct;
                        app.home_map
                            .real_survey
                            .layer_visible
                            .insert("지형 음영".to_owned(), true);
                        app.home_map.draw_guide_message = Some(format!(
                            "DEM 표현 확인 완료 (모드: {}, 수직과장: {:.1}배, 음영 세기: {}%)",
                            ["등고", "음영", "고도"][app.modals.dem_mode_idx.min(2)],
                            app.modals.dem_exaggeration,
                            app.modals.dem_relief_strength_pct
                        ));
                          // KaDemClassDialog.cpp:104-106,131
                          app.modals.dem_status_text = "적용했습니다. 색띠로 표고를 확인하세요. 음영은 지형 방향에 따라 밝기를 바꿉니다.".to_owned();
                        app.modals.active = None;
                    }
                });
            });
        });
}
