//! Paper choice widgets, target/cpp-ref/src/app/KaPaperDialog.cpp:48-175.
use super::{PaperChoice, PaperDialogDraft};
use crate::modal_types::ModalDialogController;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_layout::sheet_composer::SheetOrientation;

pub(super) const HEIGHT: f32 = 40.0;
const LABEL: f32 = 36.0;

pub(super) fn button(label: &str, width: f32) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).size(14.0)).min_size(Vec2::new(width, HEIGHT)).corner_radius(8)
}

fn choice(ui: &mut egui::Ui, text: &str, selected: bool, width: f32) -> egui::Response {
    let fill = if selected { Color32::from_rgb(237, 228, 212) } else { ui.visuals().window_fill() };
    let stroke = if selected { Stroke::new(1.5, Color32::from_rgb(181, 87, 58)) } else { ui.visuals().window_stroke() };
    ui.add_sized([width, HEIGHT], button(text, width).fill(fill).stroke(stroke))
}

pub(super) fn choices(ui: &mut egui::Ui, draft: &mut PaperDialogDraft) {
    let width = ((ui.available_width() - LABEL - 20.0) / 3.0).max(1.0);
    ui.horizontal(|ui| {
        ui.add_sized([LABEL, HEIGHT], egui::Label::new("용지"));
        ui.spacing_mut().item_spacing.x = 6.0;
        for (text, kind) in [("A4", PaperChoice::A4), ("A3", PaperChoice::A3), ("직접 입력", PaperChoice::Custom)] {
            if choice(ui, text, draft.choice == kind, width).clicked() {
                draft.choice = kind;
                draft.apply_preset();
            }
        }
    });
    ui.horizontal(|ui| {
        ui.add_sized([LABEL, HEIGHT], egui::Label::new("방향"));
        ui.spacing_mut().item_spacing.x = 6.0;
        for (text, orientation, size) in [("세로", SheetOrientation::Portrait, Vec2::new(12.0, 20.0)), ("가로", SheetOrientation::Landscape, Vec2::new(22.0, 13.0))] {
            let response = choice(ui, text, draft.orientation == orientation, width);
            if width >= 100.0 {
                let image = Rect::from_center_size(Pos2::new(response.rect.left() + 18.0, response.rect.center().y), size);
                ui.painter().rect_stroke(image, 2, Stroke::new(1.2, ui.visuals().text_color()), egui::StrokeKind::Inside);
            }
            if response.clicked() {
                draft.orientation = orientation;
                draft.apply_preset();
            }
        }
    });
}

pub(super) fn dimensions(ui: &mut egui::Ui, draft: &mut PaperDialogDraft, paper_look: bool) {
    let custom = draft.choice == PaperChoice::Custom;
    for (text, value) in [("가로", &mut draft.width_mm), ("세로", &mut draft.height_mm)] {
        ui.horizontal(|ui| {
            ui.add_sized([LABEL, 28.0], egui::Label::new(text));
            if custom || !paper_look {
                ui.add_enabled_ui(custom, |ui| {
                    ui.add_sized([ui.available_width(), HEIGHT], egui::DragValue::new(value).range(50.0..=2000.0).speed(1.0).fixed_decimals(1).suffix(" mm"));
                });
            } else {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::hover());
                let ink = ui.visuals().text_color();
                let font = egui::FontId::monospace(15.0);
                let number = ModalDialogController::plain_mm(*value);
                let number_width = ui.painter().layout_no_wrap(number.clone(), font.clone(), ink).size().x;
                let right = rect.right() - 30.0;
                ui.painter().text(Pos2::new(right, rect.center().y), egui::Align2::RIGHT_CENTER, number, font, ink);
                ui.painter().text(rect.right_center(), egui::Align2::RIGHT_CENTER, "mm", egui::FontId::proportional(12.5), ui.visuals().weak_text_color());
                // A bounded leader replaces the fixed text run (KaPaperDialog.cpp:111-130).
                let end = right - number_width - 12.0;
                if end > rect.left() {
                    ui.painter()
                        .extend(egui::Shape::dashed_line(&[rect.left_center(), Pos2::new(end, rect.center().y)], Stroke::new(1.0, ui.visuals().window_stroke().color), 1.0, 3.0));
                }
            }
        });
    }
}
