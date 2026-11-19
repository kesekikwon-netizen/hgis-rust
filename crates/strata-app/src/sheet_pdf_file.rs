//! The paper side of a sheet's PDF: its resolution and size in pixels, the words of the progress window,
//! and the writing of the file (C++ PdfExportSettings.h:15, KaDrawingPdfProgress.cpp:47-84).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// The resolution of a sheet's PDF (C++ `KaPdfExport::kSheetDpi`, PdfExportSettings.h:15).
pub(crate) const SHEET_DPI: f32 = 300.0;
/// A picture of more pixels than this is drawn at a lower resolution: A0 at 300 DPI has about 140 million.
const MOST_PIXELS: f64 = 100_000_000.0;
/// How long the tiles of a PDF may take to come before it is made with what is there.
pub(crate) const TILE_WAIT_SECONDS: f32 = 90.0;
/// The size the sheet is drawn at when it was never on the screen: the size its items are designed for.
pub(crate) const DESIGN_PX_PER_MM: f32 = 1.65;
/// The words of a cancelled PDF (KaDrawingPdfProgress.cpp:82-84).
pub(crate) const CANCELLED: &str = "PDF 만들기를 취소했습니다. 파일은 만들지 않았습니다.";

/// The resolution a paper of `paper_mm` (width, height) is drawn at: 300 DPI, lower for a very large paper.
pub(crate) fn print_dpi(paper_mm: [f64; 2]) -> f32 {
    let square_inches = paper_mm[0] / 25.4 * (paper_mm[1] / 25.4);
    // A resolution of a few hundred: f32 holds it.
    #[allow(clippy::cast_possible_truncation)]
    let most = (MOST_PIXELS / square_inches.max(1e-6)).sqrt() as f32;
    SHEET_DPI.min(most)
}

/// The size in pixels of a paper drawn at `dpi`.
pub(crate) fn picture_size(paper_mm: [f64; 2], dpi: f32) -> [usize; 2] {
    // A count of pixels: positive and far below the limits of usize.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = |mm: f64| (mm / 25.4 * f64::from(dpi)).round().max(1.0) as usize;
    [side(paper_mm[0]), side(paper_mm[1])]
}

/// The words while the picture is made (`KaDrawingPdfProgress::renderNote`, KaDrawingPdfProgress.cpp:47-58).
pub(crate) fn render_note(paper_mm: [f64; 2], dpi: f32) -> String {
    let note = format!("PDF를 그리는 중… (용지 {}×{} mm · {} DPI)", paper_mm[0].round(), paper_mm[1].round(), dpi.round());
    if paper_mm[0] * paper_mm[1] > 297.0 * 420.0 * 1.05 {
        format!("{note}\n큰 용지라 시간이 걸립니다. 취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.")
    } else {
        format!("{note}\n취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.")
    }
}

/// Writes the PDF next to `target` first and then puts it in its place, unless 취소 came in the meantime;
/// a draft that could not be written whole is taken away (`KaDrawingPdfProgress::commit`,
/// KaDrawingPdfProgress.cpp:60-80).
pub(crate) fn put_pdf(target: &Path, pdf: &[u8], cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        return Err(CANCELLED.to_owned());
    }
    let draft = target.with_extension("pdf.part");
    let mut file = std::fs::File::create(&draft).map_err(|_| "PDF 저장 파일을 열지 못했습니다.".to_owned())?;
    if std::io::Write::write_all(&mut file, pdf).and_then(|()| file.sync_all()).is_err() {
        drop(file);
        drop(std::fs::remove_file(&draft));
        return Err("PDF 파일을 저장하지 못했습니다.".to_owned());
    }
    drop(file);
    if cancel.load(Ordering::Acquire) {
        drop(std::fs::remove_file(&draft));
        return Err(CANCELLED.to_owned());
    }
    std::fs::rename(&draft, target).map_err(|_| {
        drop(std::fs::remove_file(&draft));
        "PDF 파일을 확정하지 못했습니다.".to_owned()
    })
}
