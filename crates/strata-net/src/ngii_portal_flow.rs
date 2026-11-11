//! 국토정보플랫폼 자동 진행의 단계 — C++ `KaTopographicBrowser::pollAutomation`(:728-913) 그대로.
//!
//! 0.4초마다 화면 안의 자동화 스크립트(`data/providers/ngii-*.js`)에 지금 단계를 시키고,
//! 돌려준 상태로 다음 단계를 정한다: 도엽 확인 → 로그인 → 신청 → 파일 준비 → 다운로드.
//! 되돌릴 수 없는 단계(신청 제출·내려받기 시작)는 **보내기 전에** 다음 단계로 넘겨 둔다:
//! 답을 잃어도 같은 신청을 두 번 내지 않는다.

use serde_json::{json, Map, Value};

/// 응답 없이 기다려 주는 횟수(0.4초 × 75 = 30초). C++ `m_waitTicks == 75`.
pub const MAX_WAIT_TICKS: u32 = 75;

/// 로그인 화면들의 경로(먼저 찾는다). C++ :748-751.
pub const LOGIN_PATHS: [&str; 4] = [
    "/mn/loginPopup.do",
    "/anyid/common/login.do",
    "/anyid/common/anyidLogin.do",
    "/member/login.do",
];
pub const PORTAL_PATH: &str = "/ms/map/NlipMap.do";
pub const DOWNLOAD_PATH: &str = "/pd/purchs/download.do";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Select,
    OpenForm,
    Form,
    Submit,
    Order,
    Files,
    Download,
    Transfer,
    Stopped,
    Complete,
}

impl Stage {
    /// 스크립트에 넘기는 명령 이름.
    #[must_use]
    pub const fn command(self) -> Option<&'static str> {
        Some(match self {
            Self::Select => "select",
            Self::OpenForm => "openForm",
            Self::Form => "form",
            Self::Submit => "submit",
            Self::Order => "order",
            Self::Files => "files",
            Self::Download => "download",
            Self::Transfer | Self::Stopped | Self::Complete => return None,
        })
    }
}

/// 화면에 한 번 시킬 일.
#[derive(Debug, Clone, PartialEq)]
pub struct PollRequest {
    /// 찾을 화면의 경로(앞의 것이 먼저).
    pub paths: Vec<String>,
    /// 스크립트 입력(계정 값은 들어 있지 않다).
    pub input: Value,
}

#[derive(Debug, Clone)]
pub struct PortalFlow {
    pub stage: Stage,
    /// 사용자에게 보이는 단계 이름.
    pub phase: String,
    /// 마지막 안내(입력 필요, 지연 등).
    pub message: String,
    /// 중단한 이유. 있으면 더 진행하지 않는다.
    pub failed: Option<String>,
    /// 공식 검색이 돌려준, 신청할 자료 목록.
    pub records: Vec<Value>,
    pub expected_files: u64,
    pub manifest: Vec<Value>,
    scope: Map<String, Value>,
    generation: u64,
    wait_ticks: u32,
    /// `Submit`·`Download` 를 보내며 미리 넘긴 단계의 원래 자리.
    sent_from: Option<Stage>,
}

impl PortalFlow {
    /// `scope`: 스크립트의 `select` 입력(x, y, radius, numbers, cachedNumbers — EPSG:5179).
    #[must_use]
    pub fn new(scope: Map<String, Value>, generation: u64) -> Self {
        Self {
            stage: Stage::Select,
            phase: "도엽 확인".to_owned(),
            message: String::new(),
            failed: None,
            records: Vec::new(),
            expected_files: 0,
            manifest: Vec::new(),
            scope,
            generation,
            wait_ticks: 0,
            sent_from: None,
        }
    }

    fn fail(&mut self, why: &str) {
        self.failed = Some(why.to_owned());
        self.stage = Stage::Stopped;
    }

    fn current(&self) -> Value {
        self.records.first().cloned().unwrap_or(Value::Null)
    }

