//! `dialogs_download_print` 에서 옮긴 「TrenchGenerator」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.
//! 300줄 규칙에 맞춰 앞부분(`dlg_trench_generator_controls`)과 가운데 요약
//! (`dlg_trench_generator_summary`)을 나눴다. `crate::dlg_trench_generator::render_trench_generator`
//! 경로와 위젯 순서는 그대로다.

use eframe::egui::{self, Color32, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

#[path = "dlg_trench_generator_controls.rs"]
mod dlg_trench_generator_controls;
#[path = "dlg_trench_generator_summary.rs"]
mod dlg_trench_generator_summary;

use dlg_trench_generator_controls::render_trench_controls;
use dlg_trench_generator_summary::render_trench_summary;

pub(crate) fn render_trench_generator(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
) {
    egui::Window::new("ka_modal_trench")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [60.0, 20.0])
        .default_width(380.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "시굴격자 속성 - Strata", open);
            ui.checkbox(
                &mut app.modals.trench_auto_place,
                egui::RichText::new("지금 그린(또는 선택한) 조사구역에 자동 배치").size(13.0),
            )
            .on_hover_text(
                "조사구역이 여러 개면 선택한 곳만, 선택이 없으면 마지막에 그린 곳만 깝니다.",
            );

            let (manual, kind) = render_trench_controls(app, ui);
            render_trench_summary(app, ui, manual, kind);
            // KaTrenchDialog.cpp:137-154 — btnRow (하단 실행 버튼)
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                let place_btn = egui::Button::new(
                    egui::RichText::new("구역에 깔기")
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(120.0, 30.0));
                if ui
                    .add(place_btn)
                    .on_hover_text(
                        "조사구역 위에 격자를 다시 깝니다. 깐 뒤 맵에서 끌어 옮기세요.",
                    )
                    .clicked()
                {
                    app.home_map
                        .real_survey
                        .layer_visible
                        .insert("시굴격자".to_owned(), true);
                    app.home_map.draw_guide_message = Some(format!(
                        "시굴격자 {}개를 조사구역 위에 배치했습니다 (방위 {:.0}°).",
                        app.home_map.real_survey.trenches.len(),
                        app.modals.trench_azimuth_deg
                    ));
                }
                if ui
                    .button("맵에 찍기")
                    .on_hover_text("맵을 한 번 찍어 그 점을 원점으로 놓습니다.")
                    .clicked()
                {
                    app.modals.trench_auto_place = false;
                    app.home_map.draw_guide_message = Some(
                        "맵을 한 번 찍어 시굴격자 원점을 지정하세요.".to_owned(),
                    );
                    *open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        *open = false;
                    }
                });
            });
        });
}
