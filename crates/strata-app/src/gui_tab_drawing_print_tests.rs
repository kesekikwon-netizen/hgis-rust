//! The PDF of a sheet: its resolution, what it holds, 취소, and the screen's pictures kept apart — ported in
//! part from tests/test_drawing_studio.cpp:248-283 (pdfProgress_cancelKeepsTargetUntouched).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use eframe::egui;

use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
use strata_layout::sheet_pdf_vector::extracted_text;

use super::super::sheet_pdf_file::{picture_size, print_dpi, render_note, SHEET_DPI};
use super::CANCELLED;
use crate::app_state::StrataAppState;
use crate::basemap_tiles::TileKind;

/// Runs the frames of the app until the PDF is made (two minutes at most).
fn until_made(app: &mut StrataAppState) {
    let screen = egui::Context::default();
    let end = Instant::now() + Duration::from_secs(120);
    while app.drawing.pdf.0.is_some() && Instant::now() < end {
        app.advance_sheet_pdf(&screen);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.drawing.pdf.0.is_none(), "the PDF was not made within two minutes");
}

/// 300 DPI, lower for a paper too large to draw at it (never more than a hundred million pixels); the words
/// of the progress window name the paper and the resolution, and warn about a large paper (`renderNote`).
#[test]
fn a_sheet_pdf_has_300_dpi_and_a_large_paper_less() {
    assert_eq!((print_dpi([210.0, 297.0]), print_dpi([297.0, 420.0])), (SHEET_DPI, SHEET_DPI));
    assert_eq!(picture_size([210.0, 297.0], SHEET_DPI), [2480, 3508]);
    let a0 = print_dpi([841.0, 1189.0]);
    let [width, height] = picture_size([841.0, 1189.0], a0);
    assert!(a0 < SHEET_DPI && width * height <= 100_000_000 && width * height > 95_000_000, "{a0} {width} {height}");
    assert!(!render_note([210.0, 297.0], 300.0).contains("큰 용지"));
    assert!(render_note([841.0, 1189.0], 300.0).contains("큰 용지"));
    assert!(render_note([210.0, 297.0], 300.0).starts_with("PDF를 그리는 중… (용지 210×297 mm · 300 DPI)"));
}

/// The file is a vector page: the title is text, the survey ring is a path, and the page is A4 portrait.
/// A picture of the sheet (one JPEG) does not pass.
#[test]
fn a_sheet_pdf_is_vector_text_and_paths() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("도면.pdf");
    let mut app = StrataAppState::new_startup();
    app.drawing.title_ko = "유구배치도".to_owned();
    app.drawing.sheet.scale_denominator = 5_000;
    // Work CRS EPSG:5187, metres. The ring's east side is 100 m from the centre.
    app.drawing.sheet_center = Some([200_000.0, 400_000.0]);
    app.home_map.real_survey.survey_rings = vec![vec![
        [200_000.0, 400_000.0],
        [200_100.0, 400_000.0],
        [200_100.0, 400_050.0],
    ]];
    app.start_sheet_pdf(target.clone());
    until_made(&mut app);
    let pdf = std::fs::read(&target).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(!pdf.windows(10).any(|window| window == b"/DCTDecode"), "the page is one picture");
    let text = extracted_text(&pdf);
    assert!(text.contains("유구배치도"), "{text}");
    assert!(text.contains("축척 1 : 5000"), "{text}");
    let raw = String::from_utf8_lossy(&pdf);
    assert!(raw.contains("Malgun Gothic"));
    assert!(raw.contains("/MediaBox [0 0 595.28 841.89]"), "A4 portrait is 210×297 mm");
    assert!(raw.contains(" m") && raw.contains(" l"), "the survey ring is not a path");
    // 100 m east at 1:5000 is 20 mm to the right of the map centre (EPSG:5187 metres).
    let east = app.drawing.sheet.world_to_sheet_mm([200_000.0, 400_000.0], [200_100.0, 400_000.0]);
    let x = format!("{:.2}", strata_layout::sheet_pdf_export::mm_to_pdf_points(east[0]));
    let y = format!("{:.2}", strata_layout::sheet_pdf_export::mm_to_pdf_points(app.drawing.sheet.height_mm - east[1]));
    assert!(raw.contains(&format!("{x} {y} l")), "the ring is not at the paper scale");
    assert_eq!(app.status_left_message(), format!("저장: {}", target.display()));
    assert!(!dir.path().join("도면.pdf.part").exists());
}