    /// 이번 차례에 화면에 시킬 일. 없으면(전송 중·끝남) `None`.
    pub fn poll(&mut self, birth_date: &str, purpose: &str) -> Option<PollRequest> {
        let command = self.stage.command()?;
        let download = matches!(self.stage, Stage::Files | Stage::Download);
        let paths: Vec<String> = if download {
            vec![DOWNLOAD_PATH.to_owned()]
        } else {
            LOGIN_PATHS.iter().chain([&PORTAL_PATH]).map(|p| (*p).to_owned()).collect()
        };
        let mut input = self.scope.clone();
        input.insert("command".to_owned(), json!(command));
        input.insert("diagnostics".to_owned(), json!(false));
        input.insert("generation".to_owned(), json!(self.generation.to_string()));
        input.insert("number".to_owned(), self.current().get("num").cloned().unwrap_or(Value::Null));
        input.insert("record".to_owned(), self.current());
        input.insert("records".to_owned(), Value::Array(self.records.clone()));
        input.insert("fileIndex".to_owned(), json!(0));
        if matches!(self.stage, Stage::Form | Stage::Submit) {
            input.insert("birthDate".to_owned(), json!(birth_date.trim()));
            input.insert("purpose".to_owned(), json!(purpose.trim()));
        }
        // 되돌릴 수 없는 단계는 먼저 넘겨 둔다(C++ :787-795).
        self.sent_from = match self.stage {
            Stage::Submit => {
                self.stage = Stage::Order;
                Some(Stage::Submit)
            }
            Stage::Download => {
                self.stage = Stage::Transfer;
                self.wait_ticks = 0;
                Some(Stage::Download)
            }
            _ => None,
        };
        Some(PollRequest { paths, input: Value::Object(input) })
    }

    /// 시킬 화면이 아직 없다(열리는 중). 30초를 넘기면 중단한다.
    pub fn on_missing_page(&mut self) {
        // 보내지 못했으므로 미리 넘긴 단계를 되돌린다.
        if let Some(stage) = self.sent_from.take() {
            self.stage = stage;
        }
        self.wait_ticks += 1;
        if self.wait_ticks == MAX_WAIT_TICKS {
            let download = matches!(self.stage, Stage::Files | Stage::Download);
            self.fail(if download {
                "파일 전송 화면이 열리지 않았습니다. 신청 내역과 연결 상태를 확인하세요."
            } else {
                "공식 사이트의 응답을 확인하지 못했습니다. 계정 설정과 연결 상태를 확인하세요."
            });
        }
    }

    /// 스크립트의 답. `command` 는 화면에서 실제로 돈 명령(로그인 화면이면 `login`).
    pub fn on_result(&mut self, command: &str, result: &Value) {
        let sent_from = self.sent_from.take();
        if command == "login" {
            // 로그인 화면이 먼저 받았다 — 제출·내려받기는 일어나지 않았으니 제자리로.
            if let Some(stage) = sent_from {
                self.stage = stage;
            }
            self.phase = "로그인".to_owned();
        }
        let state = result.get("state").and_then(Value::as_str).unwrap_or("");
        let message = result.get("message").and_then(Value::as_str).unwrap_or("");
        if state.is_empty() {
            self.wait_ticks += 1;
            if self.wait_ticks == MAX_WAIT_TICKS {
                self.fail("포털 자동 진행 응답을 확인하지 못했습니다. 연결 상태를 확인한 뒤 다시 시도하세요.");
            }
            return;
        }
        match (command, state) {
            (_, "blocked") => self.fail(message),
            (_, "input" | "login") => {
                self.message = message.to_owned();
                match command {
                    "submit" => self.stage = Stage::Form,
                    "download" => self.stage = Stage::Files,
                    _ => {}
                }
            }
            ("submit", "waitingForm") => self.stage = Stage::Form,
            ("order", "orderReady") => {
                self.stage = Stage::Files;
                self.wait_ticks = 0;
                self.phase = "파일 준비".to_owned();
            }
            ("order", "delayed") => self.message = message.to_owned(),
            ("select", "unavailable") => {
                self.stage = Stage::Stopped;
                self.message = "이 범위에서 제공 중인 자료가 없습니다.".to_owned();
            }
            ("select", "selected" | "cached") => {
                self.records = result.get("downloadItems").and_then(Value::as_array).cloned().unwrap_or_default();
                if self.records.is_empty() {
                    self.stage = Stage::Complete;
                    self.message = "필요한 자료를 모두 보관하고 있습니다. 다시 신청하지 않습니다.".to_owned();
                } else {
                    self.stage = Stage::OpenForm;
                    self.wait_ticks = 0;
                    self.phase = "로그인".to_owned();
                }
            }
            ("openForm", "form") => {
                self.stage = Stage::Form;
                self.wait_ticks = 0;
                self.phase = "신청".to_owned();
            }
            ("openForm", "waitingForm" | "openingForm") => self.phase = "신청".to_owned(),
            ("form", "ready") => self.stage = Stage::Submit,
            ("files", "ready") => {
                self.expected_files = result.get("files").and_then(Value::as_u64).unwrap_or(0);
                self.manifest = result.get("manifest").and_then(Value::as_array).cloned().unwrap_or_default();
                self.stage = Stage::Download;
                self.phase = "다운로드".to_owned();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "ngii_portal_flow_tests.rs"]
mod tests;
