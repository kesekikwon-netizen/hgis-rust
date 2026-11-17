//! Region reads shared by map and alignment views, target/cpp-ref/src/app/KaAlignImageDetail.cpp:14-96.
use super::{knock_out_paper, pixel_at, PictureLayer, Wanted, MOST_SIDE};
use eframe::egui::{self, ColorImage, TextureOptions};
use std::sync::mpsc::{channel, TryRecvError};
use strata_engine::{read_picture, Transformer};

impl PictureLayer {
    /// The part of the picture the view `[min x, min y, max x, max y]` (work CRS, `mpp` metres a screen pixel) needs,
    /// with room around it so that a small move of the map needs no new reading; none when none of it is seen.
    pub(crate) fn wanted(&self, view: [f64; 4], mpp: f64, work_epsg: u32) -> Option<Wanted> {
        let in_source = match self.source_epsg {
            Some(epsg) if epsg != work_epsg => Transformer::new(work_epsg, epsg).and_then(|carry| carry.transform_box(view)).ok()?,
            _ => view,
        };
        let corners = [[in_source[0], in_source[1]], [in_source[2], in_source[1]], [in_source[2], in_source[3]], [in_source[0], in_source[3]]];
        let pixels: Vec<[f64; 2]> = corners.iter().map(|c| pixel_at(&self.transform, *c)).collect::<Option<_>>()?;
        let (lo_c, hi_c) = pixels.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
        let (lo_r, hi_r) = pixels.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p[1]), hi.max(p[1])));
        let (pad_c, pad_r) = ((hi_c - lo_c) * 0.5, (hi_r - lo_r) * 0.5);
        let (w, h) = (self.size[0] as f64, self.size[1] as f64);
        let (c0, c1) = ((lo_c - pad_c).floor().max(0.0), (hi_c + pad_c).ceil().min(w));
        let (r0, r1) = ((lo_r - pad_r).floor().max(0.0), (hi_r + pad_r).ceil().min(h));
        if c1 <= c0 || r1 <= r0 {
            return None;
        }
        // Screen pixels to a picture pixel; never more than the file has.
        let pixel_m = (self.transform[1] * self.transform[5] - self.transform[2] * self.transform[4]).abs().sqrt();
        let density = (pixel_m / mpp.max(1e-9)).min(1.0);
        let (cols, rows) = (c1 - c0, r1 - r0);
        let shrink = (MOST_SIDE as f64 / (cols * density).max(rows * density)).min(1.0);
        // Pixel counts well inside usize: the picture's own size bounds them.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (window, size) =
            ([c0 as usize, r0 as usize, cols as usize, rows as usize], [((cols * density * shrink).ceil() as usize).max(1), ((rows * density * shrink).ceil() as usize).max(1)]);
        Some(Wanted { window, size })
    }

    /// Whether what is shown serves the view: it holds the wanted part, at least nearly as finely.
    pub(super) fn serves(&self, want: &Wanted) -> bool {
        self.shown.as_ref().is_some_and(|(shown, _)| {
            let holds = shown.window[0] <= want.window[0]
                && shown.window[1] <= want.window[1]
                && shown.window[0] + shown.window[2] >= want.window[0] + want.window[2]
                && shown.window[1] + shown.window[3] >= want.window[1] + want.window[3];
            let fine = |s: &Wanted| s.size[0] as f64 / s.window[2] as f64;
            holds && fine(shown) >= fine(want) * 0.8
        })
    }

    pub(super) fn start_reading(&mut self, want: Wanted) {
        let (send, receive) = channel();
        let file = self.file.clone();
        std::thread::spawn(move || drop(send.send(read_picture(&file, want.window, want.size).map_err(|e| e.0))));
        self.reading = Some((want, receive));
    }

    /// What a reading brought, made a texture.
    pub(super) fn take_arrived(&mut self, ctx: &egui::Context) -> Option<String> {
        let Some((want, receive)) = &self.reading else { return None };
        match receive.try_recv() {
            Ok(Ok(mut rgba)) => {
                if self.knock_out_white {
                    knock_out_paper(&mut rgba);
                }
                let image = ColorImage::from_rgba_unmultiplied(want.size, &rgba);
                let texture = ctx.load_texture(format!("{}#{:?}", self.key, want.window), image, TextureOptions::LINEAR);
                self.shown = Some((*want, texture));
                self.reading = None;
            }
            // The file went away or broke: what was shown stays, nothing is asked again in every frame.
            Ok(Err(why)) => {
                self.reading = None;
                return Some(why);
            }
            Err(TryRecvError::Disconnected) => {
                self.reading = None;
                return Some("왼쪽에 그림을 띄우지 못했습니다.".to_owned());
            }
            Err(TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(60)),
        }
        None
    }
}