/// The page box follows the sheet: the field sheet is 317×220 mm and A3 portrait is 297×420 mm.
#[test]
fn the_pdf_page_box_follows_the_field_sheet_and_a3() {
    let mut app = StrataAppState::new_startup();
    app.drawing.sheet.change_paper(SheetPaperSize::A4, SheetOrientation::Landscape);
    let field = app.vector_sheet_pdf(None, [0, 0]).unwrap();
    assert!(String::from_utf8_lossy(&field).contains("/MediaBox [0 0 898.58 623.62]"));
    app.drawing.sheet.change_paper(SheetPaperSize::A3, SheetOrientation::Portrait);
    let a3 = app.vector_sheet_pdf(None, [0, 0]).unwrap();
    assert!(String::from_utf8_lossy(&a3).contains("/MediaBox [0 0 841.89 1190.55]"));
}

/// 취소 leaves no file, and a file of that name stays as it was (KaDrawingPdfProgress.cpp:60-84).
#[test]
fn a_cancelled_pdf_leaves_no_file_and_an_old_file_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("도면.pdf");
    std::fs::write(&target, b"old").unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_sheet_pdf(target.clone());
    app.drawing.pdf.0.as_ref().unwrap().cancel.store(true, Ordering::Release);
    until_made(&mut app);
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    assert!(!dir.path().join("도면.pdf.part").exists());
    assert_eq!(app.status_left_message(), CANCELLED);
    assert!(app.modals.active.is_none(), "the progress window is closed");
}

/// The tiles and pictures the screen holds belong to the screen's context: the PDF draws with its own, and
/// the screen has its own back afterwards, untouched.
#[test]
fn the_screens_tiles_stay_the_screens() {
    let dir = tempfile::tempdir().unwrap();
    let screen = egui::Context::default();
    let tile = screen.load_texture("screen tile", egui::ColorImage::new([1, 1], egui::Color32::RED), egui::TextureOptions::LINEAR);
    let mut app = StrataAppState::new_startup();
    let key = (TileKind::Satellite, 16, 1, 1);
    app.basemap_textures.insert(key, tile.clone());
    let started = app.basemap_fetch.requests_started();
    app.start_sheet_pdf(dir.path().join("도면.pdf"));
    until_made(&mut app);
    assert_eq!(app.basemap_textures.get(&key).map(egui::TextureHandle::id), Some(tile.id()));
    assert_eq!(app.basemap_textures.len(), 1);
    assert_eq!(app.basemap_fetch.requests_started(), started, "the screen's fetcher asked for nothing");
    assert!(Path::new(&dir.path().join("도면.pdf")).exists());
}

/// 취소 while the sheet is still being painted stops at once: the next frame ends it, nothing is drawn further.
#[test]
fn cancel_while_painting_stops_at_the_next_frame() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_sheet_pdf(dir.path().join("도면.pdf"));
    app.drawing.pdf.0.as_ref().unwrap().cancel.store(true, Ordering::Release);
    app.advance_sheet_pdf(&egui::Context::default());
    assert!(app.drawing.pdf.0.is_none());
    assert_eq!(app.status_left_message(), CANCELLED);
}

/// The PDF is made once the tiles had time to be asked for (a tile is asked for only after the view has
/// rested, basemap_tile_key.rs `VIEW_REST_TIME`) and two passes in a row asked for nothing new; after the
/// longest wait it is made with what is there.
#[test]
fn the_pdf_waits_for_the_tiles_to_be_asked_for() {
    use std::time::Duration;
    let rest = crate::basemap_tile_key::VIEW_REST_TIME;
    assert!(!super::ready(5, 5, rest / 2), "quiet, but the tiles were not asked for yet");
    assert!(super::ready(5, 2, rest * 4));
    assert!(!super::ready(5, 1, rest * 4), "one quiet pass is not enough");
    assert!(super::ready(1, 0, Duration::from_secs(91)), "the longest wait is over");
}

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<egui::Event>) {
    let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1680.0, 960.0))), events, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
}

