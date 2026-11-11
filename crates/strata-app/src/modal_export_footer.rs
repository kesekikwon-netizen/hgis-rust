//! `render_export_submit_package` 의 하단 안내·버튼 구간.
//!
//! `modal_export.rs` 가 300줄을 넘어 한 함수의 구간을 옮겼다.
//! 실행 순서는 호출부가 보장한다.
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
                    "레이어 하나만 바꾸는 기능은 아직 Rust 판에 없습니다. 「제출 꾸러미 만들기」가 모든 조사 레이어를 EPSG:5179 SHP 로 내보냅니다."
                        .to_owned(),
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
                    // C++ `KaSubmitFlow` — 제출 폴더를 고른다. 취소하면 아무 것도 하지 않는다.
                    let picked = rfd::FileDialog::new()
                        .set_title("제출 꾸러미를 만들 빈 폴더")
                        .set_directory(&app.home_map.real_survey.survey_dir)
                        .pick_folder();
                    if let Some(out_dir) = picked {
                        app.modals.last_export_status =
                            match app.export_submit_package_to(&out_dir, findings) {
                                Ok(done) => format!("{done} ({status_txt})"),
                                Err(why) => why,
                            };
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
