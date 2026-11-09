//! `dialogs_download_print` 에서 옮긴 「DrawingPrint」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.
//! 300줄 규칙에 맞춰 왼쪽 칸(`dlg_drawing_print_options`), 오른쪽 미리보기
//! (`dlg_drawing_print_preview`), 아래 단추(`dlg_drawing_print_actions`)로 나눴다.
//! `crate::dlg_drawing_print::render_drawing_print` 경로와 위젯 순서는 그대로다.

use eframe::egui;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

#[path = "dlg_drawing_print_actions.rs"]
mod dlg_drawing_print_actions;
#[path = "dlg_drawing_print_options.rs"]
mod dlg_drawing_print_options;
#[path = "dlg_drawing_print_preview.rs"]
mod dlg_drawing_print_preview;

use dlg_drawing_print_actions::render_print_actions;
use dlg_drawing_print_options::render_print_options;
use dlg_drawing_print_preview::render_print_preview;

pub(crate) fn render_drawing_print(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
) {
    egui::Window::new("ka_modal_drawing_print")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(760.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "도면 인쇄 - Strata", open);
            ui.columns(2, |cols| {
                let (paper_summary, scale_summary) =
                    render_print_options(app, &mut cols[0]);
                render_print_preview(&mut cols[1], &paper_summary, &scale_summary);
            });
            render_print_actions(app, ui);
        });
}
