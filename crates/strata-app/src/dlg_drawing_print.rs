//! App print tab. Owner 2026-11-17; target/cpp-ref/src/app/KaPrintDialogPlate.cpp:77-125.
use crate::app_state::StrataAppState;
use eframe::egui::{self, Vec2};
#[path = "dlg_drawing_print_actions.rs"]
mod dlg_drawing_print_actions;
#[path = "dlg_drawing_print_options.rs"]
mod dlg_drawing_print_options;
#[path = "dlg_drawing_print_preview.rs"]
mod dlg_drawing_print_preview;
#[path = "print_device.rs"]
pub(crate) mod print_device;
#[path = "print_job.rs"]
pub(crate) mod print_job;
#[path = "print_plan.rs"]
pub(crate) mod print_plan;
#[path = "print_state.rs"]
pub(crate) mod print_state;

pub(crate) fn render_drawing_print(app: &mut StrataAppState, ui: &mut egui::Ui) {
    app.printing.ensure_catalog();
    app.printing.poll_catalog();
    if matches!(app.printing.catalog, print_state::CatalogState::Loading(_)) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    ui.spacing_mut().interact_size.y = 30.0;
    egui::TopBottomPanel::top("print_tab_heading").show_inside(ui, |ui| {
        ui.add_space(8.0);
        ui.heading("도면 인쇄");
        ui.label(&app.drawing.title_ko);
        ui.add_space(8.0);
    });
    egui::TopBottomPanel::bottom("print_tab_actions").show_inside(ui, |ui| {
        ui.add_space(8.0);
        dlg_drawing_print_actions::render_print_actions(app, ui);
        ui.add_space(8.0);
    });
    egui::CentralPanel::default().show_inside(ui, |ui| {
        if ui.available_width() >= 600.0 {
            let height = ui.available_height();
            let options_width = (ui.available_width() * 0.4).clamp(230.0, 330.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(Vec2::new(options_width, height), egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical().id_salt("print_options").auto_shrink([false, false]).show(ui, |ui| {
                        dlg_drawing_print_options::render_print_options(app, ui);
                    });
                });
                ui.separator();
                dlg_drawing_print_preview::render_print_preview(app, ui, height);
            });
        } else {
            egui::ScrollArea::vertical().id_salt("print_compact").auto_shrink([false, false]).show(ui, |ui| {
                dlg_drawing_print_preview::render_print_preview(app, ui, 220.0);
                ui.add_space(12.0);
                dlg_drawing_print_options::render_print_options(app, ui);
            });
        }
    });
}
