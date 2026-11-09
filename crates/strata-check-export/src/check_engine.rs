//! `ChecklistEngine` — 규칙 적재와 항목 평가.
//!
//! `impl ChecklistEngine` 이 697줄이라 세 갈래로 나눴다.
//! 도면 평가는 `check_sheet`, 기하 검사는 `check_geometry` 에 있다.

use crate::check_rules::{
    CheckResult, CheckTarget, ChecklistRule, ChecklistRulesDocument,
    DRAWING_CHECKLIST_V1_JSON,
};

#[derive(Debug, Clone)]
pub struct ChecklistEngine {
    rules: Vec<ChecklistRule>,
    path: String,
    title: String,
}

impl Default for ChecklistEngine {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

fn verb_of(check_type: &str) -> &str {
    check_type.split(':').next().unwrap_or("")
}

fn arg_of(check_type: &str, index: usize) -> &str {
    check_type.split(':').nth(index).unwrap_or("")
}

fn default_for(key: &str) -> bool {
    if key.starts_with("layout_exists:") {
        return false;
    }
    !matches!(
        key,
        "has_datum" | "has_ellipsoid" | "has_projection" | "project_crs_set"
    )
}

fn targets_from(state: &serde_json::Value, key: &str) -> Vec<CheckTarget> {
    let Some(arr) = state
        .get("offenders")
        .and_then(|o| o.get(key))
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    arr.iter()
        .map(|v| CheckTarget {
            layer_id: v
                .get("layer_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_owned(),
            layer_name: v
                .get("layer_name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_owned(),
            feature_id: v
                .get("fid")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(-1),
            label: v
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_owned(),
        })
        .collect()
}

impl ChecklistEngine {

    /// 내장 `drawing_checklist.v1.json` (27개 규칙) 로드
    #[must_use]
    pub fn with_default_rules() -> Self {
        let mut engine = Self {
            rules: Vec::new(),
            path: String::new(),
            title: String::new(),
        };
        let _ = engine.load_rules_from_str(DRAWING_CHECKLIST_V1_JSON);
        engine
    }

    /// C++ `ChecklistEngine::loadRules(const QString& jsonPath)` 1:1
    pub fn load_rules(&mut self, json_path: &std::path::Path) -> bool {
        let Ok(raw) = std::fs::read_to_string(json_path) else {
            return false;
        };
        if self.load_rules_from_str(&raw) {
            self.path = json_path.to_string_lossy().into_owned();
            true
        } else {
            false
        }
    }

    pub fn load_rules_from_str(&mut self, json_str: &str) -> bool {
        let Ok(doc) = serde_json::from_str::<ChecklistRulesDocument>(json_str) else {
            return false;
        };
        let mut loaded = Vec::new();
        for mut r in doc.rules {
            if r.id.is_empty() {
                continue;
            }
            if r.severity != "warn" {
                r.severity = "error".to_owned();
            }
            loaded.push(r);
        }
        if loaded.is_empty() {
            return false;
        }
        self.rules = loaded;
        self.title = if doc.title_ko.is_empty() || doc.version <= 0 {
            doc.title_ko
        } else {
            format!("{} v{}", doc.title_ko, doc.version)
        };
        true
    }

    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn rules(&self) -> &[ChecklistRule] {
        &self.rules
    }

    /// C++ `ChecklistEngine::stateKeyFor(const QString& ct)` 1:1
    #[must_use]
    pub fn state_key_for(ct: &str) -> String {
        let verb = verb_of(ct);
        match verb {
            "layer_nonempty" | "count_min" => format!("{}_count", arg_of(ct, 1)),
            "field_nonempty" => format!("has_{}", arg_of(ct, 2)),
            "field_any" => {
                let joined = arg_of(ct, 2)
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("_");
                format!("has_{joined}")
            }
            "project_crs_set" => "project_crs_set".to_owned(),
            "crs_in" => "project_crs_authid".to_owned(),
            "geometry_type" => {
                if arg_of(ct, 1).starts_with("survey_area") {
                    "survey_is_polygon".to_owned()
                } else {
                    "features_real_geometry".to_owned()
                }
            }
            "forbid_abstract_marker" => "has_abstract_marker".to_owned(),
            "geometry_valid" => "geometries_valid".to_owned(),
            "geometry_nonempty" => "geometries_nonempty".to_owned(),
            "geometry_nonzero_area" => "geometries_nonzero_area".to_owned(),
            "extent_within" => "features_within_survey".to_owned(),
            "layout_exists" => ct.to_owned(),
            "export_ready" => "export_crs_valid".to_owned(),
            "sheet_covers" => "sheet_covers_targets".to_owned(),
            "sheet_elements" => "sheet_has_elements".to_owned(),
            "fields_required" => "required_fields_filled".to_owned(),
            "unique_field" => format!("{}_{}_unique", arg_of(ct, 1), arg_of(ct, 2)),
            "state" => arg_of(ct, 1).to_owned(),
            _ => String::new(),
        }
    }

    /// C++ `ChecklistEngine::isSupportedCheckType(const QString& checkType)` 1:1
    #[must_use]
    pub fn is_supported_check_type(check_type: &str) -> bool {
        let verb = verb_of(check_type);
        if verb == "count_min" {
            return arg_of(check_type, 2).parse::<i64>().is_ok()
                && !arg_of(check_type, 1).is_empty();
        }
        if verb == "export_ready" {
            return arg_of(check_type, 1) == "crs";
        }
        !Self::state_key_for(check_type).is_empty()
    }

    fn eval_one(rule: &ChecklistRule, state: &serde_json::Value) -> (bool, bool) {
        let supported = Self::is_supported_check_type(&rule.check_type);
        if !supported {
            return (false, false);
        }
        let ct = &rule.check_type;
        let verb = verb_of(ct);
        let key = Self::state_key_for(ct);
        let passed = match verb {
            "layer_nonempty" => {
                state
                    .get(&key)
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0)
                    > 0
            }
            "count_min" => {
                let min_v = arg_of(ct, 2).parse::<i64>().unwrap_or(0);
                state
                    .get(&key)
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0)
                    >= min_v
            }
            "forbid_abstract_marker" => !state
                .get(&key)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            "crs_in" => {
                let auth = state
                    .get(&key)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if auth.is_empty() {
                    true
                } else {
                    arg_of(ct, 1)
                        .split(',')
                        .filter(|s| !s.trim().is_empty())
                        .any(|code| auth.eq_ignore_ascii_case(&format!("EPSG:{}", code.trim())))
                }
            }
            _ => state
                .get(&key)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_else(|| default_for(&key)),
        };
        (passed, true)
    }

    /// C++ `ChecklistEngine::evaluate(const QJsonObject& projectState)` 1:1
    #[must_use]
    pub fn evaluate(&self, project_state: &serde_json::Value) -> Vec<CheckResult> {
        let notes = project_state.get("notes");
        let mut out = Vec::with_capacity(self.rules.len());
        for r in &self.rules {
            let (passed, supported) = Self::eval_one(r, project_state);
            let mut detail_ko = String::new();
            let mut targets = Vec::new();
            if !supported {
                detail_ko = format!(
                    "이 프로그램이 모르는 검사({})라 통과로 볼 수 없습니다.",
                    r.check_type
                );
            } else if !passed {
                let key = Self::state_key_for(&r.check_type);
                targets = targets_from(project_state, &key);
                if let Some(note_str) = notes
                    .and_then(|n| n.get(&key))
                    .and_then(serde_json::Value::as_str)
                {
                    detail_ko = note_str.to_owned();
                }
            }
            out.push(CheckResult {
                id: r.id.clone(),
                severity: r.severity.clone(),
                message_ko: r.message_ko.clone(),
                passed,
                fix_ko: r.fix_ko.clone(),
                basis_ko: r.basis_ko.clone(),
                action: r.action.clone(),
                detail_ko,
                targets,
            });
        }
        out
    }
}