/// The progress window holds the app as the C++ one (`Qt::ApplicationModal`, KaDrawingPdfProgress.cpp:14):
/// while the PDF is made the keys and tools of the main window wait and a click outside the window reaches
/// nothing behind it. Esc is 취소 (the Escape of the window's 취소 button), never a way to hide the window
/// while the PDF goes on; the window stays until the PDF has stopped.
#[test]
fn the_progress_window_holds_the_app_and_esc_cancels() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = StrataAppState::new_startup();
    app.active_tab = crate::app_state::MainTab::Drawing;
    app.start_sheet_pdf(dir.path().join("도면.pdf"));
    assert!(app.modal_window_open(), "the keys and tools of the main window wait");
    let ctx = egui::Context::default();
    frame(&mut app, &ctx, Vec::new());
    let behind = ctx.layer_id_at(egui::pos2(5.0, 5.0));
    assert_eq!(behind, Some(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("ka_modal_pdf_prep"))), "a click outside the window reaches nothing behind it");
    let esc = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::default() };
    frame(&mut app, &ctx, vec![esc]);
    assert!(app.drawing.pdf.0.as_ref().expect("it stops at the next frame").cancel.load(Ordering::Acquire));
    assert_eq!(app.modals.active, Some(crate::dialogs_modal::ActiveModalDialog::PdfPreparing));
    frame(&mut app, &ctx, Vec::new());
    assert!(app.drawing.pdf.0.is_none());
    assert_eq!(app.status_left_message(), CANCELLED);
    assert!(!dir.path().join("도면.pdf").exists());
}

/// Nothing takes the place of the window while the PDF is made: it is back at the next frame.
#[test]
fn the_progress_window_comes_back_while_the_pdf_is_made() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_sheet_pdf(dir.path().join("도면.pdf"));
    app.modals.active = None;
    app.advance_sheet_pdf(&egui::Context::default());
    assert_eq!(app.modals.active, Some(crate::dialogs_modal::ActiveModalDialog::PdfPreparing));
}

/// Opening another survey stops the PDF: nothing of the next survey goes into the file named for this one.
#[test]
fn opening_another_survey_stops_the_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let other = dir.path().join("다른.gpkg");
    strata_storage::gpkg::GpkgRepository::create_new_survey(&other, strata_types::crs::WorkCrs::Epsg5187).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_sheet_pdf(dir.path().join("도면.pdf"));
    assert!(app.open_survey_gpkg(&other));
    assert!(app.drawing.pdf.0.is_none());
    assert_ne!(app.modals.active, Some(crate::dialogs_modal::ActiveModalDialog::PdfPreparing));
    assert!(!dir.path().join("도면.pdf").exists());
}

/// The window says the three steps of the C++ (KaDrawingStudio.cpp:3771, 3785, 3813): the sheet, the
/// picture, then 「파일을 저장하는 중…」 while the file is written.
#[test]
fn the_window_says_when_the_file_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_sheet_pdf(dir.path().join("도면.pdf"));
    assert_eq!(app.drawing.pdf.0.as_ref().unwrap().words(), "도면을 정리하는 중…");
    let (send, receive) = std::sync::mpsc::channel();
    app.drawing.pdf.0.as_mut().unwrap().stage = super::Stage::Making(receive);
    assert!(app.drawing.pdf.0.as_ref().unwrap().words().starts_with("PDF를 그리는 중…"));
    send.send(super::Made::Saving).unwrap();
    app.advance_sheet_pdf(&egui::Context::default());
    assert_eq!(app.drawing.pdf.0.as_ref().unwrap().words(), "파일을 저장하는 중…");
    send.send(super::Made::Done(Ok(0))).unwrap();
    app.advance_sheet_pdf(&egui::Context::default());
    assert!(app.drawing.pdf.0.is_none());
}

/// The status line says when the picture had to leave something out, rather than only 「저장:」.
#[test]
fn the_status_says_when_pictures_were_left_out() {
    let target = Path::new("C:/조사/도면.pdf");
    assert_eq!(super::saved_words(target, 0), "저장: C:/조사/도면.pdf");
    assert_eq!(super::saved_words(target, 2), "저장: C:/조사/도면.pdf · 그림 2개는 PDF에 넣지 못했습니다.");
}
