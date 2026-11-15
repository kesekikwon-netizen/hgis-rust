//! 「PDF 내보내기」 — the sheet painted again off the screen at print resolution and saved as a PDF of one
//! picture (docs/RUST_PLAN.md R3-임시; C++ `savePdf` and `exportDrawingPdf`, KaDrawingStudio.cpp:3758-3855, and
//! `KaDrawingPdfProgress`). A pass paints the sheet with its own context, its own tile stores and fetchers and
//! its own reference pictures (made at once), so nothing of the screen's pictures is mixed in. Passes run once
//! a frame until every tile they asked for has come or failed; then the picture is made, packed and written
//! in the background. 취소 leaves no file, and a file of the same name stays as it was.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, epaint::ClippedShape, Color32, Pos2, RawInput, Rect, Vec2, ViewportId};

use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{shared_tile_fetchers, BasemapFetcher, CadastralFetcher};
use crate::basemap_paint::{CadastralCellCache, TileTextureCache};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ref_vector::print::PictureSet;
use super::sheet_raster::{paint, Textures};
pub(crate) use super::sheet_pdf_file::CANCELLED;
use super::sheet_pdf_file::{picture_size, print_dpi, render_note, write_pdf, DESIGN_PX_PER_MM, TILE_WAIT_SECONDS};

/// Whether the PDF can be made after `passes` passes, the last `quiet` of which asked for nothing new and had
/// nothing on the way, `waited` after it began: the tiles had time to be asked for (a tile is asked for once
/// the view has rested, basemap_tile_key.rs `VIEW_REST_TIME`), or the longest wait is over.
fn ready(passes: u32, quiet: u32, waited: std::time::Duration) -> bool {
    let asked = waited >= crate::basemap_tile_key::VIEW_REST_TIME * 3;
    (passes >= 3 && quiet >= 2 && asked) || waited.as_secs_f32() > TILE_WAIT_SECONDS
}

/// The tile stores and fetchers of one side: the screen's, or those of a PDF being drawn.
struct TileSide {
    basemap: TileTextureCache,
    cadastral: CadastralCellCache,
    basemap_fetch: BasemapFetcher,
    cadastral_fetch: CadastralFetcher,
    painted: usize,
}

impl TileSide {
    fn fresh() -> Self {
        let (basemap_fetch, cadastral_fetch) = shared_tile_fetchers();
        Self { basemap: TileTextureCache::default(), cadastral: CadastralCellCache::default(), basemap_fetch, cadastral_fetch, painted: 0 }
    }

    /// Nothing on the way and nothing waiting to be shown (the two fetchers share one count).
    fn settled(&self) -> bool {
        self.basemap_fetch.in_flight_count() == 0 && !self.basemap_fetch.take_done_limited(0).1 && !self.cadastral_fetch.take_done_limited(0).1
    }

    fn requests(&self) -> usize {
        self.basemap_fetch.requests_started() + self.cadastral_fetch.requests_started()
    }
}

enum Stage {
    Painting,
    /// The picture is made and written in the background; it answers with the pictures it had to leave out.
    Making(Receiver<Result<usize, String>>),
}

/// A PDF of the sheet being made.
pub(crate) struct SheetPdf {
    target: PathBuf,
    ctx: egui::Context,
    textures: Textures,
    paper_mm: [f64; 2],
    px_per_mm: f32,
    pixels_per_point: f32,
    dpi: f32,
    tiles: TileSide,
    pictures: PictureSet,
    passes: u32,
    quiet: u32,
    requests: usize,
    started: Instant,
    shapes: Vec<ClippedShape>,
    stage: Stage,
    pub(crate) cancel: Arc<AtomicBool>,
}

impl SheetPdf {
    /// What the progress window says.
    pub(crate) fn words(&self) -> String {
        match self.stage {
            Stage::Painting => "도면을 정리하는 중…".to_owned(),
            Stage::Making(_) => render_note(self.paper_mm, self.dpi),
        }
    }

    fn ready(&self) -> bool {
        ready(self.passes, self.quiet, self.started.elapsed())
    }

