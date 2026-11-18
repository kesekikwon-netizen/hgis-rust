//! Shared isolated sheet rendering, target/cpp-ref/src/app/KaDrawingStudio.cpp:3758-3855.
use super::sheet_pdf_file::{picture_size, TILE_WAIT_SECONDS};
use super::sheet_raster::{self, Picture, Textures};
use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{shared_tile_fetchers, BasemapFetcher, CadastralFetcher};
use crate::basemap_paint::{CadastralCellCache, TileTextureCache};
use crate::ref_vector::print::PictureSet;
use eframe::egui::{
    self,
    epaint::{ClippedPrimitive, ClippedShape},
    Pos2, RawInput, Rect, Vec2, ViewportId,
};
use std::time::{Duration, Instant};

pub(crate) fn ready(passes: u32, quiet: u32, waited: Duration) -> bool {
    let asked = waited >= crate::basemap_tile_key::VIEW_REST_TIME * 3;
    (passes >= 3 && quiet >= 2 && asked) || waited.as_secs_f32() > TILE_WAIT_SECONDS
}

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
    fn settled(&self) -> bool {
        self.basemap_fetch.in_flight_count() == 0 && !self.basemap_fetch.take_done_limited(0).1 && !self.cadastral_fetch.take_done_limited(0).1
    }
    fn requests(&self) -> usize {
        self.basemap_fetch.requests_started() + self.cadastral_fetch.requests_started()
    }
    fn swap(&mut self, app: &mut StrataAppState) {
        std::mem::swap(&mut app.basemap_textures, &mut self.basemap);
        std::mem::swap(&mut app.cadastral_cells, &mut self.cadastral);
        std::mem::swap(&mut app.basemap_fetch, &mut self.basemap_fetch);
        std::mem::swap(&mut app.cadastral_fetch, &mut self.cadastral_fetch);
        std::mem::swap(&mut app.basemap_tiles_painted, &mut self.painted);
    }
}

pub(crate) struct SheetScene {
    ctx: egui::Context,
    textures: Textures,
    pub(crate) paper_mm: [f64; 2],
    pub(crate) dpi: f32,
    px_per_mm: f32,
    pixels_per_point: f32,
    tiles: TileSide,
    pictures: PictureSet,
    passes: u32,
    quiet: u32,
    requests: usize,
    started: Instant,
    shapes: Vec<ClippedShape>,
}

pub(crate) struct RasterInput {
    primitives: Vec<ClippedPrimitive>,
    textures: Textures,
    scale: f32,
    /// Full-page pixel size. The print tests paint the whole page; the sheet PDF uses `region_mm`.
    #[allow(dead_code)]
    size: [usize; 2],
    paper_mm: [f64; 2],
    design_px_per_mm: f32,
    dpi: f32,
}

impl RasterInput {
    #[cfg(test)]
    pub(crate) fn paint(self) -> Picture {
        sheet_raster::paint(&self.primitives, &self.textures, self.scale, self.size)
    }
    pub(crate) fn region_mm(&self, rect: [f64; 4], dpi: f32) -> Picture {
        let origin = [rect[0] as f32 * self.design_px_per_mm, rect[1] as f32 * self.design_px_per_mm];
        sheet_raster::paint_region(&self.primitives, &self.textures, self.scale * dpi / self.dpi, picture_size([rect[2], rect[3]], dpi), origin)
    }
    pub(crate) fn thumbnail(&self) -> Picture {
        let dpi = (2400.0 * 25.4 / self.paper_mm[0].max(self.paper_mm[1])) as f32;
        self.region_mm([0.0, 0.0, self.paper_mm[0], self.paper_mm[1]], dpi.min(self.dpi))
    }
}

impl SheetScene {
    pub(crate) fn new(paper_mm: [f64; 2], px_per_mm: f32, dpi: f32) -> Self {
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::gui_theme_fonts::font_definitions());
        Self {
            ctx,
            textures: Textures::default(),
            paper_mm,
            dpi,
            px_per_mm,
            pixels_per_point: dpi / 25.4 / px_per_mm,
            tiles: TileSide::fresh(),
            pictures: PictureSet::made_at_once(),
            passes: 0,
            quiet: 0,
            requests: 0,
            started: Instant::now(),
            shapes: Vec::new(),
        }
    }

    pub(crate) fn ready(&self) -> bool {
        ready(self.passes, self.quiet, self.started.elapsed())
    }

    pub(crate) fn pass(&mut self, app: &mut StrataAppState) {
        let paper = Rect::from_min_size(Pos2::ZERO, Vec2::new(self.paper_mm[0] as f32, self.paper_mm[1] as f32) * self.px_per_mm);
        let mut raw = RawInput { screen_rect: Some(paper), max_texture_side: Some(16_384), time: Some(f64::from(self.passes) * 0.1), ..Default::default() };
        raw.viewports.entry(ViewportId::ROOT).or_default().native_pixels_per_point = Some(self.pixels_per_point);
        self.tiles.swap(app);
        app.ref_vectors.swap_pictures(&mut self.pictures);
        let px_per_mm = self.px_per_mm;
        let out = self.ctx.run(raw, |ctx| app.paint_sheet_for_print(ctx, paper, px_per_mm));
        app.ref_vectors.swap_pictures(&mut self.pictures);
        self.tiles.swap(app);
        self.textures.apply(&out.textures_delta);
        self.shapes = out.shapes;
        self.passes += 1;
        let requests = self.tiles.requests();
        let quiet = requests == self.requests && self.tiles.settled() && !self.pictures.still_drawing();
        self.quiet = if quiet { self.quiet + 1 } else { 0 };
        self.requests = requests;
    }

    /// The basemap (and any cadastral picture) that actually arrived, as a JPEG of the map
    /// frame. C++ still paints that raster inside the map (`PdfExportSettings.cpp:13`
    /// `rasterizeWholeImage = false`, `:25` only skips *tiled* raster draws). No picture
    /// was drawn when the cache is empty, so the PDF does not invent one.
    pub(crate) fn basemap_jpeg(&mut self, frame_mm: [f64; 4]) -> Option<(Vec<u8>, [u32; 2])> {
        if self.tiles.basemap.is_empty() && self.tiles.cadastral.is_empty() {
            return None;
        }
        let dpi = self.dpi;
        let raster = self.raster();
        let picture = raster.region_mm(frame_mm, dpi);
        let width = u32::try_from(picture.size[0]).ok()?;
        let height = u32::try_from(picture.size[1]).ok()?;
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
            .encode(&picture.rgb, width, height, image::ExtendedColorType::Rgb8)
            .ok()?;
        Some((jpeg, [width, height]))
    }

    pub(crate) fn raster(&mut self) -> RasterInput {
        RasterInput {
            primitives: self.ctx.tessellate(std::mem::take(&mut self.shapes), self.pixels_per_point),
            textures: std::mem::take(&mut self.textures),
            scale: self.pixels_per_point,
            size: picture_size(self.paper_mm, self.dpi),
            paper_mm: self.paper_mm,
            design_px_per_mm: self.px_per_mm,
            dpi: self.dpi,
        }
    }
}
