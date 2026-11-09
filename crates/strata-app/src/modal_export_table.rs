//! `render_export_submit_package` 의 27개 검수 규칙 표.
//!
//! `modal_export.rs` 가 300줄을 넘어 한 함수의 구간을 옮겼다.
//! 실행 순서는 호출부가 보장한다.
use crate::app_state::{MainTab, StrataAppState};
use crate::modal_types::ModalDialogController;
use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{self, Color32, Stroke};

pub(crate) fn paint_export_submit_table(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    _open: &mut bool,
    modal_w: f32,
    err_cnt: usize,
    results: &[strata_check_export::checklist_engine::CheckResult],
    status_txt: &str,
) {
        // KaSubmitDialog.cpp:58 — 헤더 ("항목", "고치는 법", "위치 보기")
        egui::Frame::NONE
            .fill(Color32::from_rgb(232, 229, 220))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("위치 이동 · 순번 · JSON 규칙 ID (27개)")
                            .size(11.0)
                            .strong(),
                    );
                    ui.add_space(45.0);
                    ui.label(
                        egui::RichText::new("심각도 · 검수 항목 및 고치는 법 (근거)")
                            .size(11.0)
                            .strong(),
                    );
                });
            });

        // KaSubmitDialog.cpp:153-225 — 27개 JSON 규칙 순서대로 전체 표시 + 규칙별 [위치 보기] 버튼
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (idx, r) in results.iter().enumerate() {
                    let row_bg = if !r.passed && r.severity == "error" {
                        Color32::from_rgb(0xF3, 0xDE, 0xDA)
                    } else if !r.passed {
                        Color32::from_rgb(255, 251, 235)
                    } else if idx % 2 == 0 {
                        Color32::from_rgb(252, 251, 247)
                    } else {
                        Color32::WHITE
                    };
                    egui::Frame::NONE
                        .fill(row_bg)
                        .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                        .inner_margin(egui::Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.set_max_width(modal_w - 44.0);
                            ui.horizontal(|ui| {
                                if ui.small_button("위치 보기").clicked() {
                                    app.home_map.viewport.center_xy =
                                        app.home_map.real_survey.survey_center;
                                    app.home_map.viewport.meters_per_pixel = 1.8;
                                    app.active_tab = MainTab::Map;
                                    app.home_map.draw_guide_message = Some(format!(
                                        "「{}」 위치로 이동했습니다: {}",
                                        r.id, r.message_ko
                                    ));
                                    app.modals.active = None;
                                }
                                // KaSubmitDialog.cpp:180-200 — 실패 항목의 바로가기 버튼
                                if !r.passed {
                                    if let Some(lbl) =
                                        ModalDialogController::submit_action_label(&r.action)
                                    {
                                        if ui.small_button(lbl).clicked() {
                                            let mut next_modal = None;
                                            match r.action.as_str() {
                                                "open_layout" => {
                                                    app.active_tab = MainTab::Drawing;
                                                }
                                                "draw_survey_area"
                                                | "draw_feature" => {
                                                    app.active_tab = MainTab::Map;
                                                    app.active_tool =
                                                        strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
                                                }
                                                "add_control_point" => {
                                                    app.active_tab = MainTab::Map;
                                                    next_modal = Some(
                                                        ActiveModalDialog::ControlPoint,
                                                    );
                                                }
                                                "edit_attributes" => {
                                                    app.active_tab = MainTab::Map;
                                                    app.right_inspector.active_card =
                                                        crate::right_inspector::InspectorCardTab::LayerStyleCard;
                                                }
                                                "select_tool" => {
                                                    app.active_tab = MainTab::Map;
                                                    app.active_tool =
                                                        strata_canvas2d::map_tools::MapToolKind::Select;
                                                }
                                                "set_work_crs" => {
                                                    next_modal = Some(
                                                        ActiveModalDialog::AccountSettings,
                                                    );
                                                }
                                                _ => {
                                                    app.active_tab = MainTab::Map;
                                                }
                                            }
                                            app.home_map.draw_guide_message = Some(format!(
                                                "{} — 「{}」", lbl, r.id
                                            ));
                                            app.modals.active = next_modal;
                                        }
                                    }
                                }
                                let (badge, col) = if r.passed {
                                    (format!("[통과·{}]", r.severity), Color32::from_rgb(21, 128, 61))
                                } else if r.severity == "error" {
                                    ("[오류·error]".to_owned(), Color32::from_rgb(185, 28, 28))
                                } else {
                                    ("[주의·warn]".to_owned(), Color32::from_rgb(180, 83, 9))
                                };
                                ui.colored_label(
                                    col,
                                    egui::RichText::new(badge).strong().size(10.5),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:02}. {}",
                                        idx + 1,
                                        r.id
                                    ))
                                    .monospace()
                                    .strong()
                                    .size(11.0)
                                    .color(Color32::from_rgb(28, 25, 23)),
                                );
                                let body_txt = if r.passed {
                                    format!("{} ({})", r.message_ko, r.basis_ko)
                                } else {
                                    format!("{} — {}", r.message_ko, r.fix_ko)
                                };
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(body_txt)
                                            .size(11.0)
                                            .color(Color32::from_rgb(45, 44, 40)),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                }
            });

        ui.separator();
        // KaSubmitDialogPlate.cpp:66 — 패키지 구성 안내
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(
                    "EPSG:5179 SHP · 조사도면.pdf · MANIFEST.sha256 · README_submit.txt",
                )
                .monospace()
                .size(11.5)
                .color(Color32::from_rgb(75, 74, 70)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(status_txt)
                        .size(12.0)
                        .strong()
                        .color(if err_cnt > 0 {
                            Color32::from_rgb(185, 28, 28)
                        } else {
                            Color32::from_rgb(21, 128, 61)
                        }),
                );
            });
        });
        ui.add_space(4.0);
}
