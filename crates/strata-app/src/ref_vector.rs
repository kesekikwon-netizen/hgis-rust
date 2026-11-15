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
use crate::gui_tab_map::w2s;
use crate::left_layer_panel::LayerTreeEntry;
use crate::ref_vector::data::{LoadSpec, Look};
use crate::ref_vector::elevation::{load, render, Drawn};
use crate::ref_vector::labels::Runner;
use crate::ref_vector::raster::{Raster, RasterView};

#[path = "ref_vector_clip.rs"]
pub(crate) mod clip;
#[path = "ref_vector_data.rs"]
pub(crate) mod data;
#[path = "ref_vector_elevation.rs"]
pub(crate) mod elevation;
#[path = "ref_vector_labels.rs"]
pub(crate) mod labels;
/// The ribbon buttons that fetch such layers.
#[path = "ref_map.rs"]
pub(crate) mod map;
#[path = "ref_vector_raster.rs"]
pub(crate) mod raster;
#[path = "ref_vector_stroke.rs"]
pub(crate) mod stroke;
#[path = "ref_vector_view.rs"]
mod view;
#[cfg(test)]
#[path = "ref_vector_tests.rs"]
mod tests;

/// What the status line says when the file of a layer cannot be read (`MainWindowOverlay.cpp`, 「지질도 표시
/// 실패」 default text; docs/senior/research/post/geology.md rule 41).
pub(crate) const OPEN_FAILED: &str = "받은 지도를 열지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요.";

enum Shapes {
    Wanted,
    Loading(Receiver<(Result<Drawn, String>, f32)>),
    Ready(Drawn),
    Failed,
}

struct Picture {
    view: RasterView,
    texture: egui::TextureHandle,
}

/// A picture being made: the view it is made for, and where it arrives with the seconds it took.
type Drawing = (RasterView, Receiver<(Option<Raster>, f32)>);

struct Layer {
    key: String,
    spec: LoadSpec,
    shapes: Shapes,
    picture: Option<Picture>,
    drawing: Option<Drawing>,
    /// A view no picture could be made for: it is not asked for again in every frame.
    failed: Option<RasterView>,
}

impl Layer {
    fn new(key: String, spec: LoadSpec) -> Self {
        Self { key, spec, shapes: Shapes::Wanted, picture: None, drawing: None, failed: None }
    }
}

/// The reference vector layers of the open survey.
pub(crate) struct RefVectorLayers {
    layers: Vec<Layer>,
    runner: Runner,
    pictures_made: u32,
    labels: labels::Labels,
}

impl Default for RefVectorLayers {
    fn default() -> Self {
        let runner: Runner = Arc::new(|work| drop(std::thread::spawn(work)));
        Self { layers: Vec::new(), runner, pictures_made: 0, labels: labels::Labels::default() }
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
        let layers = self.layers.iter().map(|l| Layer::new(l.key.clone(), l.spec.clone())).collect();
        Self { layers, runner: Arc::clone(&self.runner), pictures_made: 0, labels: labels::Labels::default() }
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
    /// What the map engine needs to label the layer, once its picture is on the map.
    labelled: Option<strata_mapengine::LabelSource>,
}

impl RefVectorLayers {
    pub(crate) fn has(&self, key: &str) -> bool {
        self.layers.iter().any(|l| l.key == key)
    }

    /// Whether a layer is still being read or drawn (the scripted screenshots wait for it).
    pub(crate) fn is_busy(&self) -> bool {
        self.labels.is_busy() || self.layers.iter().any(|l| matches!(l.shapes, Shapes::Loading(_)) || l.drawing.is_some())
    }

    /// The box of what a layer draws in the work CRS, once it is read (「선택한 레이어로 이동」).
    pub(crate) fn extent_of(&self, key: &str) -> Option<[f64; 4]> {
        self.layers.iter().find(|l| l.key == key).and_then(|l| if let Shapes::Ready(data) = &l.shapes { data.extent() } else { None })
    }

