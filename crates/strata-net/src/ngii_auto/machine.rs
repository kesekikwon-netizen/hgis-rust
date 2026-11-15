//! The state of a run and what the caller may read of it (the members of `KaTopographicBrowser`).

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Map, Value};

use super::url::UrlParts;
use super::{AutoConfig, AutoEvent, PageId, Scope, Stage};

/// What a script run was for, kept until its answer comes.
#[derive(Debug, Clone)]
pub(super) enum Asked {
    /// The page script with a command (`:798`).
    Command { generation: u64, command: &'static str, page: PageId },
    /// Whether a transfer popup is the official one, by its window name (`:559`).
    DownForm { generation: u64, number: String, page: PageId },
}

/// One file of an order as the transfer page lists it (`:885-900`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ManifestFile {
    pub(super) name: String,
    pub(super) number: String,
    pub(super) bytes: f64,
}

/// A file being received. Several may arrive at once (the C++ keeps each in a request of its own).
#[derive(Debug, Clone)]
pub(super) struct Receiving {
    /// Where the complete file will be.
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) total: Option<u64>,
    pub(super) generation: u64,
    pub(super) directory: PathBuf,
    pub(super) record: Map<String, Value>,
}

/// The order of topographic sheets, stage by stage.
#[derive(Debug)]
pub struct NgiiAutomation {
    pub(super) config: AutoConfig,
    pub(super) portal: Option<UrlParts>,
    pub(super) stage: Stage,
    pub(super) generation: u64,
    pub(super) timer_active: bool,
    pub(super) script_in_flight: bool,
    pub(super) asked: HashMap<u64, Asked>,
    pub(super) last_poll: u64,
    pub(super) wait_ticks: u32,
    pub(super) scope: Option<Scope>,
    pub(super) scope_key: Option<Scope>,
    pub(super) pending_scope: Option<Scope>,
    pub(super) requested_numbers: Vec<String>,
    pub(super) records: Vec<Value>,
    pub(super) current_record: Map<String, Value>,
    pub(super) sheet_index: usize,
    pub(super) manifest: Vec<ManifestFile>,
    pub(super) expected_sheet_files: i64,
    pub(super) sheet_file_count: i64,
    pub(super) received_file_count: usize,
    pub(super) current_file_name: String,
    pub(super) received_bytes: u64,
    pub(super) total_bytes: Option<u64>,
    pub(super) sheet_directory: Option<PathBuf>,
    pub(super) sheet_directories: HashMap<String, PathBuf>,
    pub(super) completed_paths: Vec<PathBuf>,
    pub(super) receiving: Vec<Receiving>,
    pub(super) phase: &'static str,
    pub(super) phase_since: u64,
    pub(super) transfer_since: Option<u64>,
    pub(super) order_page: Option<PageId>,
    pub(super) order_download_page: Option<PageId>,
    pub(super) transfer_page: Option<PageId>,
    /// Popups a submitting page opened, with the run and sheet they were opened for (`:536-550`).
    pub(super) order_popups: HashMap<PageId, (u64, String)>,
    /// Pages whose run was stopped while they were handing over files (`:648`).
    pub(super) stopped_transfers: Vec<PageId>,
    /// Map pages that began to load anew while an order was being prepared and have not finished.
    pub(super) reloading: Vec<PageId>,
    pub(super) login_attempt: u64,
    pub(super) attention: String,
    pub(super) last_message: String,
    pub(super) failure_phase: String,
    pub(super) order_sheets_text: String,
    pub(super) status: String,
    pub(super) events: Vec<AutoEvent>,
    pub(super) last_diagnostic: String,
    /// Page, command and answered state, each change once (for the log of a run; no page content).
    pub(super) trace: Vec<String>,
}

impl NgiiAutomation {
    #[must_use]
    pub fn new(config: AutoConfig) -> Self {
        let portal = UrlParts::parse(&config.portal_url);
        Self {
            config,
            portal,
            stage: Stage::Idle,
            generation: 0,
            timer_active: false,
            script_in_flight: false,
            asked: HashMap::new(),
            last_poll: 0,
            wait_ticks: 0,
            scope: None,
            scope_key: None,
            pending_scope: None,
            requested_numbers: Vec::new(),
            records: Vec::new(),
            current_record: Map::new(),
            sheet_index: 0,
            manifest: Vec::new(),
            expected_sheet_files: 0,
            sheet_file_count: 0,
            received_file_count: 0,
            current_file_name: String::new(),
            received_bytes: 0,
            total_bytes: None,
            sheet_directory: None,
            sheet_directories: HashMap::new(),
            completed_paths: Vec::new(),
            receiving: Vec::new(),
            phase: "도엽 확인",
            phase_since: 0,
            transfer_since: None,
            order_page: None,
            order_download_page: None,
            transfer_page: None,
            order_popups: HashMap::new(),
            stopped_transfers: Vec::new(),
            reloading: Vec::new(),
            // The C++ counts the account being set as the first attempt (`:269`).
            login_attempt: 1,
            attention: String::new(),
            last_message: String::new(),
            failure_phase: String::new(),
            order_sheets_text: String::new(),
            status: String::new(),
            events: Vec::new(),
            last_diagnostic: String::new(),
            trace: Vec::new(),
        }
    }

    #[must_use]
    pub fn stage(&self) -> Stage {
        self.stage
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The phase shown to the user: 도엽 확인, 로그인, 신청, 파일 준비 or 다운로드.
    #[must_use]
    pub fn phase(&self) -> &str {
        self.phase
    }

    /// The phase in which the run failed; empty while it has not.
    #[must_use]
    pub fn failure_phase(&self) -> &str {
        &self.failure_phase
    }

    /// The notice that waits for the user; empty when there is none.
    #[must_use]
    pub fn attention(&self) -> &str {
        &self.attention
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The sheets the run is applying for, in the words of the C++ window (`:862-872`).
    #[must_use]
    pub fn order_sheets_text(&self) -> &str {
        &self.order_sheets_text
    }

    /// The file being received: its name, the bytes that are there, and its size when the transfer page
    /// listed one.
    #[must_use]
    pub fn current_file(&self) -> (&str, u64, Option<u64>) {
        (&self.current_file_name, self.received_bytes, self.total_bytes)
    }

    #[must_use]
    pub fn received_bytes(&self) -> u64 {
        self.received_bytes
    }

    /// Files of the current order that are complete, and how many the order has.
    #[must_use]
    pub fn files_done(&self) -> (usize, i64) {
        (self.received_file_count, self.sheet_file_count)
    }

    /// The last note of the page script about where it stopped (`:804-809`); empty when there is none.
    #[must_use]
    pub fn last_diagnostic(&self) -> &str {
        &self.last_diagnostic
    }

    /// The portal's records of the sheets it listed for this run (number, name, datum, rectangle).
    #[must_use]
    pub fn portal_records(&self) -> &[Value] {
        &self.records
    }

    /// The course of the run: `p<page> <command>><state>`, consecutive repeats left out, the last 200.
    #[must_use]
    pub fn answer_trace(&self) -> &[String] {
        &self.trace
    }

    /// What happened since the last call.
    pub fn take_events(&mut self) -> Vec<AutoEvent> {
        std::mem::take(&mut self.events)
    }

    pub(super) fn current_number(&self) -> String {
        self.current_record.get("num").and_then(Value::as_str).unwrap_or_default().to_owned()
    }
}
