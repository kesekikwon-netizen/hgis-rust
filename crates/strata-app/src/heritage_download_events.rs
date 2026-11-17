//! Event facts and map acceptance (`target/cpp-ref/src/app/MainWindow.cpp:3135-3148`).

use super::{Kind, Phase, Reply, Task};
use crate::app_state::StrataAppState;
use crate::download_jobs::StopCode;
use strata_net::heritage_auto::HeritageEvent;
use strata_refmap::heritage::DATASETS;

impl StrataAppState {
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
            HeritageEvent::ZipReady { kind, path } => {
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
                download.kinds[index] = Kind::Importing;
                if !download.worker.queue(Task::Zip(&DATASETS[index], path)) {
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

    pub(super) fn take_heritage_import(&mut self, reply: Reply) {
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
            Reply::Kind(dataset, Ok(load)) => {
                if let Some(index) = DATASETS.iter().position(|known| known.key == dataset.key) {
                    download.kinds[index] = Kind::Imported;
                }
                let job = download.job.clone();
                self.heritage.asked = None;
                self.take_heritage_step(&job, super::super::Step::Kind(dataset, Ok(load)));
            }
            Reply::Kind(_, Err((code, reason))) => self.stop_heritage_download(code, reason),
        }
    }
}
