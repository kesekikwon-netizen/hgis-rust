//! 체크리스트 규칙 문서(JSON)와 규칙·결과 타입.
//!
//! `checklist_engine` 에서 분리했다.

use serde::{Deserialize, Serialize};

/// C++ `data/rules/drawing_checklist.v1.json` (27개 규칙 정본 리소스)
pub const DRAWING_CHECKLIST_V1_JSON: &str = include_str!("../data/rules/drawing_checklist.v1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistSeverity {
    Error,
    Warning,
    Pass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChecklistFinding {
    pub rule_id: String,
    pub category_ko: String,
    pub message_ko: String,
    pub severity: ChecklistSeverity,
    pub passed: bool,
    pub target_world_xy: Option<[f64; 2]>,
}

/// C++ `ChecklistEngine.h` — `CheckTarget` (위반 도형/레이어 대상)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckTarget {
    #[serde(default)]
    pub layer_id: String,
    #[serde(default)]
    pub layer_name: String,
    #[serde(default = "default_fid")]
    pub feature_id: i64,
    #[serde(default)]
    pub label: String,
}

const fn default_fid() -> i64 {
    -1
}

/// C++ `ChecklistEngine.h` — `Rule` (`drawing_checklist.v1.json` 단일 항목)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistRule {
    pub id: String,
    pub severity: String,
    pub message_ko: String,
    pub check_type: String,
    pub fix_ko: String,
    pub basis_ko: String,
    #[serde(default)]
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ChecklistRulesDocument {
    #[serde(default)]
    pub(crate) version: i32,
    #[serde(default)]
    pub(crate) title_ko: String,
    #[serde(default)]
    pub(crate) rules: Vec<ChecklistRule>,
}

/// C++ `ChecklistEngine.h` — `CheckResult` (27개 규칙 평가 결과)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub id: String,
    pub severity: String,
    pub message_ko: String,
    pub passed: bool,
    pub fix_ko: String,
    pub basis_ko: String,
    pub action: String,
    pub detail_ko: String,
    pub targets: Vec<CheckTarget>,
}
