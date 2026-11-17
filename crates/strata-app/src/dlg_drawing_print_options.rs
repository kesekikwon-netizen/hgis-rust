//! Responsive form, target/cpp-ref/src/app/KaPrintDialog.cpp:126-166 and KaPrintDialogPlate.cpp:84-125.
use crate::app_state::StrataAppState;
use eframe::egui::{self, Vec2};
use strata_layout::sheet_composer::SheetPaperSize;

pub(crate) fn render_print_options(app: &mut StrataAppState, ui: &mut egui::Ui) {
    ui.strong("프린터");
    ui.add_enabled(false, egui::Button::new("프린터 선택").min_size(Vec2::new(ui.available_width(), 34.0)))
        .on_disabled_hover_text("아직 Rust 판에서 연결하지 않았습니다 (지시 UI3_PRINT)");
    ui.add_space(16.0);
    ui.strong("방법");
    ui.radio_value(&mut app.modals.print_single_sheet, false, "여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)");
    ui.radio_value(&mut app.modals.print_single_sheet, true, "한 장에 인쇄 (용지에 들어가면 실제 크기로)");
    ui.add_space(16.0);
    ui.strong("용지");
    ui.horizontal_wrapped(|ui| {
        for paper in [SheetPaperSize::A4, SheetPaperSize::A3] {
            if ui.add_sized([72.0, 32.0], egui::Button::new(paper.label_ko()).selected(app.modals.print_paper == paper)).clicked() {
                app.modals.print_paper = paper;
            }
        }
    });
    if !app.modals.print_single_sheet {
        ui.horizontal(|ui| {
            ui.label("겹침");
            ui.add(egui::DragValue::new(&mut app.modals.print_overlap_mm).range(0.0..=30.0).speed(1.0).suffix(" mm"));
        });
    }
    ui.add_space(16.0);
    ui.strong("해상도");
    ui.horizontal_wrapped(|ui| {
        for (dpi, label) in [(300, "300 DPI (보통)"), (600, "600 DPI (고정밀 제출용)")] {
            if ui.add_sized([144.0, 32.0], egui::Button::new(label).selected(app.modals.print_dpi == dpi)).clicked() {
                app.modals.print_dpi = dpi;
            }
        }
    });
    if !app.modals.print_single_sheet {
        ui.checkbox(&mut app.modals.print_marks, "자르는 점선·장 번호·5 cm 확인선 넣기");
        ui.checkbox(&mut app.modals.print_overview, "맨 앞에 붙이는 순서 안내 한 장 넣기");
    }
    ui.add_space(16.0);
    ui.separator();
    ui.label(format!("도면 · {} × {} mm", app.drawing.sheet.width_mm, app.drawing.sheet.height_mm));
    ui.label(format!("축척 · 1:{}", app.drawing.sheet.scale_denominator));
}
