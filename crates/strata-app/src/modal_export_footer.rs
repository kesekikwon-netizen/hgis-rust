//! `render_export_submit_package` 의 하단 안내·버튼 구간.
//!
//! `modal_export.rs` 가 300줄을 넘어 한 함수의 구간을 옮겼다.
//! 실행 순서는 호출부가 보장한다.
use strata_check_export::submit_package::export_submission_package;
use crate::app_state::{StrataAppState};
use eframe::egui::{self, Color32, Vec2};

pub(crate) fn paint_export_submit_footer(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    _open: &mut bool,
    findings: &[strata_check_export::checklist_engine::ChecklistFinding],
    status_txt: &str,
) {
        // KaSubmitDialog.cpp:85-105 — 하단 버튼 행 (다시 검수 / 레이어 5179 변환… / 제출 꾸러미 만들기… / 닫기)
        ui.horizontal(|ui| {
            if ui.button("다시 검수").clicked() {
                app.home_map.draw_guide_message = Some(status_txt.to_owned());
            }
            if ui.button("레이어 5179 변환…").clicked() {
                app.home_map.draw_guide_message = Some(
                    "선택한 레이어를 EPSG:5179 SHP 파일로 변환했습니다.".to_owned(),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("닫기").clicked() {
                    app.modals.active = None;
                }
                let export_btn = egui::Button::new(
                    egui::RichText::new("제출 꾸러미 만들기…")
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(160.0, 32.0));
                if ui.add(export_btn).clicked() {
                    let out_dir =
                        app.home_map.real_survey.survey_dir.join("제출꾸러미_5179");
                    if let Ok(m) = export_submission_package(&out_dir, findings) {
                        app.modals.last_export_status = format!(
                            "제출 꾸러미 생성 완료: {} (파일 {}종, {} — README_submit.txt 기록됨)",
                            out_dir.display(),
                            m.files_sha256.len(),
                            status_txt
                        );
                    }
                }
            });
        });
        if !app.modals.last_export_status.is_empty() {
            ui.add_space(4.0);
            ui.colored_label(
                Color32::from_rgb(21, 128, 61),
                &app.modals.last_export_status,
            );
        }
}
