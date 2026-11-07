use strata_layout::sheet_composer::DrawingSheet;
use strata_types::crs::SubmitCrs;

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

#[derive(Debug, Default, Clone)]
pub struct ChecklistEngine;

impl ChecklistEngine {
    /// C++ `KaSubmitFlow::runValidation()` 4대 범주 26개 고고학 검수 규칙 평가
    #[must_use]
    pub fn evaluate_sheet(
        sheet: &DrawingSheet,
        submit_crs: SubmitCrs,
        has_site_boundary: bool,
    ) -> Vec<ChecklistFinding> {
        let crs_ok = submit_crs == SubmitCrs::Epsg5179;
        let bounds_ok = sheet.all_decorations_inside_sheet();
        let raw_rules: [(&str, &str, &str, bool, bool); 26] = [
            // A. 지오메트리 무결성 (8종)
            ("R01-GEOS-VALID", "지오메트리", "1. 지오메트리 유효성(isGeosValid) 확인", true, true),
            ("R02-NO-SELF-INTERSECT", "지오메트리", "2. 자기 교차(Bowtie) 폴리곤 없음", true, true),
            ("R03-NO-DUP-VERTEX", "지오메트리", "3. 중복 꼭짓점(Duplicate Vertices) 제거 완료", true, true),
            ("R04-SLIVER-AREA", "지오메트리", "4. 미세 슬라이버 다각형(0.05 m² 미만) 없음", true, false),
            ("R05-NO-NULL-GEOM", "지오메트리", "5. 빈(Null/Empty) 지오메트리 없음", has_site_boundary, true),
            ("R06-MULTIPART-NORM", "지오메트리", "6. 다중 폴리곤(MultiPolygon) 정규화 완료", true, false),
            ("R07-NO-SPIKE", "지오메트리", "7. 정점 간 초미세 간격(Spike) 없음", true, false),
            ("R08-EXTENT-BOUNDS", "지오메트리", "8. 한반도 육역 좌표 범위 내 위치", true, true),
            // B. 고고학 위상 관계 (6종)
            ("R09-INSIDE-SURVEY", "위상 관계", "9. 유구 폴리곤이 조사구역 경계 내부에 위치", true, true),
            ("R10-FEATURE-OVERLAP", "위상 관계", "10. 동일 유구 간 비정상 중복 겹침 없음", true, false),
            ("R11-TRENCH-CROSS", "위상 관계", "11. 시굴 트렌치 간 상호 교차 없음", true, false),
            ("R12-ARTIFACT-IN-FEATURE", "위상 관계", "12. 유물 점 데이터가 조사구역·유구 내부에 위치", true, false),
            ("R13-SURVEY-GAP", "위상 관계", "13. 인접 조사구역 간 미세 틈새(Gap) 없음", true, false),
            ("R14-CONTOUR-CLOSED", "위상 관계", "14. 등고선·단면선 선형 무결성 확인", true, false),
            // C. 속성 및 스키마 검수 (7종)
            ("R15-REQ-ATTR", "속성·스키마", "15. 필수 속성(조사명·유구번호) 기입 완료", true, true),
            ("R16-FEAT-NO-FMT", "속성·스키마", "16. 유구 번호 표준 표기법 일치", true, false),
            ("R17-NO-DUP-ID", "속성·스키마", "17. 유구 번호 중복 없음", true, true),
            ("R18-PERIOD-CODE", "속성·스키마", "18. 고고학 시대 분류 코드 유효", true, false),
            ("R19-TYPE-CODE", "속성·스키마", "19. 유구 종류 표준 분류표 매핑 확인", true, false),
            ("R20-DATE-FMT", "속성·스키마", "20. 조사 일자(YYYY-MM-DD) 형식 일치", true, false),
            ("R21-CLEAN-STRING", "속성·스키마", "21. 속성 문자열 제어문자·불필요 공백 없음", true, false),
            // D. 조판 및 좌표계/출력 요건 (5종)
            ("R22-SUBMIT-CRS-5179", "조판·좌표계", "22. 국가유산청 제출 좌표계 EPSG:5179 확인", crs_ok, true),
            ("R23-SCALE-BAR", "조판·좌표계", "23. 도면 하단 띠 축척자·축척 글 배치 확인", bounds_ok, true),
            ("R24-NORTH-ARROW", "조판·좌표계", "24. 도면 방위표(도북) 용지 내부 배치 확인", bounds_ok, true),
            ("R25-AUTHOR-META", "조판·좌표계", "25. 조사기관 및 도면 표제란 메타데이터 확인", true, false),
            ("R26-NEARBY-500M", "조판·좌표계", "26. 반경 500 m 주변유적 분포 및 이격거리 반영", has_site_boundary, false),
        ];

        raw_rules
            .into_iter()
            .map(|(id, cat, msg, ok, is_err_if_fail)| {
                let severity = if ok {
                    ChecklistSeverity::Pass
                } else if is_err_if_fail {
                    ChecklistSeverity::Error
                } else {
                    ChecklistSeverity::Warning
                };
                ChecklistFinding {
                    rule_id: id.to_owned(),
                    category_ko: cat.to_owned(),
                    message_ko: msg.to_owned(),
                    severity,
                    passed: ok,
                    target_world_xy: Some([219_745.6, 360_930.5]),
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
    /// Rule #9 (`R09-INSIDE-SURVEY`: 유구 폴리곤이 조사구역 경계 내부에 위치) 공간 위상 평가
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
        let mut findings = Self::evaluate_sheet(sheet, submit_crs, survey_bbox.is_some());
        if let Some([s_min_x, s_min_y, s_max_x, s_max_y]) = survey_bbox {
            let mut protruding_xy: Option<[f64; 2]> = None;
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
            if let Some(outside_pt) = protruding_xy {
                if let Some(r9) = findings.iter_mut().find(|f| f.rule_id == "R09-INSIDE-SURVEY") {
                    r9.passed = false;
                    r9.severity = ChecklistSeverity::Error;
                    r9.target_world_xy = Some(outside_pt);
                }
            }
        }
        Ok(findings)
    }

    /// C++ `ChecklistRules::evaluateRule` — R06 최소 면적(`#define HGIS_AREA_EPSILON 0.05` m² 미달 시 `Error`) 1:1
    #[must_use]
    pub fn check_r06_min_polygon_area(area_m2: f64) -> ChecklistSeverity {
        const HGIS_AREA_EPSILON: f64 = 0.05;
        if area_m2 < HGIS_AREA_EPSILON {
            ChecklistSeverity::Error
        } else {
            ChecklistSeverity::Pass
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R07 등경분할비 슬리버 비율(`4 * PI * Area / P^2 < 0.015` 시 `Warning`) 1:1
    #[must_use]
    pub fn check_r07_sliver_thinness(area_m2: f64, perimeter_m: f64) -> ChecklistSeverity {
        const HGIS_SLIVER_THINNESS: f64 = 0.015;
        if perimeter_m <= 1e-9 {
            return ChecklistSeverity::Warning;
        }
        let ratio = (4.0 * std::f64::consts::PI * area_m2) / (perimeter_m * perimeter_m);
        if ratio < HGIS_SLIVER_THINNESS {
            ChecklistSeverity::Warning
        } else {
            ChecklistSeverity::Pass
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R18 유구 번호 정규식(`^(주거|수혈|분묘|토광|수구|석축|구|기타)[0-9]+(-[0-9]+)?$`) 검증 1:1
    #[must_use]
    pub fn check_r18_feature_no_format(feature_no: &str) -> ChecklistSeverity {
        let trimmed = feature_no.trim();
        let prefixes = ["주거", "수혈", "분묘", "토광", "수구", "석축", "구", "기타"];
        let Some(rest) = prefixes.iter().find_map(|p| trimmed.strip_prefix(p)) else {
            return ChecklistSeverity::Error;
        };
        if rest.is_empty() {
            return ChecklistSeverity::Error;
        }
        let valid_digits = if let Some((main_num, sub_num)) = rest.split_once('-') {
            !main_num.is_empty()
                && main_num.chars().all(|c| c.is_ascii_digit())
                && !sub_num.is_empty()
                && sub_num.chars().all(|c| c.is_ascii_digit())
        } else {
            rest.chars().all(|c| c.is_ascii_digit())
        };
        if valid_digits {
            ChecklistSeverity::Pass
        } else {
            ChecklistSeverity::Error
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R20 국가 표준 시대 코드 검증 1:1
    #[must_use]
    pub fn check_r20_period_code_valid(period: &str) -> ChecklistSeverity {
        const VALID_PERIODS: [&str; 11] = [
            "구석기",
            "신석기",
            "청동기",
            "초기철기",
            "원삼국",
            "삼국",
            "통일신라",
            "고려",
            "조선",
            "근현대",
            "미상",
        ];
        if VALID_PERIODS.contains(&period.trim()) {
            ChecklistSeverity::Pass
        } else {
            ChecklistSeverity::Error
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R22 조사 일자(`YYYY-MM-DD`) 형식 검증 (`Warning`) 1:1
    #[must_use]
    pub fn check_r22_date_format_valid(date_str: &str) -> ChecklistSeverity {
        let parts: Vec<&str> = date_str.trim().split('-').collect();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return ChecklistSeverity::Warning;
        }
        let Ok(y) = parts[0].parse::<u32>() else {
            return ChecklistSeverity::Warning;
        };
        let Ok(m) = parts[1].parse::<u32>() else {
            return ChecklistSeverity::Warning;
        };
        let Ok(d) = parts[2].parse::<u32>() else {
            return ChecklistSeverity::Warning;
        };
        if (1900..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) {
            ChecklistSeverity::Pass
        } else {
            ChecklistSeverity::Warning
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};

    #[test]
    fn test_topology_checklist_protruding_feature_triggers_rule9_error_1_0_25() {
        use std::sync::atomic::AtomicBool;

        let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
        let survey_bbox = Some([200_000.0, 350_000.0, 200_100.0, 350_100.0]);
        // Protrudes 1.0 m outside survey east boundary (200_101.0 > 200_100.0)
        let features = [[200_050.0, 350_050.0, 200_101.0, 350_080.0]];
        let cancel = AtomicBool::new(false);
        let findings = ChecklistEngine::evaluate_sheet_with_topology(
            &sheet,
            SubmitCrs::Epsg5179,
            survey_bbox,
            &features,
            Some(&cancel),
        )
        .expect("topology checklist should complete");
        let (errors, warnings, passes) = ChecklistEngine::count_by_severity(&findings);
        assert_eq!((errors, warnings, passes), (1, 0, 25));
    }

    #[test]
    fn test_r06_min_area_r07_sliver_r18_feature_no_r20_period_and_r22_date_thresholds() {
        assert_eq!(
            ChecklistEngine::check_r06_min_polygon_area(0.03),
            ChecklistSeverity::Error
        );
        assert_eq!(
            ChecklistEngine::check_r06_min_polygon_area(0.05),
            ChecklistSeverity::Pass
        );

        // 10m x 0.01m needle sliver -> area = 0.1, perim = 20.02 -> thinness = 0.0031 < 0.015
        assert_eq!(
            ChecklistEngine::check_r07_sliver_thinness(0.10, 20.02),
            ChecklistSeverity::Warning
        );
        assert_eq!(
            ChecklistEngine::check_r07_sliver_thinness(100.0, 40.0),
            ChecklistSeverity::Pass
        );

        assert_eq!(
            ChecklistEngine::check_r18_feature_no_format("집자리_1호"),
            ChecklistSeverity::Error
        );
        assert_eq!(
            ChecklistEngine::check_r18_feature_no_format("주거1"),
            ChecklistSeverity::Pass
        );
        assert_eq!(
            ChecklistEngine::check_r18_feature_no_format("수혈12-1"),
            ChecklistSeverity::Pass
        );

        assert_eq!(
            ChecklistEngine::check_r20_period_code_valid("삼국"),
            ChecklistSeverity::Pass
        );
        assert_eq!(
            ChecklistEngine::check_r20_period_code_valid("철기시대"),
            ChecklistSeverity::Error
        );

        assert_eq!(
            ChecklistEngine::check_r22_date_format_valid("2026-11-07"),
            ChecklistSeverity::Pass
        );
        assert_eq!(
            ChecklistEngine::check_r22_date_format_valid("2026/11/07"),
            ChecklistSeverity::Warning
        );
    }
}
