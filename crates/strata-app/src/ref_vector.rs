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

use crate::gui_tab_map::w2s;
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
/// The pictures of a sheet's PDF.
#[path = "ref_vector_print.rs"]
pub(crate) mod print;
/// The ribbon buttons that fetch such layers.
#[path = "ref_map.rs"]
pub(crate) mod map;
/// The 지적 button: its run and its row.
#[path = "cadastral_run.rs"]
pub(crate) mod cadastral;
#[path = "ref_vector_raster.rs"]
pub(crate) mod raster;
/// The app's side: legend rows, and the painting of every frame.
#[path = "ref_vector_show.rs"]
mod show;
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
    /// The look it was made in: once the layer has another, it serves only until the next one is there.
    look: Look,
}

/// A picture being made: the view and the look it is made for, and where it arrives with the seconds it took.
type Drawing = (RasterView, Look, Receiver<(Option<Raster>, f32)>);

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
    /// The world box of the map as it was painted last (`[min x, min y, max x, max y]`, work CRS).
    pub(crate) map_box: Option<[f64; 4]>,
}

impl Default for RefVectorLayers {
    fn default() -> Self {
        let runner: Runner = Arc::new(|work| drop(std::thread::spawn(work)));
        Self { layers: Vec::new(), runner, pictures_made: 0, labels: labels::Labels::default(), map_box: None }
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
        Self { layers, runner: Arc::clone(&self.runner), pictures_made: 0, labels: labels::Labels::default(), map_box: None }
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

    /// The file the layer of this row is drawn from.
    pub(crate) fn file_of(&self, key: &str) -> Option<&std::path::Path> {
        self.layers.iter().find(|l| l.key == key).map(|l| l.spec.file.as_path())
    }

    /// All that the layer of this row is drawn by.
    pub(crate) fn spec_of(&self, key: &str) -> Option<&data::LoadSpec> {
        self.layers.iter().find(|l| l.key == key).map(|l| &l.spec)
    }

    /// How the layer of this row is drawn (the legend menu offers what fits it).
    pub(crate) fn look_of(&self, key: &str) -> Option<&Look> {
        self.layers.iter().find(|l| l.key == key).map(|l| &l.spec.look)
    }

    /// Another look for a layer. It keeps what it read and shows its picture until the next one is made; a
    /// relief is read again (its light is made when it is read).
    pub(crate) fn set_look(&mut self, key: &str, look: Look) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.key == key && l.spec.look != look) {
            // Only the colours of heights are made for every picture; all else is made when the file is read.
            if !matches!(look, Look::Heights { .. }) {
                layer.shapes = Shapes::Wanted;
            }
            layer.spec.look = look;
        }
    }

    /// Other letters for a layer, or none: it is read again, with the text the letters need.
    pub(crate) fn set_label(&mut self, key: &str, label: Option<data::LabelLook>) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.key == key && l.spec.label != label) {
            (layer.shapes, layer.spec.label) = (Shapes::Wanted, label);
        }
    }

    /// The lines and rings of a layer that reach into a box of the work CRS, once the layer is read.
    pub(crate) fn runs_within(&self, key: &str, within: [f64; 4]) -> Vec<Vec<[f64; 2]>> {
        self.layers.iter().find(|l| l.key == key).map_or_else(Vec::new, |l| if let Shapes::Ready(data) = &l.shapes { data.runs_within(within) } else { Vec::new() })
    }

    /// The lowest and highest height an elevation layer holds inside a box of the work CRS.
    pub(crate) fn height_range(&self, key: &str, within: [f64; 4]) -> Option<(f32, f32)> {
        self.layers.iter().find(|l| l.key == key).and_then(|l| if let Shapes::Ready(data) = &l.shapes { data.height_range(within) } else { None })
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

    #[allow(clippy::too_many_arguments)] // a view of the map is made of this many things
    fn paint(&mut self, key: &str, ctx: &egui::Context, p: &egui::Painter, rect: Rect, vp: &strata_canvas2d::viewport::ViewportTransform, opacity: f32, paper_zoom: f32) -> Painted {
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
            match layer.drawing.as_ref().map(|(_, _, receive)| receive.try_recv()) {
                Some(Ok((Some(raster), secs))) => {
                    painted.drawn = Some((raster.view.width, raster.view.height, secs));
                    let image = ColorImage::from_rgba_premultiplied([raster.view.width as usize, raster.view.height as usize], &raster.rgba);
                    let texture = ctx.load_texture(format!("ref-vector-{key}"), image, TextureOptions::LINEAR);
                    let look = layer.drawing.take().map_or_else(|| layer.spec.look.clone(), |(_, look, _)| look);
                    layer.picture = Some(Picture { view: raster.view, texture, look });
                    self.pictures_made += 1;
                }
                Some(Ok((None, _)) | Err(TryRecvError::Disconnected)) => layer.failed = layer.drawing.take().map(|(asked, _, _)| asked),
                Some(Err(TryRecvError::Empty)) | None => {}
            }
            let (wanted, map) = view::wanted_view(rect, vp, scale, ctx.input(|i| i.max_texture_side), paper_zoom);
            let fresh = layer.picture.as_ref().is_some_and(|pic| pic.look == layer.spec.look && view::covers(&pic.view, &wanted, &map));
            let hopeless = layer.failed.as_ref().is_some_and(|failed| failed.same_as(&wanted));
            if fresh || hopeless || layer.drawing.is_some() {
                break;
            }
            let (send, receive) = channel();
            let (shapes, look, wake) = (data.clone(), layer.spec.look.clone(), ctx.clone());
            let made_in = look.clone();
            (self.runner)(Box::new(move || {
                let started = std::time::Instant::now();
                let picture = render(&shapes, &look, wanted);
                let _ = send.send((picture, started.elapsed().as_secs_f32()));
                wake.request_repaint();
            }));
            layer.drawing = Some((wanted, made_in, receive));
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
