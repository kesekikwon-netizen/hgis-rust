//! Accepted group publication (target/cpp-ref/src/app/KaHeritageBrowser.cpp:1206-1217).

use super::{Kind, Phase, Reply, Task};
use crate::app_state::StrataAppState;
use crate::download_jobs::StopCode;
use strata_net::heritage_auto::DatasetOutcome;

impl StrataAppState {
    pub(super) fn take_heritage_import(&mut self, reply: Reply) {
        #[cfg(windows)]
        let mut browser = self.heritage.browser_run.take();
        self.take_heritage_import_with(reply, |_files, _outcome| {
            #[cfg(windows)]
            {
                browser
                    .as_mut()
                    .is_some_and(|run| run.dataset_loaded(_files, _outcome))
            }
            #[cfg(not(windows))]
            {
                false
            }
        });
        #[cfg(windows)]
        {
            if self
                .heritage
                .download
                .as_ref()
                .is_none_or(|d| d.phase == Phase::Settled)
            {
                if let Some(run) = browser.as_mut() {
                    run.cancel();
                }
            }
            self.heritage.browser_run = browser;
        }
    }

    pub(super) fn take_heritage_import_with(
        &mut self,
        reply: Reply,
        mut acknowledge: impl FnMut(&[std::path::PathBuf], DatasetOutcome) -> bool,
    ) {
        let Some(download) = self.heritage.download.as_mut() else {
            return;
        };
        if download.phase == Phase::Settled || download.job.is_cancelled() {
            return;
        }
        match reply {
            Reply::Receipt(Ok(path)) => {
                download.receipts = download.receipts.saturating_sub(1);
                self.show_status_message(format!("서약서 영수증을 남겼습니다: {}", path.display()));
            }
            Reply::Receipt(Err(reason)) => self.stop_heritage_download(
                StopCode::Dir2,
                format!("서약서 영수증을 남기지 못했습니다: {reason}"),
            ),
            Reply::Kind(group, result) => {
                if group.job != download.job.id() {
                    return;
                }
                let Some(index) = download
                    .kinds
                    .iter()
                    .position(|kind| matches!(kind, Kind::Importing(active) if active == &group))
                else {
                    return;
                };
                match result {
                    Ok(load) => {
                        if !acknowledge(&group.files, DatasetOutcome::Accepted) {
                            return;
                        }
                        download.kinds[index] = Kind::Publishing(group.clone());
                        if !download.worker.queue(Task::Publish(group, load)) {
                            self.stop_heritage_download(
                                StopCode::App1,
                                crate::download_jobs::APP1_STOP_REASON.into(),
                            );
                        }
                    }
                    Err(failure) => {
                        let outcome = if failure.download_again {
                            DatasetOutcome::RetryableFailure {
                                reason: failure.reason.clone(),
                            }
                        } else {
                            DatasetOutcome::FatalFailure {
                                reason: failure.reason.clone(),
                            }
                        };
                        if !acknowledge(&group.files, outcome) {
                            return;
                        }
                        if failure.download_again {
                            download.kinds[index] = Kind::Pending;
                        } else {
                            self.stop_heritage_download(failure.code, failure.reason);
                        }
                    }
                }
            }
            Reply::Published(group, result) => {
                if group.job != download.job.id() {
                    return;
                }
                let Some(index) = download
                    .kinds
                    .iter()
                    .position(|kind| matches!(kind, Kind::Publishing(active) if active == &group))
                else {
                    return;
                };
                match result {
                    Ok(load) => {
                        download.kinds[index] = Kind::Imported;
                        let job = download.job.clone();
                        self.heritage.asked = None;
                        self.take_heritage_step(
                            &job,
                            super::super::Step::Kind(group.dataset, Ok(load)),
                        );
                    }
                    Err(failure) => self.stop_heritage_download(failure.code, failure.reason),
                }
            }
        }
    }
}
