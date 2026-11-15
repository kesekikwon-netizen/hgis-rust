//! The letters over the shapes of the reference layers (the unit symbols of the geology map).
//!
//! C++ lets QGIS write them (`target/cpp-ref/src/core/GeologyMapService.cpp:426-455`: the column 기호 over
//! the polygon, Malgun Gothic 8 pt in 40/40/40, a white edge of 0.6 mm at alpha 220). Here the places come
//! from the map engine (`strata-mapengine`: the shapes cut to the view by the `geo` crate, the labels
//! placed by the RustGIS engine): over the part of each shape that is in view, no label over another.
//! This file asks the engine off the UI thread whenever the view or the labelled layers change, and
//! writes what it placed last in the app's own font.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Rect};
use strata_canvas2d::viewport::ViewportTransform;
use strata_mapengine::{LabelEngine, LabelSource, PlacedText, ViewBox};

use crate::gui_tab_map::{paint_placed_labels, w2s, LabelDraw, LABEL_FONT_FAMILY};
use crate::ref_vector::data::{LabelLook, LoadSpec};
use crate::survey_style::{label_font_size_points, mm_to_points, qt_premultiplied};

#[cfg(test)]
#[path = "ref_vector_labels_tests.rs"]
mod tests;

/// Runs work off the UI thread.
pub(crate) type Runner = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

/// What the engine is asked: the work CRS, the labelled layers and the view.
#[derive(Debug, Clone, PartialEq)]
struct Asked {
    work_epsg: u32,
    sources: Vec<LabelSource>,
    view: ViewBox,
}

/// A layer's label look as the engine takes it: Qt points and millimetres become screen points.
pub(crate) fn source(key: &str, spec: &LoadSpec, look: &LabelLook) -> LabelSource {
    let [r, g, b] = look.ink;
    let look = strata_mapengine::LabelLook { column: look.column.to_owned(), size_pt: label_font_size_points(look.size_pt), ink: [r, g, b, 255], bold: look.bold, halo: Some((look.halo, mm_to_points(look.halo_mm))), closest_scale: look.closest_scale, inside_only: look.inside_only };
    LabelSource { key: key.to_owned(), file: spec.file.clone(), table: Some(spec.table.clone()), look }
}

/// The view the engine is asked about. On a sheet the screen shows the paper `paper_zoom` times as large;
/// the engine sees the paper at its own size, so that sizes in points and the scale are those of the paper.
pub(crate) fn engine_view(rect: Rect, vp: &ViewportTransform, paper_zoom: f32) -> ViewBox {
    let zoom = f64::from(paper_zoom);
    ViewBox { centre: vp.center_xy, metres_per_point: vp.meters_per_pixel * zoom, width_pt: f64::from(rect.width()) / zoom, height_pt: f64::from(rect.height()) / zoom }
}

/// The labels of the reference layers: the engine, what it placed last, and what is being asked.
#[derive(Default)]
pub(crate) struct Labels {
    engine: Arc<Mutex<LabelEngine>>,
    placed: Vec<PlacedText>,
    placed_for: Option<Asked>,
    asking: Option<(Asked, Receiver<Vec<PlacedText>>)>,
}

impl Labels {
    /// Whether the engine is still being asked (the scripted screenshots wait for its answer).
    pub(crate) fn is_busy(&self) -> bool {
        self.asking.is_some()
    }

    /// Writes the labels of these layers for this view. The engine is asked again when the view or the
    /// layers changed; until it answers, the labels placed last stay on their ground.
    #[allow(clippy::too_many_arguments)] // a view of the map is made of this many things
    #[cfg(test)] // the app paints through `paint_zoomed`; the tests of the map keep this form
    pub(crate) fn paint(&mut self, ctx: &egui::Context, p: &Painter, rect: Rect, vp: &ViewportTransform, work_epsg: u32, sources: Vec<LabelSource>, runner: &Runner) {
        self.paint_zoomed(ctx, p, rect, vp, work_epsg, sources, runner, 1.0);
    }

    /// The same for a view that shows paper `paper_zoom` times as large (the map frame of a sheet): the
    /// letters and their edge are those of the paper, enlarged with it.
    #[allow(clippy::too_many_arguments)] // a view of the map is made of this many things
    pub(crate) fn paint_zoomed(&mut self, ctx: &egui::Context, p: &Painter, rect: Rect, vp: &ViewportTransform, work_epsg: u32, sources: Vec<LabelSource>, runner: &Runner, paper_zoom: f32) {
        if sources.is_empty() {
            (self.placed, self.placed_for, self.asking) = (Vec::new(), None, None);
            return;
        }
        let wanted = Asked { work_epsg, sources, view: engine_view(rect, vp, paper_zoom) };
        // Twice at most: with work that runs at once the answer is there in the same frame.
        for _ in 0..2 {
            if let Some((asked, answer)) = &self.asking {
                match answer.try_recv() {
                    Ok(placed) => {
                        (self.placed, self.placed_for) = (placed, Some(asked.clone()));
                        self.asking = None;
                    }
                    Err(TryRecvError::Disconnected) => self.asking = None,
                    Err(TryRecvError::Empty) => break,
                }
            }
            if self.placed_for.as_ref() == Some(&wanted) || self.asking.is_some() {
                break;
            }
            let (send, answer) = channel();
            let (engine, asked, wake) = (Arc::clone(&self.engine), wanted.clone(), ctx.clone());
            runner(Box::new(move || {
                // What the engine cannot read or place is written nowhere: the shapes are still drawn.
                let placed = engine.lock().ok().and_then(|mut engine| engine.place(asked.work_epsg, &asked.sources, &asked.view).ok()).unwrap_or_default();
                let _ = send.send(placed);
                wake.request_repaint();
            }));
            self.asking = Some((wanted.clone(), answer));
        }
        for label in &self.placed {
            let bold = FontFamily::Name(LABEL_FONT_FAMILY.into());
            let family = if label.bold && ctx.fonts(|f| f.families().contains(&bold)) { bold } else { FontFamily::Proportional };
            let [r, g, b, a] = label.ink;
            let ink = Color32::from_rgba_unmultiplied(r, g, b, a);
            let galley = ctx.fonts(|fonts| fonts.layout_no_wrap(label.text.clone(), FontId::new(label.size_pt * paper_zoom, family), ink));
            let origin = w2s(rect, vp, label.at) - galley.rect.size() * 0.5;
            // A label that a pan has carried to the edge waits for the engine's next answer.
            if !rect.contains_rect(Rect::from_min_size(origin, galley.rect.size())) {
                continue;
            }
            let edge = |c: [u8; 4]| {
                let [r, g, b, a] = qt_premultiplied(c);
                Color32::from_rgba_premultiplied(r, g, b, a)
            };
            let (halo, width) = label.halo.map_or((Color32::TRANSPARENT, 0.0), |(c, width)| (edge(c), width * paper_zoom));
            paint_placed_labels(p, &[LabelDraw { galley, ink, angle: 0.0, origins: vec![origin] }], &[Some(0)], width, halo);
        }
    }
}
