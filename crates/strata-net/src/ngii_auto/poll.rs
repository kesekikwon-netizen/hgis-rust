//! One poll: which page is asked, with what, and what is marked before the script runs
//! (`pollAutomation` up to the script call, `KaTopographicBrowser.cpp:729-798`).

use serde_json::{json, Map, Value};

use super::machine::{Asked, NgiiAutomation};
use super::url::{login_rank, portal_map, transfer_page, UrlParts};
use super::{PageHost, PageId, Stage};

impl NgiiAutomation {
    /// The page this poll is for and whether it is a login page (`:735-755`). Among several candidates the
    /// last one wins, as in the C++ loops.
    fn page_to_ask(&self, host: &dyn PageHost) -> Option<(PageId, bool)> {
        let portal = self.portal.as_ref()?;
        let pages: Vec<(PageId, UrlParts)> = host.pages().into_iter().filter_map(|p| Some((p.id, UrlParts::parse(&p.url)?))).collect();
        if matches!(self.stage, Stage::Files | Stage::Download) {
            let own = self.order_download_page?;
            return pages.iter().rev().find(|(id, url)| *id == own && transfer_page(portal, url, self.config.test_mode)).map(|(id, _)| (*id, false));
        }
        let login = pages.iter().enumerate().filter_map(|(at, (id, url))| Some(((login_rank(portal, url)?, at), *id))).max_by_key(|(rank, _)| *rank);
        if let Some((_, id)) = login {
            return Some((id, true));
        }
        pages.iter().rev().find(|(_, url)| portal_map(portal, url)).map(|(id, _)| (*id, false))
    }

    /// What the page script is called with (`:762-784`).
    fn input(&self, command: &str, login: bool) -> Value {
        let mut input = Map::new();
        if let Some(scope) = &self.scope {
            input.insert("x".into(), json!(scope.x));
            input.insert("y".into(), json!(scope.y));
            input.insert("radius".into(), json!(scope.radius));
            input.insert("numbers".into(), json!(scope.numbers));
            input.insert("cachedNumbers".into(), json!(scope.cached_numbers));
        }
        input.insert("command".into(), json!(command));
        input.insert("diagnostics".into(), json!(self.config.diagnostics));
        input.insert("generation".into(), json!(self.generation.to_string()));
        if let Some(number) = self.current_record.get("num") {
            input.insert("number".into(), number.clone());
        }
        input.insert("record".into(), Value::Object(self.current_record.clone()));
        input.insert("records".into(), Value::Array(self.records.clone()));
        input.insert("fileIndex".into(), json!(self.received_file_count));
        let account = &self.config.account;
        if command == "form" || command == "submit" {
            input.insert("birthDate".into(), json!(account.birth_date.trim()));
            input.insert("purpose".into(), json!(account.purpose.trim()));
        }
        if login {
            input.insert("loginId".into(), json!(account.login_id));
            input.insert("loginPassword".into(), json!(account.login_password));
            input.insert("loginAttempt".into(), json!(self.login_attempt.to_string()));
        }
        Value::Object(input)
    }

    /// `pollAutomation` (`:729-798`).
    pub(super) fn poll(&mut self, host: &mut dyn PageHost, now: u64) {
        if self.script_in_flight || matches!(self.stage, Stage::Idle | Stage::Stopped | Stage::Complete | Stage::Transfer) {
            return;
        }
        let download = matches!(self.stage, Stage::Files | Stage::Download);
        let asked = self.page_to_ask(host);
        // A map page that is loading anew is left alone; this is waiting, not a missing page.
        if asked.is_some_and(|(page, login)| !login && self.reloading.contains(&page)) {
            return;
        }
        let Some((page, login)) = asked else {
            self.wait_ticks += 1;
            if self.wait_ticks == 75 {
                let words = if download {
                    "파일 전송 화면이 열리지 않았습니다. 신청 내역과 연결 상태를 확인하세요."
                } else {
                    "공식 사이트의 응답을 확인하지 못했습니다. 계정 설정과 연결 상태를 확인하세요."
                };
                self.fail(host, words);
            }
            return;
        };
        let command = if login { "login" } else { self.stage.command() };
        match command {
            "select" => self.set_phase("도엽 확인", now),
            "login" => self.set_phase("로그인", now),
            "openForm" if self.phase != "신청" => self.set_phase("로그인", now),
            "form" | "submit" => self.set_phase("신청", now),
            "order" | "files" => self.set_phase("파일 준비", now),
            "download" => self.set_phase("다운로드", now),
            _ => {}
        }
        let input = self.input(command, login);
        // Mark irreversible stages first; a lost answer must not apply for the sheets again (`:785-793`).
        if command == "submit" {
            self.stage = Stage::Order;
            self.order_page = Some(page);
            self.order_download_page = None;
        }
        if command == "download" {
            self.stage = Stage::Transfer;
            self.transfer_page = Some(page);
            self.wait_ticks = 0;
            self.transfer_since = Some(now);
            self.stopped_transfers.retain(|p| *p != page);
        }
        let script = format!("{}({});", self.config.script, input);
        if let Some(token) = host.run_script(page, &script) {
            self.script_in_flight = true;
            self.asked.insert(token, Asked::Command { generation: self.generation, command, page });
        }
    }
}
