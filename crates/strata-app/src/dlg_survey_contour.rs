//! `dialogs_download_print` 에서 옮긴 「SurveyContour」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

pub(crate) fn render_survey_contour(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ka_modal_survey_contour")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([
            440.0_f32.min(ctx.screen_rect().width() - 32.0),
            290.0_f32.min(ctx.screen_rect().height() - 32.0),
        ])
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "측량 등고선 - Strata", open);
            ui.label(
                egui::RichText::new(
                    "엑셀·CSV·DXF의 측량 좌표(X, Y, Z)로 등고선과 표고점 레이어를 만듭니다.",
                )
                .size(12.0)
                .color(Color32::from_rgb(80, 78, 72)),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("좌표 파일"));
                ui.add_sized(
                    [264.0, 26.0],
                    egui::TextEdit::singleline(&mut app.modals.contour_file_path),
                );
                let _ = ui.add_sized([74.0, 26.0], egui::Button::new("찾기…"));
            });
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("등고선 간격"));
                ui.add(
                    egui::DragValue::new(&mut app.modals.contour_interval_m)
                        .speed(0.1)
                        .range(0.1..=100.0)
                        .suffix(" m"),
                );
                ui.label(
                    egui::RichText::new("(계곡선 5줄마다 굵게 표시)")
                        .size(11.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
            });
            ui.add_space(4.0);
            // KaSurveyContourDialog.cpp:31-55 축 순서 / 이상한 점 + 고급 설정
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("축 순서"));
                ui.checkbox(
                    &mut app.modals.contour_swap_axes,
                    "X를 북쪽, Y를 동쪽으로 읽기",
                );
            });
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("이상한 점"));
                ui.checkbox(&mut app.modals.contour_exclude_suspicious, "의심점 빼기");
            });
            let adv_label = if app.modals.contour_advanced_open {
                "▾ 고급 설정"
            } else {
                "▸ 고급 설정"
            };
            if ui.button(egui::RichText::new(adv_label).size(12.5)).clicked() {
                app.modals.contour_advanced_open = !app.modals.contour_advanced_open;
            }
            if app.modals.contour_advanced_open {
                const CRS_LABELS: [&str; 5] = [
                    "작업 좌표계와 같음 (EPSG:5187)",
                    "EPSG:5185 서부원점",
                    "EPSG:5186 중부원점",
                    "EPSG:5179 UTM-K",
                    "EPSG:4326 경위도 WGS84",
                ];
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("파일 좌표계"));
                    let idx = app.modals.contour_file_crs_idx.min(4);
                    if ui
                        .add_sized(
                            [260.0, 26.0],
                            egui::Button::new(format!("{}  ▼", CRS_LABELS[idx])),
                        )
                        .clicked()
                    {
                        app.modals.contour_file_crs_idx = (idx + 1) % 5;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("최대 삼각형 변"));
                    ui.add(
                        egui::DragValue::new(&mut app.modals.contour_max_edge_m)
                            .range(0.0..=1000.0)
                            .speed(1.0)
                            .suffix(" m"),
                    );
                    if app.modals.contour_max_edge_m == 0.0 {
                        ui.label(
                            egui::RichText::new("쓰지 않음")
                                .size(11.5)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("단절선"));
                    ui.checkbox(
                        &mut app.modals.contour_use_breaklines,
                        "높이가 있는 DXF 선을 단절선으로 쓰기",
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("계산 격자"));
                    ui.add(
                        egui::DragValue::new(&mut app.modals.contour_cell_size_m)
                            .range(0.0..=5.0)
                            .speed(0.05)
                            .suffix(" m"),
                    );
                    if app.modals.contour_cell_size_m == 0.0 {
                        ui.label(
                            egui::RichText::new("자동")
                                .size(11.5)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                });
            }
            ui.label(
                egui::RichText::new(
                    "DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다.",
                )
                .size(11.5)
                .color(Color32::from_rgb(95, 94, 88)),
            );
            ui.add_space(6.0);
            let line_est = ((23.7 / app.modals.contour_interval_m.max(0.1)).round() as usize)
                .clamp(1, 500);
            let summary = format!(
                "점 248개, 표고 24.5–48.2 m. 등고선 약 {line_est}줄 (간격 {:.1} m). 의심점 3개 제외.",
                app.modals.contour_interval_m
            );
            egui::Frame::NONE
                .fill(Color32::from_rgb(225, 240, 218))
                .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(summary)
                            .size(12.0)
                            .color(Color32::from_rgb(28, 72, 32)),
                    );
                });
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_sized([68.0, 30.0], egui::Button::new("취소")).clicked() {
                    app.modals.active = None;
                }
                let make_btn = egui::Button::new(
                    egui::RichText::new("등고선 만들기")
                        .color(Color32::WHITE)
                        .strong(),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(108.0, 30.0));
                if ui.add(make_btn).clicked() {
                    app.left_layers.ensure_domain_layer(
                        strata_types::schema::DomainLayerKey::SectionLine,
                    );
                    app.home_map.draw_guide_message = Some(format!(
                        "측량점 등고선 약 {}줄(간격 {:.1} m)과 표고점 248개를 생성했습니다.",
                        line_est, app.modals.contour_interval_m
                    ));
                    app.is_dirty = true;
                    app.modals.active = None;
                }
            });
        });
}
