//! The files of an order arriving (`requestDownload`, `KaTopographicBrowser.cpp:915-1105`).
//!
//! The C++ window also saves files the user fetches by hand in its browser. A hidden run has no hand: a
//! file is taken only when the run asked the transfer page for it.

use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use strata_types::file_name::safe_file_name;

use super::machine::{NgiiAutomation, Receiving};
use super::{AutoEvent, DownloadTarget, PageHost, PageId, Stage};

/// No single file of an order is larger (`:939-941, 984-989`).
const LARGEST_FILE: u64 = 1_000_000_000;

/// A folder name nobody else has: 32 random hex digits in the usual 8-4-4-4-12 groups (`QUuid`, `:948`).
fn random_folder_name() -> String {
    let word = || std::collections::hash_map::RandomState::new().build_hasher().finish();
    let hex = format!("{:016x}{:016x}", word(), word());
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

fn data_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("dxf") || e.eq_ignore_ascii_case("zip"))
}

impl NgiiAutomation {
    /// A page offers a file (`:915-956`). `Some` says where it goes; `None` refuses it.
    pub fn download_offered(&mut self, host: &mut dyn PageHost, page: PageId, suggested: &str, now: u64) -> Option<DownloadTarget> {
        if self.stopped_transfers.contains(&page) {
            self.message("중지한 자동 신청에서 늦게 시작된 다운로드를 취소했습니다. 다시 받으려면 공식 신청 내역을 새로 열어 주세요.");
            return None;
        }
        if Some(page) != self.transfer_page || self.stage != Stage::Transfer || self.current_record.is_empty() {
            return None;
        }
        let safe = safe_file_name(suggested);
        let name = if safe.is_empty() { "수치지형도.zip".to_owned() } else { safe };
        let mut record = self.current_record.clone();
        let mut announced = None;
        if !self.manifest.is_empty() {
            let Some(listed) = self.manifest.get(self.received_file_count).filter(|f| f.name == name).cloned() else {
                self.fail(host, "요청한 파일과 도착한 파일이 달라 전송을 중단했습니다.");
                return None;
            };
            let found = self.records.iter().filter_map(Value::as_object).find(|r| r.get("num").and_then(Value::as_str) == Some(listed.number.as_str()));
            let Some(found) = found else {
                self.fail(host, "받은 파일의 도엽 정보를 확인하지 못했습니다.");
                return None;
            };
            record = found.clone();
            // The engine does not say how large a file will be; the list of the transfer page does.
            announced = Some(listed.bytes as u64);
        }
        record.insert("source".into(), json!("NGII suchiQuery.do"));
        if let Some(scope) = &self.scope {
            record.insert("requestScope".into(), json!({"x": scope.x, "y": scope.y, "radius": scope.radius, "numbers": scope.numbers, "cachedNumbers": scope.cached_numbers}));
        }
        if announced.is_some_and(|bytes| bytes > LARGEST_FILE) {
            self.fail(host, "파일 크기가 1GB를 넘어서 자동 내려받기를 중단했습니다. 공식 목록에서 나누어 내려받아 주세요.");
            return None;
        }
        let root = self.config.download_directory.clone();
        if std::fs::create_dir_all(&root).is_err() {
            self.fail(host, "다운로드 폴더를 만들지 못했습니다. 저장 공간과 쓰기 권한을 확인하세요.");
            return None;
        }
        let number = record.get("num").and_then(Value::as_str).unwrap_or_default().to_owned();
        let existing = if self.manifest.is_empty() { self.sheet_directory.clone() } else { self.sheet_directories.get(&number).cloned() };
        let directory = existing.unwrap_or_else(|| root.join(random_folder_name()));
        if std::fs::create_dir_all(&directory).is_err() {
            self.fail(host, "파일을 저장할 폴더를 만들지 못했습니다. 폴더 권한을 확인하세요.");
            return None;
        }
        self.sheet_directory = Some(directory.clone());
        if !self.manifest.is_empty() {
            self.sheet_directories.insert(number, directory.clone());
        }
        self.current_file_name.clone_from(&name);
        (self.received_bytes, self.total_bytes, self.transfer_since) = (0, announced, Some(now));
        let path = directory.join(&name);
        self.receiving.retain(|r| r.path != path);
        self.receiving.push(Receiving { path, name: name.clone(), total: announced, generation: self.generation, directory: directory.clone(), record });
        Some(DownloadTarget { directory, file_name: name })
    }

    /// The transfer that writes `path`, when it belongs to the run as it stands.
    fn receiving_at(&self, path: &Path) -> Option<Receiving> {
        self.receiving.iter().find(|r| r.path == path && r.generation == self.generation && self.stage == Stage::Transfer).cloned()
    }

