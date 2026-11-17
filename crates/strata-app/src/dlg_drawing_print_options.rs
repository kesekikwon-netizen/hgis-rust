//! Responsive form, target/cpp-ref/src/app/KaPrintDialog.cpp:126-166 and KaPrintDialogPlate.cpp:84-125.
use super::print_state::{paper_for, CatalogState};
use crate::app_state::StrataAppState;
use eframe::egui;
use strata_layout::sheet_composer::SheetPaperSize;

pub(crate) fn render_print_options(app: &mut StrataAppState, ui: &mut egui::Ui) {
    ui.strong("프린터");
    let text = app.printing.selected.as_deref().unwrap_or("(설치된 프린터 없음)");
    egui::ComboBox::from_id_salt("print_device_choice").width(ui.available_width()).truncate().selected_text(text).show_ui(ui, |ui| {
        if let CatalogState::Ready(printers) = &app.printing.catalog {
            for printer in printers {
                ui.selectable_value(&mut app.printing.selected, Some(printer.name.clone()), &printer.name);
            }
        }
    });
    match &app.printing.catalog {
        CatalogState::Loading(_) => {
            ui.spinner();
        }
        CatalogState::Failed(why) => {
            ui.label(why);
        }
        CatalogState::Unrequested | CatalogState::Ready(_) => {}
    }
    if !matches!(app.printing.catalog, CatalogState::Loading(_)) && ui.small_button("새로고침").clicked() {
        app.printing.catalog = CatalogState::Unrequested;
        app.printing.ensure_catalog();
    }
    if app.printing.selected_printer().is_some_and(|p| p.file_output) {
        ui.label("파일");
        ui.add(egui::TextEdit::singleline(&mut app.printing.output_path).desired_width(f32::INFINITY).hint_text("도면.pdf"));
    }
    ui.add_space(16.0);
    ui.strong("방법");
    ui.radio_value(&mut app.printing.single_sheet, false, "여러 장으로 나눠 크게 인쇄 (붙여서 큰 도면)");
    ui.radio_value(&mut app.printing.single_sheet, true, "한 장에 인쇄 (용지에 들어가면 실제 크기로)");
    ui.add_space(16.0);
    ui.strong("용지");
    ui.horizontal_wrapped(|ui| {
        for paper in [SheetPaperSize::A4, SheetPaperSize::A3] {
            let supported = app.printing.selected_printer().is_none_or(|p| paper_for(p, paper).is_some());
            let button = egui::Button::new(paper.label_ko()).selected(app.printing.paper == paper).min_size(egui::vec2(72.0, 32.0));
            if ui.add_enabled(supported, button).clicked() {
                app.printing.paper = paper;
            }
        }
    });
    if !app.printing.single_sheet {
        ui.horizontal(|ui| {
            ui.label("겹침");
            ui.add(egui::DragValue::new(&mut app.printing.overlap_mm).range(0.0..=30.0).speed(1.0).suffix(" mm"));
        });
    }
    ui.add_space(16.0);
    ui.strong("해상도");
    ui.horizontal_wrapped(|ui| {
        for (dpi, label) in [(300, "300 DPI (보통)"), (600, "600 DPI (고정밀 제출용)")] {
            if ui.add_sized([144.0, 32.0], egui::Button::new(label).selected(app.printing.dpi == dpi)).clicked() {
                app.printing.dpi = dpi;
            }
        }
    });
    if !app.printing.single_sheet {
        ui.checkbox(&mut app.printing.marks, "자르는 점선·장 번호·5 cm 확인선 넣기");
        ui.checkbox(&mut app.printing.overview, "맨 앞에 붙이는 순서 안내 한 장 넣기");
    }
    ui.add_space(16.0);
    ui.separator();
    ui.label(format!("도면 · {} × {} mm", app.drawing.sheet.width_mm, app.drawing.sheet.height_mm));
    ui.label(format!("축척 · 1:{}", app.drawing.sheet.scale_denominator));
    if let Ok(plan) = app.printing.plan([app.drawing.sheet.width_mm, app.drawing.sheet.height_mm]) {
        ui.label(format!("{}장 · {} × {}", plan.pages.len() + usize::from(plan.overview), plan.cols, plan.rows));
        ui.label(format!("{}% · 1:{:.0}", plan.ratio * 100.0, f64::from(app.drawing.sheet.scale_denominator) / plan.ratio));
    }
}