    /// Tessellates the last pass and makes, packs and writes the picture in the background.
    fn make(&mut self, title: String) {
        let primitives = self.ctx.tessellate(std::mem::take(&mut self.shapes), self.pixels_per_point);
        let textures = std::mem::take(&mut self.textures);
        let (send, receive) = channel();
        let (target, paper_mm, cancel, scale) = (self.target.clone(), self.paper_mm, Arc::clone(&self.cancel), self.pixels_per_point);
        let size = picture_size(paper_mm, self.dpi);
        std::thread::spawn(move || {
            let picture = paint(&primitives, &textures, scale, size);
            drop((primitives, textures));
            let written = write_pdf(&target, paper_mm, &title, &picture, &cancel).map(|()| picture.unknown_pictures);
            drop(send.send(written));
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
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::gui_theme_fonts::font_definitions());
        let job = SheetPdf {
            target,
            ctx,
            textures: Textures::default(),
            paper_mm,
            px_per_mm,
            pixels_per_point: dpi / 25.4 / px_per_mm,
            dpi,
            tiles: TileSide::fresh(),
            pictures: PictureSet::made_at_once(),
            passes: 0,
            quiet: 0,
            requests: 0,
            started: Instant::now(),
            shapes: Vec::new(),
            stage: Stage::Painting,
            cancel: Arc::new(AtomicBool::new(false)),
        };
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
                self.print_pass(&mut job);
                if job.ready() {
                    job.make(self.drawing.title_ko.clone());
                }
                None
            }
            Stage::Making(receive) => match receive.try_recv() {
                Ok(written) => Some(written),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err("PDF를 만드는 중에 멈췄습니다.".to_owned())),
            },
        };
        let Some(ended) = ended else {
            self.drawing.pdf.0 = Some(job);
            return;
        };
        if self.modals.active == Some(ActiveModalDialog::PdfPreparing) {
            self.modals.active = None;
        }
        match ended {
            Ok(_) => self.show_status_message(format!("저장: {}", job.target.display())),
            Err(why) if why == CANCELLED => self.show_status_message(CANCELLED),
            Err(why) => self.show_status_message(format!("PDF를 저장하지 못했습니다. {why}")),
        }
    }

    /// One pass of the sheet on the PDF's own context, with the PDF's own tiles and pictures.
    fn print_pass(&mut self, job: &mut SheetPdf) {
        let paper = Rect::from_min_size(Pos2::ZERO, Vec2::new(job.paper_mm[0] as f32, job.paper_mm[1] as f32) * job.px_per_mm);
        let mut raw = RawInput { screen_rect: Some(paper), max_texture_side: Some(16_384), time: Some(f64::from(job.passes) * 0.1), ..Default::default() };
        raw.viewports.entry(ViewportId::ROOT).or_default().native_pixels_per_point = Some(job.pixels_per_point);
        self.swap_tiles(&mut job.tiles);
        self.ref_vectors.swap_pictures(&mut job.pictures);
        let px_per_mm = job.px_per_mm;
        let out = job.ctx.run(raw, |ctx| self.paint_sheet_for_print(ctx, paper, px_per_mm));
        self.ref_vectors.swap_pictures(&mut job.pictures);
        self.swap_tiles(&mut job.tiles);
        job.textures.apply(&out.textures_delta);
        job.shapes = out.shapes;
        job.passes += 1;
        let requests = job.tiles.requests();
        let quiet = requests == job.requests && job.tiles.settled() && !job.pictures.still_drawing();
        job.quiet = if quiet { job.quiet + 1 } else { 0 };
        job.requests = requests;
    }

    fn swap_tiles(&mut self, side: &mut TileSide) {
        std::mem::swap(&mut self.basemap_textures, &mut side.basemap);
        std::mem::swap(&mut self.cadastral_cells, &mut side.cadastral);
        std::mem::swap(&mut self.basemap_fetch, &mut side.basemap_fetch);
        std::mem::swap(&mut self.cadastral_fetch, &mut side.cadastral_fetch);
        std::mem::swap(&mut self.basemap_tiles_painted, &mut side.painted);
    }

    /// The sheet on white paper, as the screen paints it, without the tools of the screen (toolbar, popups,
    /// handles). Coordinate points are left out: they are kept as places on the screen.
    pub(crate) fn paint_sheet_for_print(&mut self, ctx: &egui::Context, paper: Rect, px_per_mm: f32) {
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(paper, 0.0, Color32::WHITE);
        let screen_items = std::mem::take(&mut self.drawing.item_rects);
        let f = self.sheet_frame_for(paper, paper, px_per_mm, painter, false);
        self.paint_sheet_map_layers(&f);
        self.paint_sheet_grid(&f);
        self.paint_sheet_legend(&f);
        self.paint_sheet_chrome(&f);
        self.drawing.item_rects = screen_items;
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_print_tests.rs"]
mod tests;
