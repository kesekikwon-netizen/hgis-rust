//! One whole-run gauge (`target/cpp-ref/src/app/MainWindow.cpp:3135-3148`, owner exception HER-API1).

use std::path::PathBuf;

use strata_net::heritage_auto::HeritageConfig;
use strata_refmap::heritage::DATASETS;

use crate::app_state::StrataAppState;
use crate::download_jobs::DownloadJobHandle;

#[path = "heritage_download_import.rs"]
mod import;
use import::{Reply, Task, Worker};
#[cfg(test)]
#[path = "heritage_download_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "heritage_download_worker_tests.rs"]
mod worker_tests;

#[derive(Clone)]
pub(super) struct Context {
    pub(super) survey: PathBuf,
    pub(super) sido: String,
    pub(super) city: String,
    pub(super) root: PathBuf,
    /// Coordinates of the captured scope and output layers, EPSG:5187 or EPSG:5186.
    pub(super) work_epsg: u32,
    pub(super) account: crate::heritage_receipt::AccountState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Pending,
    Importing,
    Imported,
    Absent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Receiving,
    Importing,
    Settled,
}

pub(crate) struct Download {
    pub(super) context: Context,
    pub(super) config: Option<HeritageConfig>,
    pub(super) job: DownloadJobHandle,
    worker: Worker,
    kinds: [Kind; DATASETS.len()],
    phase: Phase,
    receipts: usize,
    last_receiving_note: String,
}

impl Download {
    pub(crate) fn set_fixture_base(&mut self, base: String) {
        if let Some(config) = self.config.as_mut() {
            config.fixture_base = Some(base);
        }
    }
    #[cfg(windows)]
    pub(crate) fn browser_options(
        &mut self,
        data_parent: PathBuf,
    ) -> Option<strata_heritage_run::RunOptions> {
        Some(strata_heritage_run::RunOptions {
            data_parent,
            config: self.config.take()?,
            network_allowed: std::sync::Arc::new(strata_net::network::is_allowed),
            download_root: self.context.root.clone(),
        })
    }

    pub(crate) fn new(
        app: &StrataAppState,
        scope: strata_engine::Scope,
        job: DownloadJobHandle,
        stage: String,
    ) -> Self {
        let context = Context {
            survey: app.survey_path.clone().unwrap_or_default(),
            sido: app.modals.heritage_sido.clone(),
            city: app.modals.heritage_city.clone(),
            root: app.heritage_archive_root(),
            work_epsg: app.work_crs.epsg_code(),
            account: crate::heritage_run_start::receipt_account_state(app),
        };
        let config = HeritageConfig {
            account_id: app.accounts.heritage_creds.username.clone(),
            account_password: app.accounts.heritage_creds.password.clone(),
            sido: context.sido.clone(),
            city: context.city.clone(),
            fixture_base: std::env::var("KA_HGIS_HERITAGE_FIXTURE").ok(),
        };
        let worker = Worker::start(context.clone(), scope, job.clone());
        Self {
            context,
            config: Some(config),
            job,
            worker,
            kinds: [Kind::Pending; DATASETS.len()],
            phase: Phase::Receiving,
            receipts: 0,
            last_receiving_note: stage,
        }
    }

    fn complete(&self) -> bool {
        self.phase == Phase::Importing
            && self.receipts == 0
            && self
                .kinds
                .iter()
                .all(|kind| matches!(kind, Kind::Imported | Kind::Absent))
    }
}

#[path = "heritage_download_drive.rs"]
mod drive;
#[path = "heritage_download_events.rs"]
mod events;
