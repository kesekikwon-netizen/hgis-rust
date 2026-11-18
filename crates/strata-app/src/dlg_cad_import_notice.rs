//! `dialogs_download_print` 에서 옮긴 「CadImportNotice」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Vec2};

use crate::dialogs_modal::render_strata_modal_header;

/// `KaCadImportNotice.cpp` — 끌어 놓은 CAD 도면의 좌표계 확인 안내 창.
pub(crate) fn render_cad_import_notice(
    app: &mut crate::app_state::StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
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
                    "도면에 적힌 좌표가 어느 좌표계인지 확인하고 지도에 올립니다. 다른 곳이면 「다른 위치로 바꾸기」를 누르세요.",
                )
                .size(12.0),
            );
            ui.add_space(8.0);
            // KaCadImportNotice.cpp:53-68 — candidates other than the one in use, then the two actions.
            let used_auth_id = app.work_crs.auth_id(); // stand-in for the placed id (`KaCadImport.cpp:166`)
            let notice = crate::cad_crs_notice::notice_for(std::path::Path::new(&app.modals.cad_import_path), used_auth_id, &[]);
            let mut picked = None;
            ui.horizontal(|ui| {
                crate::gui_shell::native_menu_button(ui, "다른 위치로 바꾸기", |ui| {
                    for line in &notice.menu {
                        if matches!(line.kind, crate::cad_crs_notice::CadNoticeKind::NoCoordinates) {
                            ui.separator();
                        }
                        if ui.button(&line.label).clicked() {
                            picked = Some(line.kind.clone());
                        }
                    }
                });
                if ui.add(egui::Button::new(egui::RichText::new(notice.align_label).size(12.0)).min_size(Vec2::new(110.0, 30.0))).clicked() {
                    picked = Some(crate::cad_crs_notice::CadNoticeKind::Align);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("취소").size(12.0)).min_size(Vec2::new(70.0, 30.0))).clicked() {
                        app.modals.active = None;
                    }
                });
            });
            if let Some(kind) = picked {
                app.apply_cad_notice(&kind);
            }
        });
}
