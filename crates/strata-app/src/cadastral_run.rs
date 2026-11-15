//! The 지적 button: the digital cadastral map within 5 km of the survey area is received from VWorld in a
//! thread, the bottom gauge follows the run, and the parcels become a row of their own at the top of the
//! legend (`target/cpp-ref/src/app/MainWindowCadastral.cpp:82-175`). The run itself is
//! `strata_refmap::cadastral::portal::receive`; no window opens (docs/RUST_PLAN.md 2.8, 2.9).

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use strata_net::vworld_portal::CurlPortal;
use strata_refmap::cadastral::portal::{receive, Asked, PortalStop, Why};
use strata_refmap::cadastral::Ready;
use strata_types::crs::CrsCode;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::download_jobs::{DownloadJobHandle, DownloadJobId, DownloadJobState, StopCode};

/// The row, its look and the note of what was made.
#[path = "cadastral_layer.rs"]
pub(crate) mod layer;
pub(crate) use layer::{Style, CADASTRAL_KEY};
use layer::{kept_table, write_kept};

#[cfg(test)]
#[path = "cadastral_run_tests.rs"]
mod tests;

/// The gauge line (`MainWindowCadastral.cpp:170`) and the distance (`CadastralPortal.h:14`).
const JOB_TITLE: &str = "조사 주변 지적도";
const RADIUS_M: f64 = 5000.0;
const STARTING: &str = "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다.";
const ADDED: &str = "조사 주변 5km 지적도를 추가했습니다. 지적도 옆 메뉴에서 선 색·지번 표시를 바꿀 수 있습니다.";
const KEPT: &str = "저장된 조사 주변 지적도를 사용합니다.";
const CANCELLED: &str = "지적도 받기를 취소했습니다.";
const BUSY: &str = "지도 자료를 준비 중입니다. 완료를 기다리거나 취소하세요.";
const NEEDS_SURVEY: &str = "먼저 조사를 열거나 저장하고 조사구역을 그려 주세요.";
const NEEDS_AREA: &str = "조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다.";
const NO_KEY: &str = "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요.";
const NO_ACCOUNT: &str = "지적도 메뉴에서 VWorld 계정을 설정하세요.";
/// This app's own words (the C++ has no offline switch).
const OFFLINE: &str = "인터넷 접속이 꺼져 있어 VWorld에 연결할 수 없습니다.";
const NO_SESSION: &str = "VWorld 접속을 준비하지 못했습니다.";
/// What a run is started with. The key and the account live in this press only.
pub(crate) struct Wanted {
    pub(crate) areas: Vec<String>,
    pub(crate) work_epsg: u32,
    pub(crate) key: String,
    pub(crate) username: String,
    pub(crate) password: String,
    /// `<survey folder>/지적도`.
    pub(crate) directory: PathBuf,
    /// The look of the row at the press: a run keeps it.
    pub(crate) style: Style,
}

/// Does the run: asked for a stop through the second argument, it tells its stage through the third.
type Run = Arc<dyn Fn(&Wanted, &dyn Fn() -> bool, &mut dyn FnMut(u8, &str)) -> Result<Ready, PortalStop> + Send + Sync>;
/// Runs work off the UI thread.
type Runner = Arc<dyn Fn(Box<dyn FnOnce() + Send>) + Send + Sync>;

struct Running {
    id: DownloadJobId,
    job: DownloadJobHandle,
    result: Receiver<Result<Ready, PortalStop>>,
}

