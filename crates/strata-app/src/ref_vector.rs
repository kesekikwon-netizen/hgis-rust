//! Reference vector layers of the legend: downloaded maps kept as tables in the survey folder (sheet lines,
//! geology, soil, rivers, parcels).
//!
//! C++ adds such a file to the project as a read-only layer in the reference part of the legend
//! (`target/cpp-ref/src/app/KaTopographicImportDialog.cpp:871-995`, `src/core/GeologyMapService.cpp:924`).
//! Here a layer is its legend row plus a file: it is read and drawn only while the row is in the legend and
//! checked (owner 2026-11-15, docs/RUST_PLAN.md 2.9), off the UI thread, as one picture around the view.

use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use eframe::egui::{self, Color32, ColorImage, Rect, TextureOptions};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::gui_tab_map::{s2w, w2s};
use crate::left_layer_panel::LayerTreeEntry;
use crate::ref_vector::data::{load, LoadSpec, Look, RefVectorData};
use crate::ref_vector::raster::{render, Raster, RasterView};

#[path = "ref_vector_clip.rs"]
pub(crate) mod clip;
#[path = "ref_vector_data.rs"]
pub(crate) mod data;
#[path = "ref_vector_labels.rs"]
pub(crate) mod labels;
/// The ribbon buttons that fetch such layers.
#[path = "ref_map.rs"]
pub(crate) mod map;
#[path = "ref_vector_raster.rs"]
pub(crate) mod raster;
#[path = "ref_vector_stroke.rs"]
pub(crate) mod stroke;
#[cfg(test)]
#[path = "ref_vector_tests.rs"]
mod tests;

/// What the status line says when the file of a layer cannot be read (`MainWindowOverlay.cpp`, 「지질도 표시
/// 실패」 default text; docs/senior/research/post/geology.md rule 41).
pub(crate) const OPEN_FAILED: &str = "받은 지도를 열지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요.";

/// The picture reaches this many pixels beyond the map on every side: a small pan needs no new picture.
const MARGIN: f32 = 256.0;

/// Runs work off the UI thread.
type Runner = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

enum Shapes {
    Wanted,
    Loading(Receiver<(Result<RefVectorData, String>, f32)>),
    Ready(Arc<RefVectorData>),
    Failed,
}

struct Picture {
    view: RasterView,
    texture: egui::TextureHandle,
}

struct Layer {
    key: String,
    spec: LoadSpec,
    shapes: Shapes,
    picture: Option<Picture>,
    drawing: Option<Receiver<(Option<Raster>, f32)>>,
}

/// The reference vector layers of the open survey.
pub(crate) struct RefVectorLayers {
    layers: Vec<Layer>,
    runner: Runner,
    pictures_made: u32,
}

impl Default for RefVectorLayers {
    fn default() -> Self {
        let runner: Runner = Arc::new(|work| drop(std::thread::spawn(work)));
        Self { layers: Vec::new(), runner, pictures_made: 0 }
    }
}

impl std::fmt::Debug for RefVectorLayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.layers.iter().map(|l| &l.key)).finish()
    }
}

/// A copy of the app state knows which layers there are; shapes and pictures stay with the original.
impl Clone for RefVectorLayers {
    fn clone(&self) -> Self {
        let layers = self.layers.iter().map(|l| Layer { key: l.key.clone(), spec: l.spec.clone(), shapes: Shapes::Wanted, picture: None, drawing: None }).collect();
        Self { layers, runner: Arc::clone(&self.runner), pictures_made: 0 }
    }
}

/// What painting a layer found out, for the legend row and the status line.
#[derive(Default)]
struct Painted {
    rows: Option<u64>,
    failed_now: bool,
    /// Seconds the reading took, and the size and seconds of a picture that arrived in this frame.
    read_secs: Option<f32>,
    drawn: Option<(u32, u32, f32)>,
    /// The shapes of a layer whose picture is on the map and whose rows are labelled.
    labelled: Option<(Arc<RefVectorData>, data::LabelLook)>,
}

impl RefVectorLayers {
    pub(crate) fn has(&self, key: &str) -> bool {
        self.layers.iter().any(|l| l.key == key)
    }

    /// Whether a layer is still being read or drawn (the scripted screenshots wait for it).
    pub(crate) fn is_busy(&self) -> bool {
        self.layers.iter().any(|l| matches!(l.shapes, Shapes::Loading(_)) || l.drawing.is_some())
    }

    /// Whether the layer of this row draws lines (the legend menu offers what fits the shape).
    pub(crate) fn draws_lines(&self, key: &str) -> Option<bool> {
        self.layers.iter().find(|l| l.key == key).map(|l| matches!(l.spec.look, Look::Lines { .. }))
    }

