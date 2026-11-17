//! Captured raster work, target/cpp-ref/src/app/KaAlignMapTool.cpp:346-451 and KaAlignMapToolEdit.cpp:124-168.
use super::AlignPair;
use crate::app_state::StrataAppState;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use strata_engine::AlignedRaster;
use strata_geo::align_fit::AlignAffine;

#[derive(Debug, Clone)]
pub(crate) struct Original {
    pub key: String,
    pub file: PathBuf,
    pub transform: [f64; 6],
    pub source_epsg: Option<u32>,
    pub pending: bool,
    pub white: bool,
    pub visible: bool,
    pub opacity: f32,
}

#[derive(Debug)]
pub(crate) struct Task {
    pub receive: Receiver<Result<Outcome, String>>,
    pub expected_file: PathBuf,
    pub work_epsg: u32,
    pub pairs: Vec<AlignPair>,
    pub purpose: Purpose,
}

#[derive(Debug)]
pub(crate) enum Purpose {
    Apply { save: bool },
    Restore,
}

#[derive(Debug)]
pub(crate) enum Outcome {
    Applied(AlignedRaster, AlignAffine),
    Restored,
}

#[derive(Debug, Default)]
pub(crate) struct TaskSlot(pub Option<Task>);
impl Clone for TaskSlot {
    fn clone(&self) -> Self {
        Self(None)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RasterSession {
    pub original: Original,
    pub result: Option<AlignedRaster>,
    pub task: TaskSlot,
    pub view: crate::app_state::picture_layers::pixels::PixelView,
    pub source_rect: Option<eframe::egui::Rect>,
    pub map_rect: Option<eframe::egui::Rect>,
    pub selected: Option<usize>,
    pub drag: Option<(bool, usize)>,
}

impl RasterSession {
    pub(crate) fn capture(app: &StrataAppState, target: &str) -> Option<Self> {
        let layer = app.pictures.layers.iter().find(|l| l.key == target || app.left_layers.entries.iter().any(|r| r.layer_key == l.key && r.label_ko == target))?;
        let row = app.left_layers.entries.iter().find(|r| r.layer_key == layer.key)?;
        Some(Self {
            original: Original {
                key: layer.key.clone(),
                file: layer.file.clone(),
                transform: layer.transform,
                source_epsg: layer.source_epsg.or_else(|| (!layer.align_pending).then_some(strata_types::crs::CrsCode::from(app.work_crs).epsg_number())),
                pending: layer.align_pending,
                white: layer.knock_out_white,
                visible: row.visible,
                opacity: row.opacity,
            },
            result: None,
            task: TaskSlot::default(),
            view: crate::app_state::picture_layers::pixels::PixelView::new(layer),
            source_rect: None,
            map_rect: None,
            selected: None,
            drag: None,
        })
    }
}

impl StrataAppState {
    pub(crate) fn poll_align_task(&mut self, ctx: &eframe::egui::Context) {
        let Some(session) = &mut self.align.raster else { return };
        let Some(task) = session.task.0.take() else { return };
        let result = match task.receive.try_recv() {
            Err(TryRecvError::Empty) => {
                session.task.0 = Some(task);
                ctx.request_repaint_after(std::time::Duration::from_millis(30));
                return;
            }
            Err(TryRecvError::Disconnected) => Err("맞춰진 그림을 다시 열지 못했습니다".to_owned()),
            Ok(result) => result,
        };
        let key = session.original.key.clone();
        let current_epsg = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let restoring = matches!(&result, Ok(Outcome::Restored));
        let valid =
            (restoring || (task.work_epsg == current_epsg && task.pairs == self.align.pairs)) && self.pictures.layers.iter().any(|l| l.key == key && l.file == task.expected_file);
        if !valid {
            if let Ok(Outcome::Applied(out, _)) = result {
                std::thread::spawn(move || drop(out.restore()));
            }
            self.show_status_message("점을 다시 찍고 이동하세요.");
            return;
        }
        match result {
            Err(why) => self.show_status_message(format!("도면을 옮기지 못했습니다 — {why}")),
            Ok(Outcome::Restored) => {
                let old_result = self.align.raster.as_ref().and_then(|s| s.result.clone());
                self.restore_picture_placement();
                if let Some(layer) = self.pictures.layers.iter_mut().find(|layer| layer.key == key) {
                    layer.align_pending = true;
                }
                self.align.pairs.clear();
                self.align.pending_source = None;
                self.align.applied = false;
                self.align.transform = None;
                if self.printing.job.0.is_none() && self.drawing.pdf.0.is_none() {
                    if let Some(out) = old_result {
                        std::thread::spawn(move || drop(out.restore()));
                    }
                }
                self.show_status_message("도면을 맞추기 전 자리로 되돌렸습니다");
            }
            Ok(Outcome::Applied(out, fit)) => {
                let Some(layer) = self.pictures.layers.iter_mut().find(|l| l.key == key) else { return };
                layer.file = out.file().to_path_buf();
                layer.transform = [fit.c, fit.a, fit.b, fit.f, fit.d, fit.e];
                layer.source_epsg = Some(current_epsg);
                layer.align_pending = false;
                layer.clear_picture_cache();
                self.align.pending_view = layer.work_box(current_epsg).map(|extent| (current_epsg, extent));
                self.home_map.korea_view_pending = false;
                self.home_map.survey_view_pending = false;
                self.align.saved_path = Some(out.file().display().to_string());
                if let Some(session) = &mut self.align.raster {
                    session.result = Some(out);
                }
                if let Some(row) = self.left_layers.entries.iter_mut().find(|r| r.layer_key == key) {
                    row.visible = true;
                    row.opacity = 1.0;
                    self.home_map.real_survey.layer_visible.insert(row.label_ko.clone(), true);
                }
                self.align.applied = true;
                self.align.transform = Some(fit);
                self.is_dirty = true;
                if matches!(task.purpose, Purpose::Apply { save: true }) {
                    let path = self.align.saved_path.clone().unwrap_or_default();
                    self.stop_align_session();
                    self.show_status_message(format!("맞춤 저장: {path}"));
                } else {
                    self.show_status_message(format!("이동 완료 · {}점", self.align.pairs.len()));
                }
            }
        }
    }

    pub(crate) fn restore_picture_placement(&mut self) {
        let Some(session) = &mut self.align.raster else { return };
        let old = &session.original;
        if let Some(layer) = self.pictures.layers.iter_mut().find(|l| l.key == old.key) {
            layer.file = old.file.clone();
            layer.transform = old.transform;
            layer.source_epsg = old.source_epsg;
            layer.align_pending = old.pending;
            layer.knock_out_white = old.white;
            layer.clear_picture_cache();
        }
        if let Some(row) = self.left_layers.entries.iter_mut().find(|r| r.layer_key == old.key) {
            row.visible = old.visible;
            row.opacity = old.opacity;
            self.home_map.real_survey.layer_visible.insert(row.label_ko.clone(), old.visible);
        }
        session.result = None;
    }
}
