//! Native managed paint boundary without a printer. target/cpp-ref/tests/test_tile_print.cpp:18-55,114-158.
use super::super::print_plan::{build_plan, PrintMode};
use super::*;
use std::sync::atomic::AtomicBool;

#[test]
fn managed_probe_places_each_actual_image_quadrant_on_its_own_page() {
    let dir = tempfile::tempdir().expect("temporary test files");
    let image_path = dir.path().join("quadrants.png");
    let colours = [[220, 20, 20], [20, 170, 40], [20, 40, 220], [240, 210, 20]];
    let mut rgb = Vec::new();
    for y in 0..600 {
        for x in 0..800 {
            rgb.extend_from_slice(&colours[usize::from(x >= 400) + 2 * usize::from(y >= 300)]);
        }
    }
    image::save_buffer(&image_path, &rgb, 800, 600, image::ColorType::Rgb8).expect("recorded C++ quadrant colours");
    let paper = DevicePaper { name: "FAKE-A4".to_owned(), width_mm: 210.0, height_mm: 297.0, portrait: [5.0, 5.0, 200.0, 287.0], landscape: [5.0, 5.0, 287.0, 200.0] };
    let plan = build_plan([400.0, 300.0], &paper, PrintMode::Tiled { overlap_mm: 10.0, marks: false, overview: false }).expect("recorded paper");
    assert_eq!(plan.pages.len(), 4);
    let request = PrintRequest {
        printer: "FAKE-NO-PHYSICAL-PRINTER".to_owned(),
        title: "Recorded quadrant print".to_owned(),
        paper,
        plan,
        drawing_mm: [400.0, 300.0],
        dpi: 300,
        output: None,
    };
    let request_path = dir.path().join("request.json");
    std::fs::write(&request_path, super::print_device_output::request_bytes(&request, &image_path).expect("typed request")).expect("temporary request");
    let bytes = super::print_device_process::run("Probe", Some(&request_path), &AtomicBool::new(false)).expect("hidden probe must never call PrintDocument.Print");
    assert_eq!(validate_receipt(&request, &bytes).expect("actual managed receipt").pages, 4);
    for (index, expected) in colours.into_iter().enumerate() {
        let bytes = std::fs::read(dir.path().join(format!("page-{:04}.png", index + 1))).expect("actual managed output");
        let picture = image::load_from_memory(&bytes).expect("PNG from real managed painter").to_rgb8();
        let actual = picture.get_pixel(80, 80).0;
        // C++ near() accepts40 RGB levels. Samples are15mm inside constant-colour quadrants.
        assert!(actual.iter().zip(expected).all(|(got, want)| got.abs_diff(want) < 40), "page {}: {actual:?} != {expected:?}", index + 1);
        if index == 1 {
            assert!(picture.get_pixel(640, 80).0.iter().all(|v| *v > 230), "last-column remainder was stretched");
        }
    }
}

#[test]
#[ignore = "opt-in local Microsoft PDF file output only; never a physical printer"]
fn opt_in_virtual_pdf_file_output_of_the_actual_sheet() {
    use crate::app_state::StrataAppState;
    use strata_layout::sheet_composer::SheetPaperSize;
    let printers = discover().expect("read-only installed printer catalog");
    let printer = printers.into_iter().find(|p| p.file_output && p.name == "Microsoft Print to PDF").expect("built-in virtual file printer required");
    let dir = tempfile::tempdir().expect("temporary output only");
    let output = dir.path().join("actual-sheet.pdf");
    let mut app = StrataAppState::new_startup();
    app.printing.install_catalog(vec![printer]);
    app.printing.paper = SheetPaperSize::A4;
    app.printing.output_path = output.to_string_lossy().into_owned();
    app.drawing.sheet.legend_title = "VIRTUAL-FILE-ACTUAL-SHEET".to_owned();
    app.start_device_print_using(Box::new(dispatch)).expect("captured sheet to explicit file destination");
    let ctx = eframe::egui::Context::default();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while app.printing.job.0.is_some() && std::time::Instant::now() < deadline {
        app.advance_device_print(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if app.printing.job.0.is_some() {
        app.printing.job.cancel();
        app.advance_device_print(&ctx);
        panic!("virtual file output timed out");
    }
    assert_eq!(app.status_left_message(), "프린터로 1쪽을 보냈습니다.");
    let bytes = std::fs::read(&output).expect("actual Windows PDF file committed");
    assert!(bytes.starts_with(b"%PDF-") && bytes.windows(5).any(|v| v == b"%%EOF"), "unfinished virtual PDF");
    println!("VIRTUAL_FILE_ONLY_OK bytes={} actual_page_count=1", bytes.len());
}