/// The cadastral run of the app: at most one.
pub(crate) struct CadastralJobs {
    wanted: Option<Wanted>,
    /// A press that cannot start, for the gauge: its code and words.
    refused: Option<(StopCode, &'static str)>,
    running: Option<Running>,
    run: Run,
    runner: Runner,
    online: fn() -> bool,
    vworld_key: fn() -> String,
    /// The stored account of the VWorld site: name and password.
    account: fn() -> (String, String),
    /// What 「선 색·지번 표시」 shows while it is open.
    pub(crate) style: Style,
}

fn stored_account() -> (String, String) {
    use strata_secrets::accounts::{AccountKind, AccountSettings};
    let account = AccountSettings::read_from_dir(&AccountSettings::user_config_dir(), AccountKind::VworldCadastral);
    (account.username, account.password)
}

impl Default for CadastralJobs {
    fn default() -> Self {
        let run: Run = Arc::new(|wanted, stop, stage| {
            let mut site = CurlPortal::open().map_err(|_| PortalStop::Failed { why: Why::Storage, words: NO_SESSION.to_owned() })?;
            let asked = Asked { survey: &wanted.areas, work_epsg: wanted.work_epsg, radius_m: RADIUS_M, api_key: &wanted.key, username: &wanted.username, password: &wanted.password, directory: &wanted.directory };
            receive(&asked, &mut site, stop, stage)
        });
        Self { wanted: None, refused: None, running: None, run, runner: Arc::new(|work| drop(std::thread::spawn(work))), online: strata_net::network::is_allowed, vworld_key: strata_secrets::vworld::VworldSettings::load_api_key, account: stored_account, style: Style::default() }
    }
}

impl std::fmt::Debug for CadastralJobs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CadastralJobs").field("running", &self.running.is_some()).finish()
    }
}

/// A copy of the app state carries no run.
impl Clone for CadastralJobs {
    fn clone(&self) -> Self {
        Self { wanted: None, refused: None, running: None, run: Arc::clone(&self.run), runner: Arc::clone(&self.runner), online: self.online, vworld_key: self.vworld_key, account: self.account, style: self.style }
    }
}

impl CadastralJobs {
    /// Whether a press is still being worked on (the scripted screenshots wait for its end).
    pub(crate) fn is_busy(&self) -> bool {
        self.wanted.is_some() || self.refused.is_some() || self.running.is_some()
    }
}

/// The gauge code of each kind of stop (docs/RUST_PLAN.md 2.8: one fixed code for each reason).
fn stop_code(why: Why) -> StopCode {
    match why {
        Why::Area => StopCode::Area1,
        Why::NoKey => StopCode::Key1,
        Why::KeyRefused => StopCode::Key2,
        Why::NoAccount => StopCode::Acc1,
        Why::AccountRefused => StopCode::Acc2,
        Why::Site => StopCode::Site1,
        Why::Storage => StopCode::Dir2,
        Why::Data => StopCode::Data1,
        Why::NoData => StopCode::None1,
    }
}

impl StrataAppState {
    /// The ribbon button 지적 (`MainWindowCadastral.cpp:82-133`). Every press runs, as in the C++: a run
    /// finds what is in the survey folder and receives only what is new. Without the row on the map the
    /// table kept for these very survey areas is put back first, without asking the site.
    pub(crate) fn press_cadastral(&mut self) {
        self.active_tab = MainTab::Map;
        if self.cadastral.is_busy() || self.ref_maps.is_busy() {
            return self.show_status_message(BUSY);
        }
        let Some(folder) = crate::survey_folder::survey_folder(self.survey_path.as_deref()) else {
            return self.show_status_message(NEEDS_SURVEY);
        };
        let areas = match crate::ref_vector::map::survey_area_wkts(self) {
            Ok(areas) if !areas.is_empty() => areas,
            Ok(_) => return self.show_status_message(NEEDS_AREA),
            Err(reason) => return self.show_status_message(reason),
        };
        let (work_epsg, directory) = (CrsCode::from(self.work_crs).epsg_number(), folder.join("지적도"));
        if !self.left_layers.entries.iter().any(|e| e.layer_key == CADASTRAL_KEY) {
            if let Some((file, style)) = kept_table(&directory, &areas, work_epsg, RADIUS_M) {
                self.show_cadastral_layer(file, Some(style));
                return self.show_status_message(KEPT);
            }
        }
        let key = (self.cadastral.vworld_key)();
        if key.trim().is_empty() {
            self.cadastral.refused = Some((StopCode::Key1, NO_KEY));
            return self.show_status_message(NO_KEY);
        }
        // The C++ asks for the account here and goes on when it was given (`:116-120`); here the account
        // entry opens and the next press runs.
        let (username, password) = (self.cadastral.account)();
        if username.is_empty() || password.is_empty() {
            self.modals.active = Some(ActiveModalDialog::AccountSettings);
            return self.show_status_message(NO_ACCOUNT);
        }
        if !(self.cadastral.online)() {
            self.cadastral.refused = Some((StopCode::Site1, OFFLINE));
            return self.show_status_message(OFFLINE);
        }
        self.show_status_message(STARTING);
        let style = self.ref_vectors.spec_of(CADASTRAL_KEY).map(Style::of).unwrap_or_default();
        self.cadastral.wanted = Some(Wanted { areas, work_epsg, key, username, password, directory, style });
    }

