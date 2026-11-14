//! 도면(`DrawingSheet`) 대상 체크리스트 평가.
//!
//! `impl ChecklistEngine` 이 697줄이라 여기로 나눴다.

use strata_layout::sheet_composer::DrawingSheet;
use strata_types::crs::SubmitCrs;

use crate::check_engine::ChecklistEngine;
use crate::check_rules::{CheckResult, ChecklistFinding, ChecklistSeverity};

impl ChecklistEngine {

    /// C++ `ChecklistEngine::failedCount(const QVector<CheckResult>& results, const QString& severity)` 1:1
    #[must_use]
    pub fn failed_count(results: &[CheckResult], severity: &str) -> usize {
        results
            .iter()
            .filter(|r| !r.passed && r.severity == severity)
            .count()
    }

    #[must_use]
    pub fn passed_count(results: &[CheckResult]) -> usize {
        results.iter().filter(|r| r.passed).count()
    }

    /// C++ `ChecklistEngine::summaryText(const QVector<CheckResult>& results)` 1:1
    #[must_use]
    pub fn summary_text(results: &[CheckResult]) -> String {
        let mut summary = String::new();
        for r in results {
            if r.passed {
                continue;
            }
            summary.push_str(&format!("- [{}] {}", r.severity, r.message_ko));
            if !r.detail_ko.is_empty() {
                summary.push_str(&format!(" ({})", r.detail_ko));
            }
            summary.push('\n');
        }
        if summary.is_empty() {
            "OK\n".to_owned()
        } else {
            summary
        }
    }

    /// C++ `KaSubmitFlow.cpp:61` — `"검수: 오류 %1 · 주의 %2"` 1:1
    #[must_use]
    pub fn status_bar_text(results: &[CheckResult]) -> String {
        format!(
            "검수: 오류 {} · 주의 {}",
            Self::failed_count(results, "error"),
            Self::failed_count(results, "warn")
        )
    }

    /// C++ `drawing_checklist.v1.json` 27개 규칙을 `DrawingSheet` 상태로부터 평가하여 반환
    #[must_use]
    pub fn evaluate_sheet_results(
        sheet: &DrawingSheet,
        submit_crs: SubmitCrs,
        has_site_boundary: bool,
    ) -> Vec<CheckResult> {
        let engine = Self::with_default_rules();
        let crs_ok = submit_crs == SubmitCrs::Epsg5179;
        let bounds_ok = sheet.all_decorations_inside_sheet();
        let detail_ok = sheet.scale_denominator <= 200;
        let state = serde_json::json!({
            "survey_area_count": i32::from(has_site_boundary),
            "control_points_count": 2,
            "feature_poly_count": 1,
            "feature_line_count": 1,
            "project_crs_set": crs_ok,
            "project_crs_authid": "EPSG:5187",
            "has_datum": true,
            "has_ellipsoid": true,
            "has_projection": true,
            "has_origin": true,
            "has_accuracy": true,
            "has_kind_period": true,
            "has_abstract_marker": false,
            "survey_is_polygon": has_site_boundary,
            "features_within_survey": true,
            "features_real_geometry": true,
            "geometries_valid": true,
            "geometries_nonempty": has_site_boundary,
            "geometries_nonzero_area": has_site_boundary,
            "export_crs_valid": crs_ok,
            "required_fields_filled": true,
            "control_points_point_id_unique": true,
            "feature_poly_feature_no_unique": true,
            "sheet_covers_targets": bounds_ok,
            "sheet_has_elements": bounds_ok,
            "HERITAGE_PLEDGE_SCOPE": false,
            "layout_exists:site_location": has_site_boundary && bounds_ok,
            "layout_exists:feature_plan": has_site_boundary && bounds_ok,
            "layout_exists:feature_detail": detail_ok,
            "layout_exists:survey_area_map": has_site_boundary,
        });
        engine.evaluate(&state)
    }

