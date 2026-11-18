//! 「PDF 내보내기」 — the sheet written as a vector PDF (C++ `savePdf` and `exportDrawingPdf`,
//! KaDrawingStudio.cpp:3758-3855, `forceVectorOutput` in PdfExportSettings.cpp:9-26, and
//! `KaDrawingPdfProgress`). A pass still settles the sheet on its own tile stores, so nothing of the
//! screen's pictures is mixed in. When that pass is quiet the file is text and paths, not one picture.
//! 취소 leaves no file, and a file of the same name stays as it was.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

#[cfg(test)]
use super::sheet_scene::ready;
use super::sheet_scene::SheetScene;
use eframe::egui;

pub(crate) use super::sheet_pdf_file::CANCELLED;
use super::sheet_pdf_file::{print_dpi, put_pdf, render_note, DESIGN_PX_PER_MM};
use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

enum Stage {
    Painting,
    /// The picture is made and written in the background; it answers with the pictures it had to leave out.
    Making(Receiver<Made>),
}

/// What the background work says.
pub(crate) enum Made {
    /// The file is being written.
    Saving,
    /// The end: the number of pictures left out, or why there is no file.
    Done(Result<usize, String>),
}

/// The status line after the PDF was written, with what the picture had to leave out.
pub(crate) fn saved_words(target: &std::path::Path, left_out: usize) -> String {
    match left_out {
        0 => format!("저장: {}", target.display()),
        _ => format!("저장: {} · 그림 {left_out}개는 PDF에 넣지 못했습니다.", target.display()),
    }
}

/// A PDF of the sheet being made.
pub(crate) struct SheetPdf {
    target: PathBuf,
    scene: SheetScene,
    stage: Stage,
    /// The background work is writing the file.
    saving: bool,
    pub(crate) cancel: Arc<AtomicBool>,
}

impl SheetPdf {
    /// What the progress window says.
    pub(crate) fn words(&self) -> String {
        match self.stage {
            Stage::Painting => "도면을 정리하는 중…".to_owned(),
            // KaDrawingStudio.cpp:3813.
            Stage::Making(_) if self.saving => "파일을 저장하는 중…".to_owned(),
            Stage::Making(_) => render_note(self.scene.paper_mm, self.scene.dpi),
        }
    }

    /// Writes the vector PDF in the background.
    fn make(&mut self, pdf: Vec<u8>) {
        let (send, receive) = channel();
        let (target, cancel) = (self.target.clone(), Arc::clone(&self.cancel));
        std::thread::spawn(move || {
            let written = (|| {
                drop(send.send(Made::Saving));
                put_pdf(&target, &pdf, &cancel)
            })();
            drop(send.send(Made::Done(written.map(|()| 0))));
        });
        self.stage = Stage::Making(receive);
    }
}

/// The slot of the PDF being made: a copy of the app state has none.
#[derive(Default)]
pub(crate) struct SheetPdfSlot(pub(crate) Option<Box<SheetPdf>>);

impl Clone for SheetPdfSlot {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::fmt::Debug for SheetPdfSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() { "SheetPdf(making)" } else { "SheetPdf(none)" })
    }
}

impl StrataAppState {
    /// 「PDF 내보내기」: asks where (「도면 PDF 저장」, 「도면.pdf」, KaDrawingStudio.cpp:3825-3826), then makes it.
    pub(crate) fn export_sheet_pdf(&mut self) {
        if self.drawing.pdf.0.is_some() {
            return;
        }
        let mut dialog = rfd::FileDialog::new().set_title("도면 PDF 저장").set_file_name("도면.pdf").add_filter("PDF", &["pdf"]);
        if let Some(folder) = self.survey_path.as_ref().and_then(|path| path.parent()) {
            dialog = dialog.set_directory(folder);
        }
        if let Some(target) = dialog.save_file() {
            self.start_sheet_pdf(target);
        }
    }

    /// Starts making the PDF of the sheet into `target`, at the size the sheet was last seen at.
    pub(crate) fn start_sheet_pdf(&mut self, target: PathBuf) {
        let paper_mm = [self.drawing.sheet.width_mm, self.drawing.sheet.height_mm];
        let px_per_mm = self.drawing.screen_px_per_mm.unwrap_or(DESIGN_PX_PER_MM);
        let dpi = print_dpi(paper_mm);
        let job = SheetPdf { target, scene: SheetScene::new(paper_mm, px_per_mm, dpi), stage: Stage::Painting, saving: false, cancel: Arc::new(AtomicBool::new(false)) };
        self.drawing.pdf.0 = Some(Box::new(job));
        self.modals.active = Some(ActiveModalDialog::PdfPreparing);
    }

    /// Once a frame while a PDF is made: one more pass, or the picture, or the end.
    pub(crate) fn advance_sheet_pdf(&mut self, screen: &egui::Context) {
        let Some(mut job) = self.drawing.pdf.0.take() else { return };
        screen.request_repaint();
        let cancelled = job.cancel.load(Ordering::Acquire);
        let ended = match &job.stage {
            Stage::Painting if cancelled => Some(Err(CANCELLED.to_owned())),
            Stage::Painting => {
                job.scene.pass(self);
                if !job.scene.ready() {
                    None
                } else {
                    let frame = self.drawing.sheet.map_frame;
                    let jpeg = job.scene.basemap_jpeg([frame.x_mm, frame.y_mm, frame.width_mm, frame.height_mm]);
                    match self.vector_sheet_pdf(jpeg.as_ref().map(|image| image.0.as_slice()), jpeg.as_ref().map(|image| image.1).unwrap_or([0, 0])) {
                        Ok(pdf) => {
                            job.make(pdf);
                            None
                        }
                        Err(why) => Some(Err(why)),
                    }
                }
            }
            Stage::Making(receive) => match receive.try_recv() {
                Ok(Made::Done(written)) => Some(written),
                Ok(Made::Saving) => {
                    job.saving = true;
                    None
                }
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err("PDF를 만드는 중에 멈췄습니다.".to_owned())),
            },
        };
        let Some(ended) = ended else {
            // Nothing takes the place of the window while the PDF is made: the C++ one is modal (:14).
            self.modals.active = Some(ActiveModalDialog::PdfPreparing);
            self.drawing.pdf.0 = Some(job);
            return;
        };
        if self.modals.active == Some(ActiveModalDialog::PdfPreparing) {
            self.modals.active = None;
        }
        match ended {
            Ok(left_out) => self.show_status_message(saved_words(&job.target, left_out)),
            Err(why) if why == CANCELLED => self.show_status_message(CANCELLED),
            Err(why) => self.show_status_message(format!("PDF를 저장하지 못했습니다. {why}")),
        }
    }

    /// The PDF stops when the survey is left: the next survey's sheet is not the one the file was named for.
    pub(crate) fn drop_sheet_pdf(&mut self) {
        if let Some(job) = self.drawing.pdf.0.take() {
            job.cancel.store(true, Ordering::Release);
            if self.modals.active == Some(ActiveModalDialog::PdfPreparing) {
                self.modals.active = None;
            }
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_print_tests.rs"]
mod tests;
