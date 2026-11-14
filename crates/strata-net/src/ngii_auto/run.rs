//! Starting, stopping, failing, the notices and the time limits of a run
//! (`KaTopographicBrowser.cpp:224-236, 362-388, 641-727`).

use super::machine::NgiiAutomation;
use super::url::{portal_map, trusted, UrlParts};
use super::{AutoEvent, PageHost, Scope, Stage};

/// The page-side state of a run, cleared when a run starts or stops (`:649-651, 714-716`).
const RESET_SCRIPT: &str = "window.__kaTopographicSelection=null;window.__kaTopographicOrder=null;window.__kaTopographicSubmission=null;";
/// The page script is asked every 400 ms (`:221`).
const POLL_EVERY_MS: u64 = 400;

impl NgiiAutomation {
    /// `setPhase` (`:362-365`).
    pub(super) fn set_phase(&mut self, phase: &'static str, now: u64) {
        if self.phase != phase {
            self.phase = phase;
            self.phase_since = now;
        }
    }

    /// `automationMessage` (`:641-645`): the words are kept as the notice and told once.
    pub(super) fn message(&mut self, message: &str) {
        self.status = message.to_owned();
        self.attention = message.to_owned();
        if message != self.last_message {
            self.last_message = message.to_owned();
            self.events.push(AutoEvent::NeedsInput(message.to_owned()));
        }
    }

    fn clear_page_state(&mut self, host: &mut dyn PageHost) {
        let Some(portal) = &self.portal else {
            return;
        };
        for page in host.pages() {
            if UrlParts::parse(&page.url).is_some_and(|url| trusted(portal, &url)) {
                // No answer is waited for.
                let _ = host.run_script(page.id, RESET_SCRIPT);
            }
        }
    }

    /// The user's stop (`stopAutomatic`, `:647-659`). Files being received are the browser's to cancel: the
    /// caller ends the hidden session when the stage is `Stopped`.
    pub fn stop(&mut self, host: &mut dyn PageHost) {
        self.generation += 1;
        self.timer_active = false;
        self.script_in_flight = false;
        if let Some(page) = self.transfer_page.take() {
            self.stopped_transfers.push(page);
        }
        self.pending_scope = None;
        self.stage = Stage::Stopped;
        self.requested_numbers.clear();
        self.order_page = None;
        self.order_download_page = None;
        self.receiving.clear();
        self.clear_page_state(host);
        self.status = "자동 진행을 중지했습니다. 이미 신청한 자료는 공식 신청 내역에서 확인할 수 있습니다.".to_owned();
        self.order_sheets_text.clear();
        self.attention.clear();
        self.failure_phase.clear();
        self.transfer_since = None;
    }

    /// `failAutomation` (`:375-387`): the run stops and says why, with the phase it stopped in.
    pub(super) fn fail(&mut self, host: &mut dyn PageHost, message: &str) {
        let phase = self.phase;
        let ordered = std::mem::take(&mut self.order_sheets_text);
        self.stop(host);
        self.failure_phase = phase.to_owned();
        self.order_sheets_text = ordered;
        self.message(message);
    }

    /// Asks for the sheets of a circle (`prepareSheets`, `:660-685`).
    pub fn prepare_sheets(&mut self, host: &mut dyn PageHost, scope: Scope, now: u64) {
        let sorted = |list: &[String]| {
            let mut list = list.to_vec();
            list.sort();
            list.dedup();
            list
        };
        let (numbers, cached) = (sorted(&scope.numbers), sorted(&scope.cached_numbers));
        let sheet = |n: &String| n.len() == 6 && n.bytes().all(|b| b.is_ascii_digit());
        if !(scope.x.is_finite() && scope.y.is_finite() && scope.radius.is_finite() && scope.radius > 0.0) || numbers.len() > 100 || !numbers.iter().chain(&cached).all(sheet) {
            self.message("자동 선택할 1:25,000 도엽 목록과 반경을 확인하지 못했습니다.");
            return;
        }
        let scope = Scope { numbers, cached_numbers: cached, ..scope };
        if self.scope_key.as_ref() == Some(&scope) && self.stage != Stage::Stopped {
            return;
        }
        // The portal query uses a 5 km circle. A few metres of canvas wobble must not start the same order again.
        if self.stage != Stage::Stopped && !self.requested_numbers.is_empty() && self.requested_numbers == scope.numbers {
            return;
        }
        if matches!(self.stage, Stage::Order | Stage::Files | Stage::Download | Stage::Transfer) {
            self.pending_scope = Some(scope);
            self.status = "현재 신청한 도엽을 받은 뒤 새 조사 범위의 자료를 확인합니다.".to_owned();
            return;
        }
        self.start(host, scope, now);
    }

    /// `startAutomation` (`:698-727`).
    pub(super) fn start(&mut self, host: &mut dyn PageHost, scope: Scope, now: u64) {
        self.attention.clear();
        self.failure_phase.clear();
        self.order_sheets_text.clear();
        self.current_file_name.clear();
        (self.received_bytes, self.total_bytes, self.sheet_file_count, self.received_file_count) = (0, None, 0, 0);
        (self.phase, self.phase_since, self.transfer_since) = ("도엽 확인", now, None);
        self.generation += 1;
        self.script_in_flight = false;
        self.wait_ticks = 0;
        self.requested_numbers.clone_from(&scope.numbers);
        self.scope_key = Some(scope.clone());
        self.scope = Some(scope);
        self.pending_scope = None;
        self.records.clear();
        self.current_record.clear();
        self.sheet_index = 0;
        self.manifest.clear();
        self.sheet_directories.clear();
        self.receiving.clear();
        (self.transfer_page, self.order_page, self.order_download_page) = (None, None, None);
        self.stage = Stage::Select;
        self.last_message.clear();
        self.clear_page_state(host);
        if self.config.script.is_empty() {
            self.fail(host, "자동 선택 구성 파일을 찾지 못했습니다. 앱 설치 파일을 확인하세요.");
            return;
        }
        let portal_open = self.portal.as_ref().is_some_and(|portal| host.pages().iter().any(|p| UrlParts::parse(&p.url).is_some_and(|url| portal_map(portal, &url))));
        if !portal_open {
            host.navigate(&self.config.portal_url);
        }
        self.timer_active = true;
        self.last_poll = now;
    }

    /// Call often (every frame is fine): checks the time limits and asks the page script every 400 ms.
    pub fn tick(&mut self, host: &mut dyn PageHost, now: u64) {
        if !self.timer_active {
            return;
        }
        // `:224-236` — a transfer may be silent for 60 s; any other phase may take 90 s, login 300 s.
        let transfer = self.stage == Stage::Transfer;
        let limit = if self.phase == "로그인" { 300_000 } else { 90_000 };
        if transfer && self.transfer_since.is_some_and(|since| now.saturating_sub(since) >= 60_000) {
            self.fail(host, "60초 동안 새 파일 데이터가 수신되지 않았습니다. 연결 상태를 확인한 뒤 다시 시도하세요.");
            return;
        }
        if !transfer && now.saturating_sub(self.phase_since) >= limit {
            self.fail(host, "이 단계의 서버 응답 시간이 초과되었습니다. 계정 설정과 연결 상태를 확인하세요. 신청은 자동으로 반복하지 않습니다.");
            return;
        }
        if now.saturating_sub(self.last_poll) >= POLL_EVERY_MS {
            self.last_poll = now;
            self.poll(host, now);
        }
    }
}