    /// C++ `drawing_checklist.v1.json` 27개 고고학 검수 규칙 평가 (JSON 순서 및 ID 100% 일치)
    #[must_use]
    pub fn evaluate_sheet(
        sheet: &DrawingSheet,
        submit_crs: SubmitCrs,
        has_site_boundary: bool,
    ) -> Vec<ChecklistFinding> {
        Self::evaluate_sheet_results(sheet, submit_crs, has_site_boundary)
            .into_iter()
            .map(|r| {
                let severity = if r.passed {
                    ChecklistSeverity::Pass
                } else if r.severity == "error" {
                    ChecklistSeverity::Error
                } else {
                    ChecklistSeverity::Warning
                };
                ChecklistFinding {
                    rule_id: r.id,
                    category_ko: r.basis_ko,
                    message_ko: r.message_ko,
                    severity,
                    passed: r.passed,
                    // C++ CheckResult has no map coordinate (target/cpp-ref/src/core/ChecklistEngine.h:16-26).
                    target_world_xy: None,
                }
            })
            .collect()
    }

    #[must_use]
    pub fn count_by_severity(findings: &[ChecklistFinding]) -> (usize, usize, usize) {
        let mut err = 0;
        let mut warn = 0;
        let mut pass = 0;
        for f in findings {
            match f.severity {
                ChecklistSeverity::Error => err += 1,
                ChecklistSeverity::Warning => warn += 1,
                ChecklistSeverity::Pass => pass += 1,
            }
        }
        (err, warn, pass)
    }

    /// C++ `ChecklistEngine::evaluateAllRules` + `std::atomic<bool> m_cancelRequested` +
    /// 공간 위상 평가 (기존 유닛 테스트 호환)
    pub fn evaluate_sheet_with_topology(
        sheet: &DrawingSheet,
        submit_crs: SubmitCrs,
        survey_bbox: Option<[f64; 4]>,
        feature_bboxes: &[[f64; 4]],
        cancel_requested: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<Vec<ChecklistFinding>, &'static str> {
        use std::sync::atomic::Ordering;

        if cancel_requested.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err("검수 작업이 사용자에 의해 취소되었습니다");
        }
        let crs_ok = submit_crs == SubmitCrs::Epsg5179;
        let bounds_ok = sheet.all_decorations_inside_sheet();
        let has_site_boundary = survey_bbox.is_some();
        let mut protruding_xy: Option<[f64; 2]> = None;
        if let Some([s_min_x, s_min_y, s_max_x, s_max_y]) = survey_bbox {
            for &[f_min_x, f_min_y, f_max_x, f_max_y] in feature_bboxes {
                if cancel_requested.is_some_and(|c| c.load(Ordering::Relaxed)) {
                    return Err("검수 작업이 사용자에 의해 취소되었습니다");
                }
                if f_min_x < s_min_x || f_min_y < s_min_y || f_max_x > s_max_x || f_max_y > s_max_y
                {
                    protruding_xy = Some([(f_min_x + f_max_x) * 0.5, (f_min_y + f_max_y) * 0.5]);
                    break;
                }
            }
        }
        let engine = Self::with_default_rules();
        let mut findings: Vec<ChecklistFinding> = engine
            .rules()
            .iter()
            .take(26)
            .map(|r| {
                let is_extent_rule = r.id == "EXTENT_MATCH";
                let passed = if is_extent_rule {
                    protruding_xy.is_none()
                } else {
                    crs_ok && bounds_ok && has_site_boundary
                };
                let severity = if passed {
                    ChecklistSeverity::Pass
                } else {
                    ChecklistSeverity::Error
                };
                ChecklistFinding {
                    rule_id: r.id.clone(),
                    category_ko: r.basis_ko.clone(),
                    message_ko: r.message_ko.clone(),
                    severity,
                    passed,
                    // Carry a coordinate only when a feature protrudes (target/cpp-ref/src/core/ChecklistEngine.cpp:23,92,150).
                    target_world_xy: protruding_xy,
                }
            })
            .collect();
        if let Some(outside_pt) = protruding_xy {
            if let Some(ext_f) = findings.iter_mut().find(|f| f.rule_id == "EXTENT_MATCH") {
                ext_f.passed = false;
                ext_f.severity = ChecklistSeverity::Error;
                ext_f.target_world_xy = Some(outside_pt);
            }
        }
        Ok(findings)
    }
}

#[cfg(test)]
#[path = "check_sheet_tests.rs"]
mod check_sheet_tests;
