//! Fake dispatch boundary only; real rendering must carry the frozen sheet to it.
use super::super::{print_device::Printer, print_plan::DevicePaper, print_state::CatalogState};
use super::*;

#[test]
fn starting_print_captures_real_sheet_for_a_background_dispatch() {
    let mut app = StrataAppState::new_startup();
    let printer = Printer {
        name: "FAKE-JOB-PRINTER".to_owned(),
        is_default: true,
        file_output: false,
        papers: vec![DevicePaper { name: "A4".to_owned(), width_mm: 210.0, height_mm: 297.0, portrait: [5.0, 5.0, 200.0, 287.0], landscape: [5.0, 5.0, 287.0, 200.0] }],
    };
    app.printing.selected = Some(printer.name.clone());
    app.printing.catalog = CatalogState::Ready(vec![printer]);
    app.drawing.title_ko = "CAPTURED-SHEET".to_owned();
    let (send, receive) = std::sync::mpsc::channel();
    let dispatch: Dispatch = Box::new(move |request, raster, _cancel| {
        let picture = raster.paint();
        assert_eq!(request.title, "CAPTURED-SHEET");
        assert_eq!(picture.size, [2480, 3508]);
        assert!(picture.rgb.chunks_exact(3).filter(|rgb| rgb.iter().all(|c| *c < 80)).count() > 2000, "actual sheet was not rendered");
        send.send(request.title.clone()).expect("test receiver is alive");
        Ok(PrintReceipt { version: 1, printer: request.printer, pages: 1, paper_mm: [210.0, 297.0] })
    });
    app.start_device_print_using(dispatch).expect("valid fake device must start the real render pipeline");
    app.drawing.title_ko = "MUTATED-AFTER-CLICK".to_owned();
    let ctx = eframe::egui::Context::default();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut captured = None;
    while std::time::Instant::now() < deadline {
        app.advance_device_print(&ctx);
        if let Ok(title) = receive.try_recv() {
            captured = Some(title);
        }
        if captured.is_some() && app.printing.job.0.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(captured.as_deref(), Some("CAPTURED-SHEET"), "real frozen scene did not reach dispatch");
    assert_eq!(app.status_left_message(), "프린터로 1쪽을 보냈습니다.");
}

#[test]
fn a0_at_600_dpi_starts_without_allocating_a_whole_drawing_bitmap() {
    use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
    let mut app = StrataAppState::new_startup();
    let printer = Printer {
        name: "FAKE-A3-PRINTER".to_owned(),
        is_default: true,
        file_output: false,
        papers: vec![DevicePaper { name: "A3".to_owned(), width_mm: 297.0, height_mm: 420.0, portrait: [5.0, 5.0, 287.0, 410.0], landscape: [5.0, 5.0, 410.0, 287.0] }],
    };
    app.printing.install_catalog(vec![printer]);
    app.printing.single_sheet = false;
    app.printing.dpi = 600;
    app.drawing.set_paper_and_orientation(SheetPaperSize::A0, SheetOrientation::Portrait);
    app.start_device_print_using(Box::new(|_, _, _| panic!("cancelled before dispatch"))).expect("page-sized rendering must accept A0 at requested600 DPI");
    app.printing.job.cancel();
    app.advance_device_print(&eframe::egui::Context::default());
    assert!(app.printing.job.0.is_none());
}
