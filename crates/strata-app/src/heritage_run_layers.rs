//! Map results (`target/cpp-ref/src/app/MainWindow.cpp:3135-3143,3187-3225`).

use super::{Step, DONE, HERITAGE_JOB_TITLE, KEY_PREFIX, NOT_RECEIVED};
use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobHandle, StopCode};
use crate::ref_vector::data::{LabelLook, LoadSpec, Look, PolygonFill};
use strata_engine::ShapeKind;
use strata_refmap::heritage::{HeritageDataset, NAME_COLUMN, TABLE};
use strata_refmap::heritage_load::Loaded;
use strata_types::crs::CrsCode;

impl StrataAppState {
    /// Puts the layers of a kind on the map: an outline of 0.35 mm in the colour of the kind, never filled
    /// (a fill would hide the satellite picture, the parcels and the map sheets, `HeritageStyle.cpp:42-57,
    /// 134`); the names in bold 7 pt with a white edge, only closer than 1:10,000 (`:136, 174-180`,
    /// `LabelOps.cpp:30-44`).
    pub(crate) fn show_heritage_layers(&mut self, dataset: &HeritageDataset, layers: &[Loaded]) {
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let [red, green, blue] = dataset.colour;
        let stroke = [red, green, blue, 255];
        for layer in layers {
            let look = match layer.kind {
                ShapeKind::Polygons => Look::Polygons {
                    stroke,
                    width_mm: 0.35,
                    fill: PolygonFill::None,
                },
                ShapeKind::Lines => Look::Lines {
                    stroke,
                    width_mm: 0.35,
                },
            };
            let label = layer.named.then_some(LabelLook {
                column: NAME_COLUMN,
                size_pt: 7.0,
                ink: [31, 35, 40],
                halo_mm: 0.8,
                halo: [255, 255, 255, 230],
                bold: true,
                closest_scale: Some(10_000.0),
                inside_only: false,
            });
            let key = format!("{KEY_PREFIX}{}:{}", dataset.key, layer.title);
            self.show_reference_vector_layer(
                &key,
                &layer.title,
                LoadSpec {
                    file: layer.file.clone(),
                    table: TABLE.to_owned(),
                    work_epsg,
                    look,
                    label,
                },
            );
        }
    }

    /// Takes the rows of the button away: those of an earlier run do not stay under those of the next.
    pub(crate) fn remove_heritage_rows(&mut self) {
        self.remove_reference_vector_rows_of(KEY_PREFIX);
    }

    /// One word of the worker; true when the run is over. A kind says how many sites it put on the map
    /// (`:3220`); a kind that cannot be loaded stops the run (`KaHeritageBrowser.cpp:445-477`).
    pub(crate) fn take_heritage_step(&mut self, job: &DownloadJobHandle, step: Step) -> bool {
        if self.snap.is_some() {
            match &step {
                Step::Kind(dataset, Ok(load)) => eprintln!(
                    "[SNAP] {HERITAGE_JOB_TITLE} {} 줄 {} 곳 {} 말 {}",
                    dataset.name,
                    load.layers.len(),
                    load.layers.iter().map(|layer| layer.sites).sum::<u64>(),
                    load.messages.len()
                ),
                Step::Kind(dataset, Err(failure)) => eprintln!(
                    "[SNAP] {HERITAGE_JOB_TITLE} {} 멈춤 {}",
                    dataset.name, failure.reason
                ),
                Step::End { missing } => {
                    eprintln!("[SNAP] {HERITAGE_JOB_TITLE} 끝 없는 종류 {}", missing.len())
                }
            }
        }
        match step {
            Step::Kind(dataset, Ok(load)) => {
                self.show_heritage_layers(dataset, &load.layers);
                let sites: u64 = load.layers.iter().map(|layer| layer.sites).sum();
                let mut said = Vec::new();
                if !load.layers.is_empty() {
                    said.push(format!("{} {sites}곳을 올렸습니다.", dataset.name));
                }
                said.extend(load.messages);
                self.show_status_message(said.join(" "));
                false
            }
            Step::Kind(_, Err(failure)) => {
                self.show_status_message(failure.reason.clone());
                job.report_stopped(StopCode::Data1, failure.reason);
                true
            }
            Step::End { missing } => {
                let said = if missing.is_empty() {
                    DONE.to_owned()
                } else {
                    format!("{NOT_RECEIVED}: {}.", missing.join(", "))
                };
                self.show_status_message(said.clone());
                if missing.is_empty() {
                    job.report_done(said);
                } else {
                    job.report_stopped(StopCode::None1, said);
                }
                true
            }
        }
    }
}
