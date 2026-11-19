//! Ratio and direct-fill search off the UI thread (`KaTrenchDialogPlan.cpp:121-144`).
//! The click only starts the worker. A later frame applies the cells if the inputs are unchanged.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use strata_geo::trench_plan::{build_for_target_ratio, build_in_area, Cell, PickedArea};

use crate::app_state::StrataAppState;

use super::{finish_plan, selected_survey_fids, spec_from_dialog};

pub(super) struct Planned {
    pub(super) cells: Vec<Cell>,
    pub(super) area_m2: f64,
    pub(super) target_pct: f64,
    pub(super) error: String,
    pub(super) append_keep: bool,
}

/// Shared with a cloned app: one receiver, so a clone does not take the result away.
#[derive(Clone)]
pub(crate) struct TrenchPlaceJob {
    stamp: String,
    rx: Arc<Mutex<Receiver<Planned>>>,
}

impl std::fmt::Debug for TrenchPlaceJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrenchPlaceJob")
            .field("stamp", &self.stamp)
            .finish_non_exhaustive()
    }
}

pub(super) fn place_stamp(app: &StrataAppState) -> String {
    let area = app
        .survey_tables
        .iter()
        .find(|t| t.table == "survey_area")
        .map(|t| {
            t.layer
                .features
                .iter()
                .map(|f| format!("{}:{}", f.fid, f.wkt_geom))
                .collect::<Vec<_>>()
                .join("|")
        })
        .unwrap_or_default();
    let selected = selected_survey_fids(app)
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{area}#{selected}#{}#{}#{}#{}#{}#{}#{}",
        app.modals.trench_auto_place,
        app.modals.trench_survey_kind_idx,
        app.modals.trench_azimuth_deg,
        app.modals.trench_balk_m,
        app.modals.trench_length.meters(),
        app.modals.trench_rows,
        app.modals.trench_cols
    )
}

pub(super) fn start_auto_plan(app: &mut StrataAppState, picked: &PickedArea) {
    let Some(area) = picked.area.clone() else {
        return;
    };
    let area_m2 = picked.area_m2;
    let kind = app.modals.trench_survey_kind_idx.min(2);
    let azimuth = app.modals.trench_azimuth_deg;
    let spec = spec_from_dialog(app);
    let stamp = place_stamp(app);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let planned = if kind == 2 {
            match build_in_area(&spec, Some(&area)) {
                Ok(cells) => Planned {
                    cells,
                    area_m2,
                    target_pct: 0.0,
                    error: String::new(),
                    append_keep: false,
                },
                Err(error) => Planned {
                    cells: Vec::new(),
                    area_m2,
                    target_pct: 0.0,
                    error,
                    append_keep: true,
                },
            }
        } else {
            let target_pct = if kind == 1 { 2.0 } else { 10.0 };
            let plan = build_for_target_ratio(Some(&area), target_pct, 2.0, azimuth);
            Planned {
                cells: plan.cells,
                area_m2,
                target_pct,
                error: plan.error,
                append_keep: false,
            }
        };
        let _ = tx.send(planned);
    });
    app.trench_place_job = Some(TrenchPlaceJob {
        stamp,
        rx: Arc::new(Mutex::new(rx)),
    });
    app.show_status_message("배치를 계산하는 중입니다…");
}

pub(crate) fn poll_trench_place(app: &mut StrataAppState, ctx: &eframe::egui::Context) {
    let Some(job) = app.trench_place_job.clone() else {
        return;
    };
    let received = job
        .rx
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv();
    match received {
        Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(30)),
        Err(TryRecvError::Disconnected) => {
            app.trench_place_job = None;
            app.show_status_message(
                "시굴격자 계산이 중단되었습니다. 다시 적용하세요. 기존 격자는 유지됩니다.",
            );
        }
        Ok(plan) => {
            app.trench_place_job = None;
            if job.stamp == place_stamp(app) {
                finish_plan(app, plan);
            } else {
                app.show_status_message(
                    "규격이 바뀌어 이번 계산은 적용하지 않았습니다. 기존 격자는 유지됩니다.",
                );
            }
        }
    }
}
