//! Pressing 전체다운로드, waiting for the ZIP, and moving to the next kind.
//!
//! The site builds the ZIP on demand and answers with a page until it is ready (40–120 s in
//! practice, `KaHeritageBrowser.cpp:1470-1524`). A kind with no rows is skipped; a kind that
//! fails three times stops the run with words that say so.
//!
//! The C++ retries a broken transfer without limit, waiting 5, 10, 20 then 30 s between tries
//! (`KaHeritageBrowser.cpp:1359-1372`). This port stops after three, because an unbounded retry
//! loop against an account can lock it; the words say which try was the last.

use super::machine::{HeritageAutomation, HeritageEvent, HeritageStage};

/// The waits between retries, as the C++ has them (`:1363`).
const RETRY_DELAYS_MS: [u64; 4] = [5_000, 10_000, 20_000, 30_000];
/// How many transfers of one kind may fail before the run stops.
pub const MAX_ATTEMPTS: u8 = 3;

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
        self.offered.push(path);
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
                self.wait_since_ms = self.asked_at_ms;
                self.response_only_retry = false;
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
        let kind = self.kind_name();
        self.retry_count = 0;
        self.files_this_kind += 1;
        Some(HeritageEvent::ZipReady { kind, path })
    }

    /// A transfer failed, or what came back was not a file.
    ///
    /// Waits the C++ delays, then presses download again. Stops on the [`MAX_ATTEMPTS`]th
    /// failure, which the C++ does not do (`:1359-1372`).
    pub fn transfer_failed(&mut self, reason: &str, now_ms: u64) -> Option<HeritageEvent> {
        if self.is_over() {
            return None;
        }
        self.retry_count = self.retry_count.saturating_add(1);
        if self.retry_count >= MAX_ATTEMPTS {
            let kind = self.kind_name();
            return self.stop(format!(
                "{kind}: {reason} — {MAX_ATTEMPTS}번 다시 받았지만 되지 않았습니다. \
                 잠시 뒤 다시 눌러 주세요."
            ));
        }
        let delay =
            RETRY_DELAYS_MS[usize::from(self.retry_count - 1).min(RETRY_DELAYS_MS.len() - 1)];
        self.retry_at_ms = now_ms.saturating_add(delay);
        self.go(HeritageStage::Waiting);
        Some(HeritageEvent::Note(format!(
            "{reason} — {}초 뒤 다시 받습니다({}/{MAX_ATTEMPTS}).",
            delay / 1000,
            self.retry_count + 1
        )))
    }

    /// Whether the wait for the current kind is over because the server is still building it.
    ///
    /// The C++ gives the server 40–120 s (`:1470-1524`); past that it asks the page again.
    #[must_use]
    pub fn wait_expired(&self, now_ms: u64) -> bool {
        self.stage == HeritageStage::Waiting
            && now_ms.saturating_sub(self.wait_since_ms) >= WAIT_LIMIT_MS
    }

    /// The current kind is done. Moves to the next, or finishes the run.
    pub fn kind_done(&mut self) -> Option<HeritageEvent> {
        if self.is_over() || self.files_this_kind == 0 {
            return None;
        }
        self.kind_index += 1;
        self.retry_count = 0;
        self.files_this_kind = 0;
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

    /// The run's own timers, read once a frame: a retry that has come due, and a wait that has
    /// run past [`WAIT_LIMIT_MS`].
    ///
    /// The C++ does both from its poll (`KaHeritageBrowser.cpp:1470-1524`).
    pub fn timers(&mut self, now_ms: u64) -> Vec<HeritageEvent> {
        if self.stage != HeritageStage::Waiting {
            return Vec::new();
        }
        if self.retry_due(now_ms) {
            self.retry_at_ms = 0;
            // Pressing download again is the next ask; the wait stage hands it to the caller.
            self.go(HeritageStage::Kind);
            return vec![HeritageEvent::Note(format!(
                "{} — 다시 받습니다.",
                self.kind_name()
            ))];
        }
        if self.wait_expired(now_ms) && self.files_this_kind == 0 {
            // The C++ gives up on a kind whose file never came (`:1470-1524`).
            let kind = self.kind_name();
            let words =
                format!("{kind}: 서버가 파일을 만들지 못했습니다. 잠시 뒤 다시 눌러 주세요.");
            self.stage = HeritageStage::Stopped(words.clone());
            return vec![HeritageEvent::Stopped(words)];
        }
        Vec::new()
    }

    /// The retry wait is over: press download again.
    #[must_use]
    pub fn retry_due(&self, now_ms: u64) -> bool {
        self.stage == HeritageStage::Waiting && now_ms >= self.retry_at_ms && self.retry_at_ms > 0
    }
}

/// How long the site is given to build a ZIP before the page is asked again.
///
/// The C++ waits 40–120 s in practice (`KaHeritageBrowser.cpp:1470-1524`); 150 s leaves room
/// for a slow day without hanging the run forever.
pub const WAIT_LIMIT_MS: u64 = 150_000;
