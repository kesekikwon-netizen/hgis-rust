//! The work of the 「유산」 button on data received earlier (`HeritageRecentDownloads.cpp`,
//! `HeritageImport.cpp`): the archives of the chosen city that can be used again are unpacked into the app's
//! own data folder, and every kind is loaded off the UI thread, one after the other; each kind goes on the
//! map as soon as it is ready (`MainWindow.cpp:3135-3140`: the six are not waited for).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};

use chrono::NaiveDateTime;
use strata_engine::Scope;
use strata_refmap::heritage::{HeritageDataset, DATASETS};
use strata_refmap::heritage_load::{load_dataset, LoadFailure};
use strata_refmap::heritage_recent::{find, latest, region_folder};
use strata_topo::archive::{prepare, Limits};
use strata_types::crs::CrsCode;

use super::{Step, HERITAGE_JOB_TITLE};
use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobHandle, DownloadJobState, StopCode};

/// This app's own words, for the case where the intranet run cannot be started here.
///
/// It used to say "인트라넷에서 새로 받기는 준비 중입니다", which was true when no run existed.
/// The run exists now (`heritage_run_start.rs`), so the words say what actually blocks: the
/// account, a survey, or the city. The C++ goes straight to the intranet in this case.
const NOTHING_RECEIVED: &str = "이 시·군은 최근 30일 안에 받아 둔 주변유적 자료가 없습니다.";

/// What the dialog asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wanted {
    pub(crate) sido: String,
    pub(crate) city: String,
    /// 「최근 받은 자료 다시 쓰기」.
    pub(crate) reuse: bool,
}

/// The kinds of a run with their archives.
type Plan = Vec<(&'static HeritageDataset, Vec<PathBuf>)>;

/// Where the worker reads and writes.
struct Places {
    /// The unpacked archives (`MainWindow.cpp:3201-3204`: `<app data>/주변유적/SHP`).
    library: PathBuf,
    /// The copies of this run.
    work: PathBuf,
}

impl StrataAppState {
    /// The folder the received archives are kept under: the app's own data folder, not the survey's (the
    /// C++ keeps them out of folders that are synchronised, `MainWindow.cpp:3168-3170`).
    pub(crate) fn heritage_archive_root(&self) -> PathBuf {
        self.heritage.archive_root.clone().unwrap_or_else(|| self.accounts.config_dir.clone())
    }

    /// When the newest data of a city that can be used again was received; asked once for a city, not
    /// every frame of the dialog.
    pub(crate) fn heritage_reuse_when(&mut self, sido: &str, city: &str) -> Option<NaiveDateTime> {
        let asked = (sido.to_owned(), city.to_owned());
        if let Some((known, when)) = &self.heritage.asked {
            if *known == asked {
                return *when;
            }
        }
        let when = region_folder(&self.heritage_archive_root(), sido, city).and_then(|folder| latest(&folder, chrono::Local::now().naive_local()));
        self.heritage.asked = Some((asked, when));
        when
    }

    /// Starts a run for a city: the kinds that can be used again are loaded in a thread.
    pub(crate) fn begin_heritage_run(&mut self, wanted: &Wanted, now: f64) {
        let refuse = |app: &mut Self, code: StopCode, reason: String| drop(app.download_jobs.add_job(HERITAGE_JOB_TITLE, DownloadJobState::Stopped { code, reason }, now));
        let scope = match self.heritage_scope() {
            Ok(scope) => scope,
            Err((code, reason)) => return refuse(self, code, reason),
        };
        let root = self.heritage_archive_root();
        let at = chrono::Local::now().naive_local();
        let folder = region_folder(&root, &wanted.sido, &wanted.city);
        let plan: Plan = DATASETS.iter().map(|dataset| (dataset, folder.as_ref().filter(|_| wanted.reuse).map(|folder| find(folder, dataset, at)).unwrap_or_default())).collect();
        if plan.iter().all(|(_, archives)| archives.is_empty()) {
            return refuse(self, StopCode::None1, NOTHING_RECEIVED.to_owned());
        }
        self.drop_heritage_run();
        self.remove_heritage_rows();
        let library = root.join("주변유적").join("SHP");
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_millis());
        let copies = library.join("UTF8");
        // The copies of earlier runs are no longer drawn. One that is still held open stays until next time.
        for old in std::fs::read_dir(&copies).into_iter().flatten().flatten().filter(|entry| entry.file_name().to_string_lossy().starts_with("strata-")) {
            let _ = std::fs::remove_dir_all(old.path());
        }
        let places = Places { library, work: copies.join(format!("strata-{stamp}")) };
        let first = DownloadJobState::Running { fraction: Some(0.0), stage: Some("받아 둔 자료 여는 중".to_owned()) };
        let (_, job) = self.download_jobs.add_job(HERITAGE_JOB_TITLE, first, now);
        let (send, steps) = channel();
        let (worker_job, work_epsg) = (job.clone(), CrsCode::from(self.work_crs).epsg_number());
        std::thread::spawn(move || load_plan(&plan, &places, &scope, work_epsg, &worker_job, &send));
        self.heritage.running = Some((steps, job));
    }
}

/// The worker: every kind with archives, in the order of the kinds; a kind that fails ends the run.
fn load_plan(plan: &Plan, places: &Places, scope: &Scope, work_epsg: u32, job: &DownloadJobHandle, send: &Sender<Step>) {
    let cancelled = || job.is_cancelled();
    // The intranet names the files of its archives in CP949 (`HeritageImport.cpp:173-185`).
    let limits = Limits { korean_names: true, ..Limits::default() };
    let unpack = |archive: &Path| {
        let prepared = prepare(archive, &places.library, &limits, &cancelled);
        if prepared.error.is_empty() && !prepared.canceled {
            Ok(prepared.files)
        } else {
            Err(LoadFailure { reason: prepared.error, download_again: prepared.invalid_archive })
        }
    };
    let with_archives: Vec<_> = plan.iter().filter(|(_, archives)| !archives.is_empty()).collect();
    for (done, (dataset, archives)) in with_archives.iter().enumerate() {
        if cancelled() {
            return;
        }
        #[allow(clippy::cast_precision_loss)] // six kinds
        let fraction = done as f32 / with_archives.len() as f32;
        job.report_running(Some(fraction), Some(format!("{} 올리는 중 ({}/{})", dataset.name, done + 1, with_archives.len())));
        let outcome = load_dataset(dataset, archives, &unpack, Some((scope, work_epsg)), "", &places.work.join(dataset.key));
        let failed = outcome.is_err();
        if cancelled() || send.send(Step::Kind(dataset, outcome)).is_err() || failed {
            return;
        }
    }
    let missing = plan.iter().filter(|(_, archives)| archives.is_empty()).map(|(dataset, _)| dataset.name).collect();
    let _ = send.send(Step::End { missing });
}