    /// A run belongs to the survey it was started for: when another survey takes its place it is told to
    /// stop and its outcome is not waited for (`MainWindowCadastral.cpp:138`).
    pub(crate) fn drop_cadastral_run(&mut self) {
        (self.cadastral.wanted, self.cadastral.refused) = (None, None);
        if let Some(running) = self.cadastral.running.take() {
            self.download_jobs.request_cancel(running.id);
            running.job.report_cancelled();
        }
    }

    /// Every frame: starts a wanted run, and takes the outcome of a running one.
    pub(crate) fn drive_cadastral(&mut self, ctx: &eframe::egui::Context, now: f64) {
        if let Some((code, reason)) = self.cadastral.refused.take() {
            drop(self.download_jobs.add_job(JOB_TITLE, DownloadJobState::Stopped { code, reason: reason.to_owned() }, now));
        }
        if let Some(wanted) = self.cadastral.wanted.take() {
            let (id, job) = self.download_jobs.add_job(JOB_TITLE, DownloadJobState::Running { fraction: Some(0.0), stage: Some(STARTING.to_owned()) }, now);
            let (send, result) = channel();
            let (run, worker_job, wake) = (Arc::clone(&self.cadastral.run), job.clone(), ctx.clone());
            (self.cadastral.runner)(Box::new(move || {
                let outcome = run(&wanted, &|| worker_job.is_cancelled(), &mut |value, words| worker_job.report_running(Some(f32::from(value) / 100.0), Some(words.to_owned())));
                if let Ok(ready) = &outcome {
                    write_kept(&wanted.directory, &wanted.areas, wanted.work_epsg, RADIUS_M, &ready.file, wanted.style);
                }
                let _ = send.send(outcome);
                wake.request_repaint();
            }));
            self.cadastral.running = Some(Running { id, job, result });
        }
        let Some(running) = self.cadastral.running.as_ref() else {
            return;
        };
        let outcome = match running.result.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return,
            // The worker ended without an answer: the gauge turns the job into APP-1 by itself.
            Err(TryRecvError::Disconnected) => {
                self.cadastral.running = None;
                return;
            }
        };
        let Some(Running { job, .. }) = self.cadastral.running.take() else {
            return;
        };
        if self.snap.is_some() {
            eprintln!("[SNAP] {JOB_TITLE} {:?}", outcome.as_ref().map(|ready| (ready.parcels, ready.reused)));
        }
        match outcome {
            Ok(ready) => {
                self.show_cadastral_layer(ready.file, None);
                job.report_done(ADDED);
                self.show_status_message(ADDED);
            }
            Err(PortalStop::Cancelled) => {
                job.report_cancelled();
                self.show_status_message(CANCELLED);
            }
            Err(PortalStop::Failed { why, words }) => {
                // The account is offered right where the login failed (`:142-155`).
                if why == Why::AccountRefused {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                }
                job.report_stopped(stop_code(why), words);
            }
        }
    }
}

#[cfg(test)]
impl CadastralJobs {
    /// The run is `run`, work happens inside the call that asks, the network counts as on, and a key and
    /// an account are there (none is read from the PC).
    pub(crate) fn for_test(run: Run) -> Self {
        Self { wanted: None, refused: None, running: None, run, runner: Arc::new(|work| work()), online: || true, vworld_key: || "KEY".to_owned(), account: || ("field".to_owned(), "secret".to_owned()), style: Style::default() }
    }

    pub(crate) fn set_key(&mut self, key: fn() -> String) {
        self.vworld_key = key;
    }

    pub(crate) fn set_account(&mut self, account: fn() -> (String, String)) {
        self.account = account;
    }

    pub(crate) fn set_online(&mut self, online: fn() -> bool) {
        self.online = online;
    }

    /// Work is kept and never done: a run stays running.
    pub(crate) fn hold_runs(&mut self) {
        let held = std::sync::Mutex::new(Vec::new());
        self.runner = Arc::new(move |work| held.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(work));
    }
}
