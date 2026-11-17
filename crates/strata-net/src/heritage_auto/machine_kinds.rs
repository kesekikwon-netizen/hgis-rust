//! Download grouping (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1150-1217,1271-1289`).

use super::machine::{HeritageAutomation, HeritageEvent, HeritageStage};

impl HeritageAutomation {
    /// Whether a file the site offers should be kept right now.
    ///
    /// The C++ refuses a file at any stage but the download itself
    /// (`KaHeritageBrowser.cpp:1295-1302`): a page that fires a download while we are still
    /// logging in is not the file the run asked for.
    #[must_use]
    pub fn accepts_files(&self) -> bool {
        matches!(
            self.stage,
            HeritageStage::DownloadAll | HeritageStage::Waiting
        )
    }

    /// A file was offered and accepted; its path is known before it finishes.
    pub fn file_offered(&mut self, path: std::path::PathBuf) {
        if self.accepts_files() {
            self.group.offer(path);
        }
    }

    /// The kind's 한국어 이름, for the words the user sees.
    pub(super) fn kind_name(&self) -> &'static str {
        self.current_kind()
    }

    /// Presses 전체다운로드. The answer says whether the button was pressed, not whether a file
    /// came: `all` means pressed, and the wait stage takes over (`:415-440`).
    pub(super) fn on_download_all(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            "all" => {
                self.go(HeritageStage::Waiting);
                Some(HeritageEvent::Note(format!(
                    "{} — 서버가 파일을 만드는 중입니다.",
                    self.kind_name()
                )))
            }
            "no-region" => {
                let kind = self.kind_name();
                self.stop(format!(
                    "{kind}: 지역이 골라지지 않았습니다. 다시 눌러 주세요."
                ))
            }
            "ambiguous" => {
                let kind = self.kind_name();
                self.stop(format!(
                    "{kind}: 「전체다운로드」 단추가 여러 개라 누르지 않았습니다."
                ))
            }
            "not-found" => {
                let kind = self.kind_name();
                self.stop(format!("{kind}: 「전체다운로드」 단추를 찾지 못했습니다."))
            }
            other => {
                let kind = self.kind_name();
                self.stop(format!(
                    "{kind}: 다운로드를 누르는 중 문제가 생겼습니다({other})."
                ))
            }
        }
    }

    /// The ZIP for this kind is complete on disk.
    pub fn zip_arrived(&mut self, path: std::path::PathBuf) -> Option<HeritageEvent> {
        if !self.accepts_files() {
            return None;
        }
        self.group.complete(&path, self.observed_at_ms);
        None
    }

    /// A transfer alone cannot advance the kind (`KaHeritageBrowser.cpp:1206-1217`).
    /// Removed with the native runner caller once its separate bridge dependency is consumed.
    pub fn kind_done(&mut self) -> Option<HeritageEvent> {
        None
    }

    /// Both received files and proven absence end a kind (`KaHeritageBrowser.cpp:1108-1112,1271-1289`).
    pub(super) fn advance_kind(&mut self) -> Option<HeritageEvent> {
        self.kind_index += 1;
        self.retry_count = 0;
        self.group = super::transfer_group::TransferGroup::default();
        self.retry_at_ms = 0;
        self.pledge_kept = self.pledge_kept || self.pledge.is_some();
        if self.kind_index >= super::scripts::KINDS.len() {
            self.go(HeritageStage::Done);
            return Some(HeritageEvent::Done);
        }
        self.go(HeritageStage::Kind);
        Some(HeritageEvent::Note(format!(
            "{} — 다음 종류로 넘어갑니다.",
            self.kind_name()
        )))
    }

    /// Observes quiet groups and the navigation bound (KaHeritageBrowser.cpp:871-885,1150-1217).
    pub fn timers(&mut self, now_ms: u64) -> Vec<HeritageEvent> {
        self.observe_time(now_ms);
        let mut events = std::mem::take(&mut self.queued_events);
        if matches!(
            self.stage,
            HeritageStage::DownloadAll | HeritageStage::Waiting
        ) {
            if self.group.ready(now_ms) {
                let kind = self.kind_name();
                let files = self.group.files();
                self.go(HeritageStage::AwaitingImport);
                events.push(HeritageEvent::DatasetReady { kind, files });
            }
            return events;
        }
        if !self.is_over()
            && !matches!(
                self.stage,
                HeritageStage::Waiting
                    | HeritageStage::Idle
                    | HeritageStage::AwaitingImport
                    | HeritageStage::RetryWait
            )
            && now_ms.saturating_sub(self.phase_since_ms) > super::readiness::NAVIGATION_LIMIT_MS
        {
            events.extend(self.stop(format!(
                "「{}」 단계에서 화면이 예상과 달라 멈췄습니다. 아래 내용을 보고 알려 주세요.",
                self.progress_words()
            )));
            return events;
        }
        events
    }
}
