//! Hidden run lifetime (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:479-488`, HER-API1).

use super::{Download, Phase};
use crate::app_state::StrataAppState;
use crate::download_jobs::StopCode;
use std::sync::mpsc::TryRecvError;

impl StrataAppState {
    pub(crate) fn heritage_browser_ended(&mut self) {
        if self
            .heritage
            .download
            .as_ref()
            .is_some_and(|download| download.phase == Phase::Receiving)
        {
            self.stop_heritage_download(
                StopCode::App1,
                crate::download_jobs::APP1_STOP_REASON.into(),
            );
        }
    }
    pub(crate) fn cancel_heritage_download(&mut self) {
        if let Some(download) = self.heritage.download.as_mut() {
            if download.phase != Phase::Settled {
                self.download_jobs.request_cancel(download.job.id());
                download.job.report_cancelled();
                download.phase = Phase::Settled;
                // Clear only this run's receiving words (target/cpp-ref/src/app/MainWindow.cpp:3157).
                if self.status_bar_hint == download.last_receiving_note {
                    self.status_bar_hint.clear();
                }
                if self.home_map.draw_guide_message.as_deref()
                    == Some(download.last_receiving_note.as_str())
                {
                    self.home_map.draw_guide_message = Some("취소했습니다.".into());
                }
            }
            download.config = None;
            download.worker.send = None;
        }
        #[cfg(windows)]
        if let Some(run) = self.heritage.browser_run.as_mut() {
            run.cancel();
        }
        if !self.heritage.browser_busy() {
            self.heritage.download = None;
        }
    }

    pub(crate) fn stop_heritage_download(&mut self, code: StopCode, reason: String) {
        if let Some(download) = self.heritage.download.as_mut() {
            download.job.report_stopped(code, reason.clone());
            self.download_jobs.request_cancel(download.job.id());
            download.phase = Phase::Settled;
            download.worker.send = None;
        }
        self.show_status_message(reason);
        self.heritage_run_requested = false;
        #[cfg(windows)]
        if let Some(run) = self.heritage.browser_run.as_mut() {
            run.cancel();
        }
    }

    pub(crate) fn drive_heritage_download(&mut self, ctx: &eframe::egui::Context) {
        if self.heritage.download.as_ref().is_some_and(|download| {
            self.survey_path.as_ref() != Some(&download.context.survey)
                || self.work_crs.epsg_code() != download.context.work_epsg
                || download.job.is_cancelled()
        }) {
            self.heritage_run_requested = false;
            self.cancel_heritage_download();
        }
        while let Some(download) = self.heritage.download.as_ref() {
            if download.phase == Phase::Settled {
                break;
            }
            match download.worker.replies.try_recv() {
                Ok(reply) => self.take_heritage_import(reply),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.stop_heritage_download(
                        StopCode::App1,
                        crate::download_jobs::APP1_STOP_REASON.into(),
                    );
                    break;
                }
            }
        }
        if self
            .heritage
            .download
            .as_ref()
            .is_some_and(Download::complete)
        {
            if let Some(download) = self.heritage.download.as_mut() {
                download.job.report_done(super::super::DONE);
                download.phase = Phase::Settled;
                download.worker.send = None;
            }
            self.show_status_message(super::super::DONE);
        }
        if !self.heritage.browser_busy()
            && self
                .heritage
                .download
                .as_ref()
                .is_some_and(|d| d.phase == Phase::Settled)
        {
            self.heritage.download = None;
        }
        // Hidden browser timers and asynchronous cleanup need frames even when the gauge is
        // settled (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:479-488`, HER-API1).
        if self.heritage.download.is_some() || self.heritage.browser_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }
}
