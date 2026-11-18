//! Paper dialog draft and acceptance, target/cpp-ref/src/app/KaPaperDialog.cpp:43-198.
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::render_strata_modal_header;
use crate::modal_types::ModalDialogController;
use eframe::egui::{self, Color32, Vec2};
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

#[path = "modal_paper_controls.rs"]
mod controls;
#[path = "modal_paper_layout.rs"]
mod layout;
pub(crate) use layout::{keyboard_entry as paper_keyboard_entry, note_trigger as note_paper_trigger, Trigger as PaperTrigger};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaperChoice {
    A4,
    A3,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PaperDialogDraft {
    choice: PaperChoice,
    orientation: SheetOrientation,
    width_mm: f64,
    height_mm: f64,
    last_pass: u64,
    origin: Option<PaperTrigger>,
}

impl PaperDialogDraft {
    // Preserve the C++ default and dimensions; layout is GUI points, these values are millimetres.
    // target/cpp-ref/src/app/KaPaperDialog.cpp:64-66,80-100,161-179.
    fn new_default(pass: u64, origin: Option<PaperTrigger>) -> Self {
        Self {
            choice: PaperChoice::A4,
            orientation: SheetOrientation::Portrait,
            width_mm: DrawingSheet::A4_PORTRAIT_WIDTH_MM,
            height_mm: DrawingSheet::A4_PORTRAIT_HEIGHT_MM,
            last_pass: pass,
            origin,
        }
    }
    fn apply_preset(&mut self) {
        let size = match self.choice {
            PaperChoice::A4 => Some(SheetPaperSize::A4),
            PaperChoice::A3 => Some(SheetPaperSize::A3),
            PaperChoice::Custom => None,
        };
        if let Some(paper) = size {
            (self.width_mm, self.height_mm) = DrawingSheet::preset_dimensions_mm(paper, self.orientation);
        }
    }
    fn scale_note(&self) -> &'static str {
        let orientation = if self.choice == PaperChoice::A4 && self.orientation == SheetOrientation::Landscape {
            SheetOrientation::Landscape
        } else {
            SheetOrientation::Portrait
        };
        ModalDialogController::paper_scale_note(SheetPaperSize::A4, orientation)
    }
}

pub(crate) fn render_paper_setup(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let draft_id = egui::Id::new("ka_paper_dialog_draft");
    let pass = ctx.cumulative_pass_nr();
    let mut draft = match layout::take_entry(ctx) {
        Some(layout::Entry::Clicked(source)) => PaperDialogDraft::new_default(pass, Some(source)),
        Some(layout::Entry::Keyboard) => PaperDialogDraft::new_default(pass, None),
        None => ctx
            .data(|data| data.get_temp::<PaperDialogDraft>(draft_id))
            .filter(|draft| pass <= draft.last_pass.saturating_add(1))
            .unwrap_or_else(|| PaperDialogDraft::new_default(pass, None)),
    };
    draft.last_pass = pass;
    let placement = layout::Placement::new(ctx, draft.origin);
    let mut clear_draft = false;
    let frame = egui::Frame::popup(&ctx.style()).corner_radius(10).inner_margin(12);
    let (fill, edge) = (frame.fill, frame.stroke);
    let response = egui::Modal::new(egui::Id::new("ka_modal_paper_setup"))
        .area(placement.area(egui::Id::new("ka_modal_paper_setup")))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width((placement.width - 26.0).max(1.0));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            let top = ui.cursor().top();
            render_strata_modal_header(ui, "용지 설정", open);
            let header = ui.cursor().top() - top;
            let body_height = (placement.height_limit - 26.0 - header - 58.0).max(64.0);
            egui::ScrollArea::vertical().id_salt("paper-settings-body").max_height(body_height).auto_shrink([false, true]).show(ui, |ui| {
                controls::choices(ui, &mut draft);
                ui.add_space(4.0);
                controls::dimensions(ui, &mut draft, app.paper_look);
                ui.add_space(4.0);
                ui.label(egui::RichText::new(draft.scale_note()).size(12.5).color(ui.visuals().text_color()));
            });
            ui.add_space(2.0);
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let width = ((ui.available_width() - 56.0) / 3.0).max(80.0);
                if ui.add_sized([width, controls::HEIGHT], controls::button("취소", width)).clicked() {
                    app.modals.active = None;
                    clear_draft = true;
                }
                let ok = egui::Button::new(egui::RichText::new("용지 만들기").size(14.0).color(Color32::WHITE))
                    .min_size(Vec2::new(width, controls::HEIGHT))
                    .corner_radius(8)
                    .fill(Color32::from_rgb(181, 87, 58))
                    .stroke(egui::Stroke::NONE);
                if ui.add_sized([width, controls::HEIGHT], ok).clicked() && *open {
                    // Existing accept handlers, target/cpp-ref/src/app/MainWindowExport.cpp:216-255,
                    // KaDrawingStudio.cpp:447-454,555-559. Never change sheet data while editing the draft.
                    let was_open = app.open_studio_tabs.contains(&MainTab::Drawing);
                    app.modals.custom_paper_mode = draft.choice == PaperChoice::Custom;
                    match draft.choice {
                        PaperChoice::A4 => app.drawing.set_paper_and_orientation(SheetPaperSize::A4, draft.orientation),
                        PaperChoice::A3 => app.drawing.set_paper_and_orientation(SheetPaperSize::A3, draft.orientation),
                        PaperChoice::Custom => {
                            app.drawing.sheet.reset_paper_mm(draft.width_mm, draft.height_mm);
                            app.drawing.last_findings = ChecklistEngine::evaluate_sheet(&app.drawing.sheet, SubmitCrs::Epsg5179, true);
                        }
                    }
                    if was_open {
                        app.sheet_paper_changed();
                    }
                    app.open_studio_tab(MainTab::Drawing);
                    app.hide_sub_tools();
                    app.home_map.draw_guide_message = Some(if was_open {
                        let (w, h) = (app.drawing.sheet.width_mm.round() as i64, app.drawing.sheet.height_mm.round() as i64);
                        format!("용지 전환 완료: {w}×{h} mm — 기존 지도 범위와 배치가 유지되었습니다.")
                    } else {
                        app.drawing.sheet_center = None;
                        StrataAppState::DRAWING_STUDIO_WELCOME.to_owned()
                    });
                    app.modals.active = None;
                    clear_draft = true;
                }
            });
        });
    placement.paint_connection(&response.response, fill, edge);
    // Outside clicks are consumed, not accepted/cancelled, as QDialog::exec (KaPaperDialog.cpp:193-198).
    if response.is_top_modal && !response.any_popup_open && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        *open = false;
    }
    if clear_draft || !*open {
        ctx.data_mut(|data| data.remove::<PaperDialogDraft>(draft_id));
    } else {
        ctx.data_mut(|data| data.insert_temp(draft_id, draft));
    }
}
