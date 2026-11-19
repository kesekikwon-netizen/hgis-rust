//! Original-pixel image pane, target/cpp-ref/src/app/KaImageView.cpp:139-215 and KaAlignImageDetail.cpp:14-96.
use super::{PictureLayer, Wanted, MOST_SIDE};
use eframe::egui::{self, Pos2, Rect};

pub(crate) struct PixelView {
    reader: PictureLayer,
    pub centre: [f64; 2],
    pub zoom: f64,
    pub error: Option<String>,
}

impl Clone for PixelView {
    fn clone(&self) -> Self {
        let mut copy = Self::new(&self.reader);
        copy.centre = self.centre;
        copy.zoom = self.zoom;
        copy
    }
}
impl std::fmt::Debug for PixelView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelView").field("size", &self.reader.size).finish()
    }
}

impl PixelView {
    pub(crate) fn new(layer: &PictureLayer) -> Self {
        Self {
            reader: PictureLayer {
                key: layer.key.clone(),
                file: layer.file.clone(),
                size: layer.size,
                transform: layer.transform,
                source_epsg: layer.source_epsg,
                align_pending: true,
                knock_out_white: false,
                brightness: 0,
                cache_epoch: 0,
                shown: None,
                reading: None,
            },
            centre: [layer.size[0] as f64 * 0.5, layer.size[1] as f64 * 0.5],
            zoom: 1.0,
            error: None,
        }
    }
    pub(crate) fn scale(&self, rect: Rect) -> f64 {
        (f64::from(rect.width()) / self.reader.size[0] as f64).min(f64::from(rect.height()) / self.reader.size[1] as f64) * self.zoom
    }
    pub(crate) fn screen(&self, rect: Rect, pixel: [f64; 2]) -> Pos2 {
        let scale = self.scale(rect);
        Pos2::new(rect.center().x + ((pixel[0] - self.centre[0]) * scale) as f32, rect.center().y + ((pixel[1] - self.centre[1]) * scale) as f32)
    }
    pub(crate) fn pixel(&self, rect: Rect, at: Pos2) -> [f64; 2] {
        let scale = self.scale(rect);
        [self.centre[0] + f64::from(at.x - rect.center().x) / scale, self.centre[1] + f64::from(at.y - rect.center().y) / scale]
    }
    pub(crate) fn paint(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: Rect) {
        if let Some(why) = self.reader.take_arrived(ctx) {
            self.error = Some(why);
        }
        let (low, high) = (self.pixel(rect, rect.left_top()), self.pixel(rect, rect.right_bottom()));
        let first = [low[0].floor().max(0.0), low[1].floor().max(0.0)];
        let last = [high[0].ceil().min(self.reader.size[0] as f64), high[1].ceil().min(self.reader.size[1] as f64)];
        if last[0] > first[0] && last[1] > first[1] {
            let window = [first[0] as usize, first[1] as usize, (last[0] - first[0]) as usize, (last[1] - first[1]) as usize];
            let density = self.scale(rect).min(1.0);
            let shrink = (MOST_SIDE as f64 / (window[2].max(window[3]) as f64 * density)).min(1.0);
            let size = [((window[2] as f64 * density * shrink).ceil() as usize).max(1), ((window[3] as f64 * density * shrink).ceil() as usize).max(1)];
            let wanted = Wanted { window, size };
            if self.error.is_none() && self.reader.reading.is_none() && !self.reader.serves(&wanted) {
                self.reader.start_reading(wanted);
                ctx.request_repaint_after(std::time::Duration::from_millis(30));
            }
        }
        if let Some((shown, texture)) = &self.reader.shown {
            let [x, y, w, h] = shown.window.map(|v| v as f64);
            let target = Rect::from_min_max(self.screen(rect, [x, y]), self.screen(rect, [x + w, y + h]));
            p.with_clip_rect(rect).image(texture.id(), target, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), egui::Color32::WHITE);
        }
    }
}

#[cfg(test)]
#[path = "picture_layers_pixels_tests.rs"]
mod tests;