    /// Lets go of what is in memory for layers without a row (`rows`) and of the pictures of unchecked ones.
    fn release(&mut self, rows: &[&str], shown: &[&str]) {
        for layer in &mut self.layers {
            if !rows.contains(&layer.key.as_str()) {
                (layer.shapes, layer.picture, layer.drawing) = (Shapes::Wanted, None, None);
            } else if !shown.contains(&layer.key.as_str()) {
                (layer.picture, layer.drawing) = (None, None);
            }
        }
    }

    fn paint(&mut self, key: &str, ctx: &egui::Context, p: &egui::Painter, rect: Rect, vp: &strata_canvas2d::viewport::ViewportTransform, opacity: f32) -> Painted {
        let mut painted = Painted::default();
        let Some(layer) = self.layers.iter_mut().find(|l| l.key == key) else {
            return painted;
        };
        if matches!(layer.shapes, Shapes::Wanted) {
            let (send, receive) = channel();
            let (spec, wake) = (layer.spec.clone(), ctx.clone());
            (self.runner)(Box::new(move || {
                let started = std::time::Instant::now();
                let read = load(&spec);
                let _ = send.send((read, started.elapsed().as_secs_f32()));
                wake.request_repaint();
            }));
            layer.shapes = Shapes::Loading(receive);
        }
        if let Shapes::Loading(receive) = &layer.shapes {
            match receive.try_recv() {
                Ok((Ok(data), secs)) => {
                    (painted.rows, painted.read_secs) = (Some(data.rows), Some(secs));
                    layer.shapes = Shapes::Ready(Arc::new(data));
                }
                Ok((Err(_), _)) | Err(TryRecvError::Disconnected) => {
                    painted.failed_now = true;
                    layer.shapes = Shapes::Failed;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        let Shapes::Ready(data) = &layer.shapes else {
            return painted;
        };
        // Twice at most: with work that runs at once, the picture asked for below is there in the same frame.
        let scale = ctx.pixels_per_point();
        for _ in 0..2 {
            match layer.drawing.as_ref().map(Receiver::try_recv) {
                Some(Ok((Some(raster), secs))) => {
                    painted.drawn = Some((raster.view.width, raster.view.height, secs));
                    let image = ColorImage::from_rgba_premultiplied([raster.view.width as usize, raster.view.height as usize], &raster.rgba);
                    let texture = ctx.load_texture(format!("ref-vector-{key}"), image, TextureOptions::LINEAR);
                    layer.picture = Some(Picture { view: raster.view, texture });
                    layer.drawing = None;
                    self.pictures_made += 1;
                }
                Some(Ok((None, _)) | Err(TryRecvError::Disconnected)) => layer.drawing = None,
                Some(Err(TryRecvError::Empty)) | None => {}
            }
            let wanted = wanted_view(rect, vp, scale, ctx.input(|i| i.max_texture_side));
            let fresh = layer.picture.as_ref().is_some_and(|pic| covers(&pic.view, &wanted));
            if fresh || layer.drawing.is_some() {
                break;
            }
            let (send, receive) = channel();
            let (shapes, wake) = (Arc::clone(data), ctx.clone());
            (self.runner)(Box::new(move || {
                let started = std::time::Instant::now();
                let picture = render(&shapes, wanted);
                let _ = send.send((picture, started.elapsed().as_secs_f32()));
                wake.request_repaint();
            }));
            layer.drawing = Some(receive);
        }
        if let Some(pic) = &layer.picture {
            let world = pic.view.world();
            let on_screen = Rect::from_min_max(w2s(rect, vp, [world[0], world[3]]), w2s(rect, vp, [world[2], world[1]]));
            // The picture is premultiplied: fading multiplies every channel.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let tint = Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
            p.image(pic.texture.id(), on_screen, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
            painted.labelled = layer.spec.label.map(|look| (Arc::clone(data), look));
        }
        painted
    }
}

/// The picture to make for this view of the map: the map's own pixels plus the margin on every side.
fn wanted_view(rect: Rect, vp: &strata_canvas2d::viewport::ViewportTransform, scale: f32, max_side: usize) -> RasterView {
    let metres_per_pixel = vp.meters_per_pixel / f64::from(scale);
    let top_left = s2w(rect, vp, rect.left_top());
    // Whole pixels, and never more than the graphics card takes as one picture.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let side = |points: f32| ((points * scale).ceil() + 2.0 * MARGIN).min(max_side as f32).max(1.0) as u32;
    RasterView {
        left: top_left[0] - f64::from(MARGIN) * metres_per_pixel,
        top: top_left[1] + f64::from(MARGIN) * metres_per_pixel,
        metres_per_pixel,
        width: side(rect.width()),
        height: side(rect.height()),
        pixels_per_mm: crate::survey_style::mm_to_points(1.0) * scale,
    }
}

/// Whether a picture still serves: the same scale (to a thousandth), and the map itself (the wanted picture
/// without its margin) lies inside it.
fn covers(have: &RasterView, wanted: &RasterView) -> bool {
    let same_scale = (have.metres_per_pixel / wanted.metres_per_pixel - 1.0).abs() < 1e-3;
    let (have, want) = (have.world(), wanted.world());
    let margin = f64::from(MARGIN) * wanted.metres_per_pixel;
    same_scale && want[0] + margin >= have[0] && want[1] + margin >= have[1] && want[2] - margin <= have[2] && want[3] - margin <= have[3]
}

impl StrataAppState {
    /// Puts the layer on the map: its legend row, checked, above the tile maps and the background. A layer of
    /// that key that is there already now draws the new file; its row keeps what the user set on it.
    pub(crate) fn show_reference_vector_layer(&mut self, key: &str, title: &str, spec: LoadSpec) {
        self.ref_vectors.layers.retain(|l| l.key != key);
        self.ref_vectors.layers.push(Layer { key: key.to_owned(), spec, shapes: Shapes::Wanted, picture: None, drawing: None });
        if !self.left_layers.entries.iter().any(|e| e.layer_key == key) {
            let entries = &self.left_layers.entries;
            let at = entries.iter().position(|e| e.group == LegendGroup::Basemap || crate::ref_tile_layers::ref_tile_layer(&e.layer_key).is_some()).unwrap_or(entries.len());
            let row = LayerTreeEntry { layer_key: key.to_owned(), label_ko: title.to_owned(), group: LegendGroup::ReferenceMap, visible: true, opacity: 1.0, read_only: true, snap_enabled: false, feature_count: 0 };
            self.left_layers.entries.insert(at, row);
        }
        // The check mark is kept by the row's present name (the user may have renamed the row).
        if let Some(name) = self.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone()) {
            self.home_map.real_survey.layer_visible.insert(name, true);
        }
    }

    /// Rows of an earlier survey do not belong to the next one.
    pub(crate) fn remove_reference_vector_rows(&mut self) {
        let layers = std::mem::take(&mut self.ref_vectors.layers);
        self.left_layers.entries.retain(|e| !layers.iter().any(|l| l.key == e.layer_key));
    }

    /// Paints the checked reference vector rows, the lowest row first.
    pub(crate) fn paint_reference_vector_layers(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: Rect) {
        let rows: Vec<&str> = self.left_layers.entries.iter().map(|e| e.layer_key.as_str()).filter(|k| self.ref_vectors.has(k)).collect();
        let shown: Vec<(String, f32)> = self
            .left_layers
            .entries
            .iter()
            .rev()
            .filter(|e| self.ref_vectors.has(&e.layer_key) && self.home_map.real_survey.is_layer_visible(&e.label_ko))
            .map(|e| (e.layer_key.clone(), e.opacity))
            .collect();
        let shown_keys: Vec<&str> = shown.iter().map(|(key, _)| key.as_str()).collect();
        self.ref_vectors.release(&rows, &shown_keys);
        let viewport = self.home_map.viewport;
        // Labels go over every reference picture, chosen among all layers at once.
        let mut labels = labels::Pending::default();
        for (key, opacity) in &shown {
            let painted = self.ref_vectors.paint(key, ctx, p, rect, &viewport, *opacity);
            if let Some((data, look)) = &painted.labelled {
                labels.add(ctx, data, look, rect, &viewport);
            }
            if let Some(rows) = painted.rows {
                self.left_layers.update_feature_count_badge(key, usize::try_from(rows).unwrap_or(usize::MAX));
            }
            if painted.failed_now {
                self.show_status_message(OPEN_FAILED);
            }
            // A scripted run has nobody watching: how long the work took goes to its log.
            if self.snap.is_some() {
                if let (Some(rows), Some(secs)) = (painted.rows, painted.read_secs) {
                    eprintln!("[SNAP] 참조 벡터 {key}: {rows} rows read in {secs:.2} s");
                }
                if let Some((width, height, secs)) = painted.drawn {
                    eprintln!("[SNAP] 참조 벡터 {key}: picture {width} x {height} in {secs:.3} s");
                }
            }
        }
        labels.paint(p);
    }
}
