//! Event facts and map acceptance (`target/cpp-ref/src/app/MainWindow.cpp:3135-3148`).

#[cfg(any(windows, test))]
use super::{Group, Kind, Phase, Task};
use crate::app_state::StrataAppState;
#[cfg(any(windows, test))]
use crate::download_jobs::StopCode;
#[cfg(any(windows, test))]
use strata_net::heritage_auto::HeritageEvent;
#[cfg(any(windows, test))]
use strata_refmap::heritage::DATASETS;

impl StrataAppState {
    #[cfg(any(windows, test))]
    pub(crate) fn take_heritage_event(&mut self, event: HeritageEvent) {
        let Some(download) = self.heritage.download.as_mut() else {
            return;
        };
        if download.phase == Phase::Settled || download.job.is_cancelled() {
            return;
        }
        match event {
            HeritageEvent::Note(words) => {
                download.last_receiving_note.clone_from(&words);
                download.job.report_running(None, Some(words.clone()));
                self.show_status_message(words);
            }
            HeritageEvent::PledgeAgreed { terms } => {
                download.receipts += 1;
                if !download.worker.queue(Task::Pledge(terms)) {
                    self.stop_heritage_download(
                        StopCode::App1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                }
            }
            HeritageEvent::DatasetReady { kind, files } => {
                let Some(index) = DATASETS.iter().position(|dataset| dataset.name == kind) else {
                    return self.stop_heritage_download(
                        StopCode::Data1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                };
                if download.kinds[index] != Kind::Pending {
                    return self.stop_heritage_download(
                        StopCode::Data1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                }
                let group = Group {
                    job: download.job.id(),
                    dataset: &DATASETS[index],
                    files,
                };
                download.kinds[index] = Kind::Importing(group.clone());
                if !download.worker.queue(Task::Group(group)) {
                    self.stop_heritage_download(
                        StopCode::App1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                }
            }
            HeritageEvent::KindAbsent { kind } => {
                let Some(index) = DATASETS.iter().position(|dataset| dataset.name == kind) else {
                    return self.stop_heritage_download(
                        StopCode::Data1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                };
                if download.kinds[index] != Kind::Pending {
                    return self.stop_heritage_download(
                        StopCode::Data1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                }
                download.kinds[index] = Kind::Absent;
            }
            HeritageEvent::Done => {
                if download.kinds.contains(&Kind::Pending) {
                    return self.stop_heritage_download(
                        StopCode::Data1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                }
                download.phase = Phase::Importing;
            }
            HeritageEvent::Cancelled => self.cancel_heritage_download(),
            HeritageEvent::Stopped(words) => {
                let code = if words.contains("로그인")
                    || words.contains("아이디")
                    || words.contains("비밀번호")
                {
                    StopCode::Acc2
                } else {
                    StopCode::Site1
                };
                self.stop_heritage_download(code, words);
            }
        }
    }
}
