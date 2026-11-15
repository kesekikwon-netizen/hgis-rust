//! The PDF of a sheet: its resolution, what it holds, 취소, and the screen's pictures kept apart — ported in
//! part from tests/test_drawing_studio.cpp:248-283 (pdfProgress_cancelKeepsTargetUntouched).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use eframe::egui;

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

/// The JPEG inside a PDF of one picture, and the size the PDF says it has.
fn picture_of(pdf: &[u8]) -> (image::RgbImage, String) {
    let head = pdf.windows(10).position(|w| w == b"/DCTDecode").unwrap();
    let start = head + pdf[head..].windows(7).position(|w| w == b"stream\n").unwrap() + 7;
    let end = start + pdf[start..].windows(10).position(|w| w == b"\nendstream").unwrap();
    let size_at = pdf[..head].windows(6).rposition(|w| w == b"/Width").unwrap();
    let size = String::from_utf8_lossy(&pdf[size_at..head]).into_owned();
    (image::load_from_memory(&pdf[start..end]).unwrap().to_rgb8(), size)
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

/// The PDF holds the sheet as the screen paints it, at 300 DPI: white paper at the corner, the light map
/// frame, and the dark lines and letters of the sheet; the status line names the file.
#[test]
fn a_sheet_pdf_is_the_sheet_as_a_picture() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("도면.pdf");
    let mut app = StrataAppState::new_startup();
    app.start_sheet_pdf(target.clone());
    until_made(&mut app);
    let pdf = std::fs::read(&target).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let (picture, size) = picture_of(&pdf);
    assert!(size.starts_with("/Width 2480 /Height 3508"), "{size}");
    assert_eq!(picture.dimensions(), (2480, 3508));
    let corner = picture.get_pixel(20, 20).0;
    assert!(corner.iter().all(|c| *c > 245), "{corner:?}");
    let dark = picture.pixels().filter(|p| p.0.iter().all(|c| *c < 80)).count();
    assert!(dark > 2_000, "{dark} dark pixels");
    assert_eq!(app.status_left_message(), format!("저장: {}", target.display()));
    assert!(!dir.path().join("도면.pdf.part").exists());
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
