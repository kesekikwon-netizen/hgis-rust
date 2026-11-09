//! `dialogs_download_print` 에서 옮긴 「CadImportNotice」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Vec2};

use crate::dialogs_modal::render_strata_modal_header;

/// `KaCadImportNotice.cpp` — 끌어 놓은 CAD 도면의 좌표계 확인 안내 창.
pub(crate) fn render_cad_import_notice(
    app: &mut crate::app_state::StrataAppState,
    ctx: &egui::Context,
    mut open: &mut bool,
) {
    let full = ctx.screen_rect();
    let w = 460.0_f32.min(full.width() - 32.0);
    let h = 260.0_f32.min(full.height() - 32.0);
    let pos = Pos2::new(
        ((full.width() - w) * 0.5).max(16.0),
        ((full.height() - h) * 0.5).max(16.0),
    );
    egui::Window::new("ka_modal_cad_notice")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(pos)
        .fixed_size([w, h])
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "CAD 도면 좌표계 확인", open);
            ui.label(
                egui::RichText::new(format!("「{}」", app.modals.cad_import_path))
                    .size(11.5)
                    .monospace()
                    .color(Color32::from_rgb(80, 79, 74)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "도면에 적힌 좌표가 어느 좌표계인지 확인하고 지도에 올립니다. 도면에 좌표계가 적혀 있지 않으면 두 점을 찍어 맞추세요.",
                )
                .size(12.0),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("도면 좌표계").size(12.0).strong());
                ui.label(
                    egui::RichText::new(app.work_crs.auth_id())
                        .size(12.0)
                        .monospace(),
                );
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let add_btn = egui::Button::new(
                    egui::RichText::new("이 좌표계로 지도에 올리기")
                        .color(Color32::WHITE)
                        .strong()
                        .size(12.5),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(170.0, 30.0));
                if ui
                    .add(add_btn)
                    .on_hover_text(
                        "고른 좌표계로 도면을 지도에 올립니다. 나중에 정합에서 미세 조정할 수 있습니다.",
                    )
                    .clicked()
                {
                    app.active_tab = crate::app_state::MainTab::Map;
                    app.home_map.draw_guide_message = Some(format!(
                        "CAD 도면을 {} 좌표계로 지도에 올렸습니다.",
                        app.work_crs.auth_id()
                    ));
                    app.modals.active = None;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("두 점으로 맞추기").size(12.0),
                        )
                        .min_size(Vec2::new(130.0, 30.0)),
                    )
                    .on_hover_text("도면 위 두 점과 지도 위 두 점을 찍어 맞춥니다.")
                    .clicked()
                {
                    app.active_tab = crate::app_state::MainTab::Map;
                    app.home_map.draw_guide_message =
                        Some("두 점으로 맞추기 — 도면 위 점 하나를 찍으세요.".to_owned());
                    app.modals.active = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("취소").size(12.0))
                                .min_size(Vec2::new(70.0, 30.0)),
                        )
                        .clicked()
                    {
                        app.modals.active = None;
                    }
                });
            });
        });
}