    /// How the layer of this row is drawn (the legend menu offers what fits it).
    pub(crate) fn look_of(&self, key: &str) -> Option<&Look> {
        self.layers.iter().find(|l| l.key == key).map(|l| &l.spec.look)
    }

    /// Layers are kept in the work CRS: when the user changes it they are read again into the new one.
    fn follow_work_crs(&mut self, work_epsg: u32) {
        for layer in self.layers.iter_mut().filter(|l| l.spec.work_epsg != work_epsg) {
            *layer = Layer::new(layer.key.clone(), LoadSpec { work_epsg, ..layer.spec.clone() });
        }
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
                    (painted.rows, painted.read_secs) = (data.rows(), Some(secs));
                    layer.shapes = Shapes::Ready(data);
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
            match layer.drawing.as_ref().map(|(_, receive)| receive.try_recv()) {
                Some(Ok((Some(raster), secs))) => {
                    painted.drawn = Some((raster.view.width, raster.view.height, secs));
                    let image = ColorImage::from_rgba_premultiplied([raster.view.width as usize, raster.view.height as usize], &raster.rgba);
                    let texture = ctx.load_texture(format!("ref-vector-{key}"), image, TextureOptions::LINEAR);
                    layer.picture = Some(Picture { view: raster.view, texture });
                    layer.drawing = None;
                    self.pictures_made += 1;
                }
                Some(Ok((None, _)) | Err(TryRecvError::Disconnected)) => layer.failed = layer.drawing.take().map(|(asked, _)| asked),
                Some(Err(TryRecvError::Empty)) | None => {}
            }
            let (wanted, map) = view::wanted_view(rect, vp, scale, ctx.input(|i| i.max_texture_side));
            let fresh = layer.picture.as_ref().is_some_and(|pic| view::covers(&pic.view, &wanted, &map));
            let hopeless = layer.failed.as_ref().is_some_and(|failed| failed.same_as(&wanted));
            if fresh || hopeless || layer.drawing.is_some() {
                break;
            }
            let (send, receive) = channel();
            let (shapes, wake) = (data.clone(), ctx.clone());
            (self.runner)(Box::new(move || {
                let started = std::time::Instant::now();
                let picture = render(&shapes, wanted);
                let _ = send.send((picture, started.elapsed().as_secs_f32()));
                wake.request_repaint();
            }));
            layer.drawing = Some((wanted, receive));
        }
        if let Some(pic) = &layer.picture {
            let world = pic.view.world();
            let on_screen = Rect::from_min_max(w2s(rect, vp, [world[0], world[3]]), w2s(rect, vp, [world[2], world[1]]));
            // The picture is premultiplied: fading multiplies every channel.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let tint = Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0).round() as u8);
            p.image(pic.texture.id(), on_screen, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
            painted.labelled = layer.spec.label.as_ref().map(|look| labels::source(key, &layer.spec, look));
        }
        painted
    }
}

impl StrataAppState {
    /// Puts the layer on the map: its legend row, checked, above the tile maps and the background. A layer of
    /// that key that is there already now draws the new file; its row keeps what the user set on it.
    pub(crate) fn show_reference_vector_layer(&mut self, key: &str, title: &str, spec: LoadSpec) {
        self.ref_vectors.layers.retain(|l| l.key != key);
        self.ref_vectors.layers.push(Layer::new(key.to_owned(), spec));
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
        self.ref_vectors.follow_work_crs(strata_types::crs::CrsCode::from(self.work_crs).epsg_number());
        self.ref_vectors.release(&rows, &shown_keys);
        let viewport = self.home_map.viewport;
        // Labels go over every reference picture, placed among all layers at once by the map engine.
        let mut labelled = Vec::new();
        for (key, opacity) in &shown {
            let mut painted = self.ref_vectors.paint(key, ctx, p, rect, &viewport, *opacity);
            labelled.extend(painted.labelled.take());
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
        let (runner, work_epsg) = (Arc::clone(&self.ref_vectors.runner), strata_types::crs::CrsCode::from(self.work_crs).epsg_number());
        self.ref_vectors.labels.paint(ctx, p, rect, &viewport, work_epsg, labelled, &runner);
    }
}
