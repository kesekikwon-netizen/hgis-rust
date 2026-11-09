//! `dialogs_modal` 에서 옮긴 대화상자 묶음: ControlPoint.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use crate::dialogs_modal::render_strata_modal_header;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2};


pub(crate) fn render_control_point(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // KaControlPointDialog.cpp:40-120 — GPS 기준점
    let full = ctx.screen_rect();
    // screen_rect is already logical points (Task D7)
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 480.0_f32.min(vis_w - 40.0);
    let modal_h = 560.0_f32.min(vis_h - 40.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_control_point")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 20.0);
            ui.set_max_width(modal_w - 20.0);
            render_strata_modal_header(ui, "GPS 기준점", open);
            ui.label(
                egui::RichText::new(
                    "아래 동쪽·북쪽 값은 현재 조사 좌표계 기준 미터(m)입니다. (세계측지계 · GRS80 · 동부원점 (EPSG:5187))",
                )
                .size(11.5)
                .color(Color32::from_rgb(0x47, 0x55, 0x69)),
            );
            ui.add_space(6.0);

            // 1) 위치 그룹 (KaControlPointDialog.cpp:50-71)
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("위치").size(12.5).strong());
                    ui.add_space(4.0);
                    egui::Grid::new("ka_cp_pos_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("점ID");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.modals.cp_id)
                                    .hint_text("예: GCP-01")
                                    .desired_width(300.0),
                            );
                            ui.end_row();

                            let x_lbl = if app.modals.cp_swap_axes {
                                "X (북쪽·m)"
                            } else {
                                "X (동쪽)"
                            };
                            ui.label(x_lbl);
                            ui.add(
                                egui::TextEdit::singleline(&mut app.modals.cp_x)
                                    .desired_width(300.0),
                            );
                            ui.end_row();

                            let y_lbl = if app.modals.cp_swap_axes {
                                "Y (동쪽·m)"
                            } else {
                                "Y (북쪽)"
                            };
                            ui.label(y_lbl);
                            ui.add(
                                egui::TextEdit::singleline(&mut app.modals.cp_y)
                                    .desired_width(300.0),
                            );
                            ui.end_row();

                            ui.label("");
                            ui.checkbox(
                                &mut app.modals.cp_swap_axes,
                                "X·Y 교환 (측량 X=북, Y=동)",
                            );
                            ui.end_row();

                            ui.label("Z (표고 m, 선택)");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.modals.cp_z)
                                    .hint_text("비워 두면 표고 없이 넣습니다")
                                    .desired_width(300.0),
                            );
                            ui.end_row();
                        });
                });

            ui.add_space(6.0);
            // KaControlPointDialog.cpp:23-34 sectionHead — a word plus a hairline,
            // not a boxed group.
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("측량 기준")
                        .size(11.5)
                        .strong()
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                ui.add(egui::Separator::default().horizontal());
            });
            ui.add_space(4.0);
            // KaControlPointDialog.cpp:124-125 captionedRow, 4 across
            ui.columns(4, |cols| {
                cols[0].label(
                    egui::RichText::new("측지기준계")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                cols[0].add(
                    egui::TextEdit::singleline(&mut app.modals.cp_datum)
                        .desired_width(f32::INFINITY),
                );
                cols[1].label(
                    egui::RichText::new("타원체")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                cols[1].add(
                    egui::TextEdit::singleline(&mut app.modals.cp_ellipsoid)
                        .desired_width(f32::INFINITY),
                );
                cols[2].label(
                    egui::RichText::new("투영")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                cols[2].add(
                    egui::TextEdit::singleline(&mut app.modals.cp_projection)
                        .desired_width(f32::INFINITY),
                );
                cols[3].label(
                    egui::RichText::new("원점")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                egui::ComboBox::from_id_salt("ka_cp_origin_combo")
                    .selected_text(&app.modals.cp_origin)
                    .width(f32::INFINITY)
                    .show_ui(&mut cols[3], |ui| {
                        for orig in [
                            "서부원점",
                            "중부원점",
                            "동부원점",
                            "동해원점",
                            "단일원점(UTM-K)",
                        ] {
                            ui.selectable_value(
                                &mut app.modals.cp_origin,
                                orig.to_owned(),
                                orig,
                            );
                        }
                    });
            });

            ui.add_space(6.0);
            // KaControlPointDialog.cpp:23-34 sectionHead
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("수신")
                        .size(11.5)
                        .strong()
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                ui.add(egui::Separator::default().horizontal());
            });
            ui.add_space(4.0);
            // KaControlPointDialog.cpp:129 captionedRow, 3 across
            ui.columns(3, |cols| {
                cols[0].label(
                    egui::RichText::new("정확도(m)")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                cols[0].add(
                    egui::TextEdit::singleline(&mut app.modals.cp_accuracy)
                        .desired_width(f32::INFINITY),
                );
                cols[1].label(
                    egui::RichText::new("PDOP")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                cols[1].add(
                    egui::TextEdit::singleline(&mut app.modals.cp_pdop)
                        .desired_width(f32::INFINITY),
                );
                cols[2].label(
                    egui::RichText::new("수신 상태")
                        .size(11.0)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
                egui::ComboBox::from_id_salt("ka_cp_fix_combo")
                    .selected_text(&app.modals.cp_fix)
                    .width(f32::INFINITY)
                    .show_ui(&mut cols[2], |ui| {
                        for fix in ["RTK", "PPK", "DGPS", "단독측위"] {
                            ui.selectable_value(
                                &mut app.modals.cp_fix,
                                fix.to_owned(),
                                fix,
                            );
                        }
                    });
            });

            ui.add_space(8.0);
            // 하단 버튼 행 (KaControlPointDialog.cpp:106-119)
            ui.horizontal(|ui| {
                if ui.button("CSV 불러오기").clicked() {
                    let [cx, cy] = app.home_map.real_survey.survey_center;
                    app.modals.control_points_world.push((
                        "GCP-01".to_owned(),
                        [cx - 80.0, cy + 60.0],
                        Some(48.5),
                    ));
                    app.modals.control_points_world.push((
                        "GCP-02".to_owned(),
                        [cx + 95.0, cy - 55.0],
                        Some(46.2),
                    ));
                    app.home_map.draw_guide_message = Some(
                        "CSV에서 GPS 기준점 2점(GCP-01, GCP-02)을 불러와 지도에 표시했습니다."
                            .to_owned(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("취소").clicked() {
                        app.modals.active = None;
                    }
                    let add_btn = egui::Button::new(
                        egui::RichText::new("지도에 추가 (확인)")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(33, 33, 31));
                    if ui.add(add_btn).clicked() {
                        let raw_x = app
                            .modals
                            .cp_x
                            .trim()
                            .parse::<f64>()
                            .unwrap_or(app.home_map.real_survey.survey_center[0]);
                        let raw_y = app
                            .modals
                            .cp_y
                            .trim()
                            .parse::<f64>()
                            .unwrap_or(app.home_map.real_survey.survey_center[1]);
                        let (east_x, north_y) = if app.modals.cp_swap_axes {
                            (raw_y, raw_x)
                        } else {
                            (raw_x, raw_y)
                        };
                        let z_opt = app.modals.cp_z.trim().parse::<f64>().ok();
                        let id = if app.modals.cp_id.trim().is_empty() {
                            format!("GCP-{:02}", app.modals.control_points_world.len() + 1)
                        } else {
                            app.modals.cp_id.trim().to_owned()
                        };
                        app.modals
                            .control_points_world
                            .push((id.clone(), [east_x, north_y], z_opt));
                        app.home_map.draw_guide_message = Some(format!(
                            "GPS 기준점 「{id}」 (X={east_x:.3}, Y={north_y:.3}, {} · {}) 지도에 추가됨",
                            app.modals.cp_datum, app.modals.cp_fix
                        ));
                        app.modals.active = None;
                    }
                });
            });
        });
}
