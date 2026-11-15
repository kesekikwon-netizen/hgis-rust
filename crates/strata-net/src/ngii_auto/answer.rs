//! What an answer of the page script does to the run (the callback of `pollAutomation`,
//! `KaTopographicBrowser.cpp:799-913`).

use std::collections::HashSet;

use serde_json::{Map, Value};
use strata_types::file_name::safe_file_name;

use super::machine::{Asked, ManifestFile, NgiiAutomation};
use super::{AutoEvent, PageHost, PageId, Stage};

fn text(object: &Map<String, Value>, key: &str) -> String {
    object.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn field(object: &Map<String, Value>, key: &str) -> Value {
    object.get(key).cloned().unwrap_or(Value::Null)
}

fn list(object: &Map<String, Value>, key: &str) -> Vec<Value> {
    object.get(key).and_then(Value::as_array).cloned().unwrap_or_default()
}

impl NgiiAutomation {
    /// The value of a script run arrived (JSON text; anything that is not an object counts as no answer).
    pub fn script_answered(&mut self, host: &mut dyn PageHost, token: u64, json: &str, now: u64) {
        let Some(asked) = self.asked.remove(&token) else {
            return;
        };
        let open = |page: PageId| host.pages().iter().any(|p| p.id == page);
        match asked {
            Asked::DownForm { generation, number, page } => {
                // `:559-563` — the official transfer window is named downForm; tab order proves nothing.
                let named = serde_json::from_str::<Value>(json).ok() == Some(Value::Bool(true));
                if named && open(page) && generation == self.generation && self.current_number() == number && matches!(self.stage, Stage::Order | Stage::Files) {
                    self.order_download_page = Some(page);
                }
            }
            Asked::Command { generation, command, page } => {
                if generation != self.generation {
                    return;
                }
                self.script_in_flight = false;
                if !open(page) {
                    return;
                }
                let result = match serde_json::from_str::<Value>(json) {
                    Ok(Value::Object(object)) => object,
                    _ => Map::new(),
                };
                self.answered(host, command, page, &result, now);
            }
        }
    }

    fn answered(&mut self, host: &mut dyn PageHost, command: &str, page: PageId, result: &Map<String, Value>, now: u64) {
        let state = text(result, "state");
        let message = text(result, "message");
        if let Some(note) = result.get("diagnostics").filter(|d| d.as_object().is_some_and(|o| !o.is_empty())) {
            self.last_diagnostic = note.to_string();
        }
        let step = format!("p{} {command}>{state}", page.0);
        if self.trace.last() != Some(&step) {
            if self.trace.len() >= 200 {
                self.trace.remove(0);
            }
            self.trace.push(step);
        }
        if state.is_empty() {
            self.wait_ticks += 1;
            if self.wait_ticks == 75 {
                self.fail(host, "포털 자동 진행 응답을 확인하지 못했습니다. 연결 상태를 확인한 뒤 다시 시도하세요.");
            }
            return;
        }
        if state == "blocked" {
            self.fail(host, &message);
            return;
        }
        if state == "input" || state == "login" {
            self.message(&message);
            // The marked stage is taken back: nothing went out (`:819-820`).
            match command {
                "submit" => self.stage = Stage::Form,
                "download" => self.stage = Stage::Files,
                _ => {}
            }
            return;
        }
        if command == "openForm" && (state == "waitingForm" || state == "openingForm") {
            self.set_phase("신청", now);
        }
        if matches!(state.as_str(), "selected" | "cached" | "form" | "ready" | "orderReady") {
            self.attention.clear();
        }
        match (command, state.as_str()) {
            ("submit", "waitingForm") => self.stage = Stage::Form,
            ("order", "orderReady") => {
                self.stage = Stage::Files;
                self.wait_ticks = 0;
                self.set_phase("파일 준비", now);
            }
            ("order", "delayed") => self.message(&message),
            ("select", "unavailable") => {
                self.stage = Stage::Stopped;
                self.timer_active = false;
                self.events.push(AutoEvent::SelectionUpdated { items: Value::Array(Vec::new()), cached: Value::Null, unavailable: field(result, "unavailable") });
                self.message(self.config.product.words()[1]);
            }
            ("select", "selected" | "cached") => self.selected(result),
            ("openForm", "form") => {
                self.stage = Stage::Form;
                self.wait_ticks = 0;
            }
            ("form", "ready") => self.stage = Stage::Submit,
            ("files", "ready") => self.files_ready(host, page, result),
            _ => {}
        }
    }

    /// `:845-879` — the portal's own list of sheets: nothing to fetch, or the first sheet to apply for.
    fn selected(&mut self, result: &Map<String, Value>) {
        let all = list(result, "items");
        if let Some(scope) = &mut self.scope {
            scope.numbers = all.iter().filter_map(|record| record.get("num").and_then(Value::as_str).map(str::to_owned)).collect();
        }
        self.records = list(result, "downloadItems");
        let update = AutoEvent::SelectionUpdated { items: Value::Array(all), cached: field(result, "cached"), unavailable: field(result, "unavailable") };
        if self.records.is_empty() {
            self.stage = Stage::Complete;
            self.timer_active = false;
            self.status = self.config.product.words()[2].to_owned();
            self.events.push(update);
            return;
        }
        let Some(record) = self.records.get(self.sheet_index).and_then(Value::as_object).cloned() else {
            return;
        };
        // Show exactly which sheets the automatic order is about to apply for (`:862-872`).
        let mut order: Vec<String> = self.records.iter().filter_map(Value::as_object).map(|r| format!("{} {}", text(r, "num"), text(r, "name")).trim().to_owned()).collect();
        let total = order.len();
        if total > 8 {
            order.truncate(8);
            order.push(format!("외 {}장", total - 8));
        }
        self.order_sheets_text = format!("공식 신청할 도엽 {total}장 · {} — 중지하려면 「취소」를 누르세요.", order.join(", "));
        self.current_record = record;
        self.stage = Stage::OpenForm;
        self.wait_ticks = 0;
        self.events.push(update);
    }

    /// `:885-911` — the transfer page lists the files of the order; every one must belong to exactly one
    /// sheet that was applied for, under a safe name, before anything is fetched.
    fn files_ready(&mut self, host: &mut dyn PageHost, page: PageId, result: &Map<String, Value>) {
        self.expected_sheet_files = result.get("files").and_then(Value::as_f64).map_or(0, |n| n as i64);
        self.manifest.clear();
        self.sheet_directories.clear();
        let listed = list(result, "manifest");
        if !listed.is_empty() {
            let (mut names, mut ids, mut numbers) = (HashSet::new(), HashSet::new(), HashSet::new());
            let mut valid = listed.len() as i64 == self.expected_sheet_files;
            for item in &listed {
                let empty = Map::new();
                let file = item.as_object().unwrap_or(&empty);
                let (name, id, number) = (text(file, "name"), text(file, "id"), text(file, "number"));
                let bytes = file.get("bytes").and_then(Value::as_f64).unwrap_or(0.0);
                let found = self.records.iter().filter(|r| r.get("num").and_then(Value::as_str) == Some(number.as_str())).count();
                valid = valid && !name.is_empty() && safe_file_name(&name) == name && !id.is_empty() && !names.contains(&name) && !ids.contains(&id) && found == 1 && bytes > 0.0;
                names.insert(name.clone());
                ids.insert(id);
                numbers.insert(number.clone());
                self.manifest.push(ManifestFile { name, number, bytes });
            }
            if !valid || numbers.len() != self.records.len() {
                self.fail(host, "신청한 도엽과 파일 연결을 확인하지 못해 전송을 중단했습니다.");
                return;
            }
        }
        self.sheet_file_count = self.expected_sheet_files;
        self.received_file_count = 0;
        self.current_file_name.clear();
        (self.received_bytes, self.total_bytes) = (0, None);
        self.sheet_directory = None;
        self.completed_paths.clear();
        self.transfer_page = Some(page);
        self.stage = Stage::Download;
    }
}