    /// So many bytes of the file are there (`:966-989`). Bytes arriving keep the transfer alive.
    pub fn download_progress(&mut self, host: &mut dyn PageHost, path: &Path, bytes: u64, now: u64) {
        let Some(file) = self.receiving_at(path) else {
            return;
        };
        if bytes > LARGEST_FILE {
            // The C++ cancels the file, and a cancelled file of an order stops the order (`:1087-1091`).
            self.fail(host, "현재 파일 수신이 취소되어 다음 도엽 신청을 중지했습니다. 기존 지도와 완료한 파일은 유지됩니다.");
            return;
        }
        // `:977-979` — the window shows the file that moved last.
        if file.name != self.current_file_name || bytes != self.received_bytes {
            self.transfer_since = Some(now);
        }
        (self.current_file_name, self.received_bytes, self.total_bytes) = (file.name, bytes, file.total);
    }

    /// The transfer broke off (`:1096-1102`).
    pub fn download_failed(&mut self, host: &mut dyn PageHost, path: &Path) {
        if self.receiving_at(path).is_some() {
            self.fail(host, "내려받기가 중단됐습니다. 전송 연결 끊김\n인터넷 연결과 저장 공간을 확인한 뒤 다시 시도하세요.");
        }
    }

    /// The file is complete at `path` (`:1015-1085`).
    pub fn download_finished(&mut self, host: &mut dyn PageHost, path: &Path, now: u64) {
        let Some(Receiving { directory, record, .. }) = self.receiving_at(path) else {
            return;
        };
        self.receiving.retain(|r| r.path != path);
        if !path.is_file() {
            self.fail(host, "받은 파일을 확인하지 못했습니다. 저장 공간과 폴더 권한을 확인한 뒤 다시 시도하세요.");
            return;
        }
        self.received_file_count += 1;
        self.status = "내려받기가 완료되었습니다. 받은 자료를 확인해 지도에 불러오세요.".to_owned();
        self.completed_paths.push(path.to_path_buf());
        self.expected_sheet_files -= 1;
        let hand_on = |events: &mut Vec<AutoEvent>, paths: &[PathBuf], only_in: Option<&Path>| {
            for done in paths.iter().filter(|p| data_file(p) && only_in.is_none_or(|folder| p.parent() == Some(folder))) {
                events.push(AutoEvent::SheetDownloaded { path: done.clone(), record: record.clone() });
            }
        };
        if !self.manifest.is_empty() {
            let number = record.get("num").and_then(Value::as_str).unwrap_or_default();
            // A sheet is handed on when none of the files still to come belongs to it (`:1034-1046`).
            if !self.manifest.iter().skip(self.received_file_count).any(|f| f.number == number) {
                self.sheet_index += 1;
                hand_on(&mut self.events, &self.completed_paths, Some(&directory));
            }
            if self.expected_sheet_files > 0 {
                // The next file is asked for only after this one is saved (`:1047-1050`).
                self.stage = Stage::Download;
                (self.phase_since, self.transfer_since) = (now, Some(now));
            } else {
                self.order_done(host, now, "선택한 지형도 파일을 모두 받았습니다. 지도 적재 결과를 확인합니다.");
            }
            return;
        }
        if self.expected_sheet_files > 0 {
            return;
        }
        // Metadata files count towards completion, but only data files are handed on (`:1060-1067`).
        hand_on(&mut self.events, &self.completed_paths, None);
        self.sheet_index += 1;
        if self.pending_scope.is_none() && self.sheet_index < self.records.len() {
            self.transfer_page = None;
            self.current_record = self.records[self.sheet_index].as_object().cloned().unwrap_or_default();
            self.stage = Stage::OpenForm;
            self.wait_ticks = 0;
            self.current_file_name.clear();
            (self.sheet_file_count, self.received_file_count, self.total_bytes, self.received_bytes) = (0, 0, None, 0);
            self.set_phase("로그인", now);
        } else {
            self.order_done(host, now, "선택한 지형도 파일을 모두 받았습니다. 좌표 정보와 자료를 확인합니다.");
        }
    }

    /// The last file of an order is there: a waiting request starts, or the run is complete (`:1051-1056`).
    fn order_done(&mut self, host: &mut dyn PageHost, now: u64, words: &str) {
        self.transfer_page = None;
        if let Some(pending) = self.pending_scope.take() {
            self.start(host, pending, now);
        } else {
            self.stage = Stage::Complete;
            self.timer_active = false;
            self.status = words.to_owned();
        }
    }
}
