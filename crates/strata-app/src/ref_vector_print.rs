//! The pictures of the reference layers while a sheet is drawn for its PDF: the screen's pictures are set
//! aside, and the PDF's own are made at once (no thread), so that one pass of the sheet has them all.

use std::sync::Arc;

use super::labels::{Labels, Runner};
use super::raster::RasterView;
use super::{Drawing, Picture, RefVectorLayers};

/// A layer's picture, the one being made and the view no picture could be made for.
type Kept = (String, Option<Picture>, Option<Drawing>, Option<RasterView>);

/// The pictures of one side (the screen, or a PDF being drawn), by layer, with the way work is done there.
pub(crate) struct PictureSet {
    layers: Vec<Kept>,
    runner: Runner,
    labels: Labels,
    map_box: Option<[f64; 4]>,
}

impl PictureSet {
    /// The side of a PDF before its first pass: no pictures yet, and every piece of work done at once.
    pub(crate) fn made_at_once() -> Self {
        Self { layers: Vec::new(), runner: Arc::new(|work| work()), labels: Labels::default(), map_box: None }
    }

    /// Whether a picture of this side is still being made.
    pub(crate) fn still_drawing(&self) -> bool {
        self.layers.iter().any(|(_, _, drawing, _)| drawing.is_some())
    }
}

impl RefVectorLayers {
    /// Puts the pictures of `set` in the place of those in use, and gives those back in `set`.
    pub(crate) fn swap_pictures(&mut self, set: &mut PictureSet) {
        let mut taken = Vec::with_capacity(self.layers.len());
        for layer in &mut self.layers {
            taken.push((layer.key.clone(), layer.picture.take(), layer.drawing.take(), layer.failed.take()));
            if let Some(kept) = set.layers.iter_mut().find(|(key, ..)| *key == layer.key) {
                (layer.picture, layer.drawing, layer.failed) = (kept.1.take(), kept.2.take(), kept.3.take());
            }
        }
        set.layers = taken;
        std::mem::swap(&mut self.runner, &mut set.runner);
        std::mem::swap(&mut self.labels, &mut set.labels);
        std::mem::swap(&mut self.map_box, &mut set.map_box);
    }
}
