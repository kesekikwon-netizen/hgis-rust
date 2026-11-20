//! One whole-run gauge (`target/cpp-ref/src/app/MainWindow.cpp:3135-3148`, owner exception HER-API1).

use std::path::PathBuf;

use strata_net::heritage_auto::HeritageConfig;
#[cfg(any(windows, test))]
use strata_refmap::heritage::DATASETS;

use crate::app_state::StrataAppState;
use crate::download_jobs::DownloadJobHandle;

#[path = "heritage_download_import.rs"]
mod import;
#[cfg(test)]
use import::ImportFailure;
#[cfg(any(windows, test))]
use import::{Group, Task};
use import::{Reply, Worker};
#[cfg(test)]
#[path = "heritage_download_group_tests.rs"]
mod group_tests;
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
    /// The archive root. Read by the hidden-browser worker, which runs on Windows and in tests.
    #[cfg(any(windows, test))]
    pub(super) root: PathBuf,
    /// Coordinates of the captured scope and output layers, EPSG:5187 or EPSG:5186.
    pub(super) work_epsg: u32,
    /// Read when a pledge receipt is written. That write runs on Windows and in tests.
    #[cfg(any(windows, test))]
    pub(super) account: crate::heritage_receipt::AccountState,
}

#[cfg(any(windows, test))]
#[derive(Clone, PartialEq, Eq)]
enum Kind {
    Pending,
    /// Set when a hidden-browser kind starts importing. That session is Windows-only; tests set it too.
    #[cfg(any(windows, test))]
    Importing(Group),
    #[cfg(any(windows, test))]
    Publishing(Group),
    #[cfg(any(windows, test))]
    Imported,
    #[cfg(any(windows, test))]
    Absent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Receiving,
    #[cfg(any(windows, test))]
    Importing,
    Settled,
}

pub(crate) struct Download {
    pub(super) context: Context,
    pub(super) config: Option<HeritageConfig>,
    pub(super) job: DownloadJobHandle,
    worker: Worker,
    #[cfg(any(windows, test))]
    kinds: [Kind; DATASETS.len()],
    phase: Phase,
    #[cfg(any(windows, test))]
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
            #[cfg(any(windows, test))]
            root: app.heritage_archive_root(),
            work_epsg: app.work_crs.epsg_code(),
            #[cfg(any(windows, test))]
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
            #[cfg(any(windows, test))]
            kinds: std::array::from_fn(|_| Kind::Pending),
            phase: Phase::Receiving,
            #[cfg(any(windows, test))]
            receipts: 0,
            last_receiving_note: stage,
        }
    }

    fn complete(&self) -> bool {
        // Kinds become Imported or Absent only after the hidden browser, which is Windows-only.
        #[cfg(not(any(windows, test)))]
        {
            false
        }
        #[cfg(any(windows, test))]
        {
            self.phase == Phase::Importing
                && self.receipts == 0
                && self
                    .kinds
                    .iter()
                    .all(|kind| matches!(kind, Kind::Imported | Kind::Absent))
        }
    }

    #[cfg(windows)]
    pub(crate) fn publication_pending(&self) -> bool {
        self.phase != Phase::Settled
            && self
                .kinds
                .iter()
                .any(|kind| matches!(kind, Kind::Publishing(_)))
    }
}

#[path = "heritage_download_drive.rs"]
mod drive;
#[path = "heritage_download_events.rs"]
mod events;
#[path = "heritage_download_results.rs"]
mod results;
