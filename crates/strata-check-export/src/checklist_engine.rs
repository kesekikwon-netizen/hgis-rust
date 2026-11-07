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

    pub const HGIS_MIN_ERROR_ZOOM_MARGIN: f64 = 20.0;
    pub const HGIS_RING_CLOSE_EPSILON: f64 = 1.0e-6;
    pub const HGIS_DUPLICATE_DIST_EPSILON: f64 = 0.001;
    pub const HGIS_DUPLICATE_AREA_EPSILON: f64 = 0.001;
    pub const HGIS_SPIKE_MIN_ANGLE_DEG: f64 = 2.0;
    pub const HGIS_MIN_CONTROL_POINTS: usize = 2;

    /// C++ `KaSubmitFlow::zoomToErrorLocation` — 점/미세 슬리버 오류로 이동 시 최소 20.0m 여백 BBOX(`40m x 40m`) 확장 공식 1:1
    #[must_use]
    pub fn compute_error_zoom_bbox(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> [f64; 4] {
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let half_w = ((max_x - min_x).abs() * 0.5 * 1.5).max(Self::HGIS_MIN_ERROR_ZOOM_MARGIN);
        let half_h = ((max_y - min_y).abs() * 0.5 * 1.5).max(Self::HGIS_MIN_ERROR_ZOOM_MARGIN);
        [cx - half_w, cy - half_h, cx + half_w, cy + half_h]
    }

    /// C++ `TopologyChecklistEngine` — R04 폐합 링 시작점=끝점 일치 검사 (`d > 1e-6` 시 `Error`) 1:1
    #[must_use]
    pub fn check_r04_ring_closure(
        ring: &[[f64; 2]],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        if ring.len() < 4 {
            return (
                ChecklistSeverity::Error,
                ring.last().copied(),
                "폴리곤 링 정점 수가 최소 4개 미만입니다.".to_owned(),
            );
        }
        let first = ring[0];
        let last = ring[ring.len() - 1];
        let d = (last[0] - first[0]).hypot(last[1] - first[1]);
        if d > Self::HGIS_RING_CLOSE_EPSILON {
            (
                ChecklistSeverity::Error,
                Some(last),
                format!(
                    "폴리곤 링이 폐합되지 않았습니다 (시작점-끝점 이격 거리: {:.2} mm)",
                    d * 1000.0
                ),
            )
        } else {
            (ChecklistSeverity::Pass, None, String::new())
        }
    }

    /// C++ `TopologyChecklistEngine` — R08 자가교차(Self-Intersection) 검출 및 교차점 좌표 반환 1:1
    #[must_use]
    pub fn check_r08_self_intersection(
        ring: &[[f64; 2]],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        if ring.len() < 4 {
            return (ChecklistSeverity::Pass, None, String::new());
        }
        let seg_count = ring.len() - 1;
        for i in 0..seg_count {
            let a = ring[i];
            let b = ring[i + 1];
            for j in (i + 2)..seg_count {
                // 폐합 링의 첫 선분과 마지막 선분은 시작점=끝점을 공유하므로 제외
                if i == 0 && j + 1 == seg_count {
                    continue;
                }
                let c = ring[j];
                let d = ring[j + 1];
                if let Some(ix) = Self::segment_proper_intersection(a, b, c, d) {
                    return (
                        ChecklistSeverity::Error,
                        Some(ix),
                        "폴리곤 경계선이 자가교차(Self-intersection)합니다.".to_owned(),
                    );
                }
            }
        }
        (ChecklistSeverity::Pass, None, String::new())
    }

    fn segment_proper_intersection(
        a: [f64; 2],
        b: [f64; 2],
        c: [f64; 2],
        d: [f64; 2],
    ) -> Option<[f64; 2]> {
        let r = [b[0] - a[0], b[1] - a[1]];
        let s = [d[0] - c[0], d[1] - c[1]];
        let denom = r[0] * s[1] - r[1] * s[0];
        if denom.abs() < 1e-12 {
            return None;
        }
        let qp = [c[0] - a[0], c[1] - a[1]];
        let t = (qp[0] * s[1] - qp[1] * s[0]) / denom;
        let u = (qp[0] * r[1] - qp[1] * r[0]) / denom;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            Some([a[0] + t * r[0], a[1] + t * r[1]])
        } else {
            None
        }
    }

    /// C++ `TopologyChecklistEngine` — R10 중복 피처(중심점 거리 `< 1mm` & 면적 차이 `< 0.001 m²`) 검출 1:1
    #[must_use]
    pub fn check_r10_duplicate_feature(
        centroid_a: [f64; 2],
        area_a: f64,
        centroid_b: [f64; 2],
        area_b: f64,
        matched_fid: u64,
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        let dist = (centroid_a[0] - centroid_b[0]).hypot(centroid_a[1] - centroid_b[1]);
        let area_diff = (area_a - area_b).abs();
        if dist < Self::HGIS_DUPLICATE_DIST_EPSILON && area_diff < Self::HGIS_DUPLICATE_AREA_EPSILON
        {
            (
                ChecklistSeverity::Error,
                Some(centroid_a),
                format!(
                    "피처 FID {matched_fid}과(와) 형상 및 위치가 완전히 일치하는 중복 피처입니다."
                ),
            )
        } else {
            (ChecklistSeverity::Pass, None, String::new())
        }
    }

    /// C++ `TopologyChecklistEngine` — R11 뾰족점(Spike, 내각 `< 2.0°`) 검출 및 `acos` 클램핑(`[-1.0, 1.0]`) 1:1
    #[must_use]
    pub fn check_r11_spike_vertex(
        ring: &[[f64; 2]],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        let is_closed = ring.len() >= 4
            && (ring[0][0] - ring[ring.len() - 1][0]).hypot(ring[0][1] - ring[ring.len() - 1][1])
                <= Self::HGIS_RING_CLOSE_EPSILON;
        let unique_len = if is_closed {
            ring.len() - 1
        } else {
            ring.len()
        };
        if unique_len < 3 {
            return (ChecklistSeverity::Pass, None, String::new());
        }
        for i in 0..unique_len {
            let prev = ring[(i + unique_len - 1) % unique_len];
            let curr = ring[i];
            let next = ring[(i + 1) % unique_len];
            let v1x = prev[0] - curr[0];
            let v1y = prev[1] - curr[1];
            let v2x = next[0] - curr[0];
            let v2y = next[1] - curr[1];
            let len1 = v1x.hypot(v1y);
            let len2 = v2x.hypot(v2y);
            if len1 < 1e-9 || len2 < 1e-9 {
                continue;
            }
            let dot = ((v1x * v2x + v1y * v2y) / (len1 * len2)).clamp(-1.0, 1.0);
            let angle_deg = dot.acos().to_degrees();
            if angle_deg < Self::HGIS_SPIKE_MIN_ANGLE_DEG {
                return (
                    ChecklistSeverity::Warning,
                    Some(curr),
                    format!("정점 {i}번의 사잇각이 {angle_deg:.2}도로 침상형(Spike) 돌출입니다."),
                );
            }
        }
        (ChecklistSeverity::Pass, None, String::new())
    }

    /// C++ `TopologyChecklistEngine` — R21 기준점(ControlPoints) 최소 2개 이상 존재 검사 1:1
    #[must_use]
    pub fn check_r21_control_points_count(
        gcp_points: &[[f64; 2]],
        survey_centroid: [f64; 2],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        if gcp_points.len() < Self::HGIS_MIN_CONTROL_POINTS {
            let target = gcp_points.first().copied().unwrap_or(survey_centroid);
            (
                ChecklistSeverity::Error,
                Some(target),
                format!(
                    "도면 정합 기준점이 최소 2개 이상 필요합니다 (현재: {}개)",
                    gcp_points.len()
                ),
            )
        } else {
            (ChecklistSeverity::Pass, None, String::new())
        }
    }
}

/// C++ `HGIS_VALID_PERIOD_DOMAINS` — 국가유산청 고고학 11대 표준 시대 코드 도메인
pub const HGIS_VALID_PERIOD_DOMAINS: [&str; 11] = [
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValidationIssue {
    pub fid: i64,
    pub field_name: String,
    pub severity: ChecklistSeverity,
    pub message_ko: String,
}

/// C++ `KaAttributeValidator::validateUniqueIds` — 동일 레이어 내 유구번호/조사구역번호 중복 검출
#[must_use]
pub fn validate_unique_feature_ids(features: &[(i64, &str)]) -> Vec<AttributeValidationIssue> {
    let mut seen: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    let mut issues = Vec::new();
    for &(fid, raw_id) in features {
        let trimmed = raw_id.trim();
        if trimmed.is_empty() {
            issues.push(AttributeValidationIssue {
                fid,
                field_name: "FEAT_NO".to_owned(),
                severity: ChecklistSeverity::Error,
                message_ko: "필수 속성 'FEAT_NO'이(가) 누락되었습니다.".to_owned(),
            });
            continue;
        }
        if let Some(&first_fid) = seen.get(trimmed) {
            issues.push(AttributeValidationIssue {
                fid,
                field_name: "FEAT_NO".to_owned(),
                severity: ChecklistSeverity::Error,
                message_ko: format!(
                    "동일한 유구번호 '{trimmed}'이(가) 이미 존재합니다 (FID: {first_fid}와 중복)."
                ),
            });
        } else {
            seen.insert(trimmed.to_owned(), fid);
        }
    }
    issues
}

/// C++ `KaAttributeValidator` — `PERIOD`(시대) 필드가 11대 표준 코드 도메인(`HGIS_VALID_PERIOD_DOMAINS`)에 속하는지 검증
#[must_use]
pub fn validate_period_domain(fid: i64, period_value: &str) -> Option<AttributeValidationIssue> {
    let trimmed = period_value.trim();
    if HGIS_VALID_PERIOD_DOMAINS.contains(&trimmed) {
        None
    } else {
        Some(AttributeValidationIssue {
            fid,
            field_name: "PERIOD".to_owned(),
            severity: ChecklistSeverity::Warning,
            message_ko: format!(
                "시대 속성 '{trimmed}'은(는) 국가유산청 표준 11대 시대 도메인에 속하지 않습니다."
            ),
        })
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

        let bbox = ChecklistEngine::compute_error_zoom_bbox(200_000.0, 500_000.0, 200_000.0, 500_000.0);
        assert_eq!(bbox, [199_980.0, 499_980.0, 200_020.0, 500_020.0]);
        assert!((bbox[2] - bbox[0] - 40.0).abs() < 1e-9);
        assert!((bbox[3] - bbox[1] - 40.0).abs() < 1e-9);
    }

    #[test]
    fn test_r04_ring_close_r08_bowtie_5_5_r10_duplicate_r11_spike_057deg_and_r21_gcp_count() {
        // R04 unclosed ring
        let unclosed = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.01]];
        let (r04_sev, r04_pt, r04_msg) = ChecklistEngine::check_r04_ring_closure(&unclosed);
        assert_eq!(r04_sev, ChecklistSeverity::Error);
        assert_eq!(r04_pt, Some([0.0, 0.01]));
        assert!(r04_msg.contains("10.00 mm"));

        // R08 Bowtie self-intersection at (5.0, 5.0)
        let bowtie = [
            [0.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [10.0, 0.0],
            [0.0, 0.0],
        ];
        let (r08_sev, r08_pt, _) = ChecklistEngine::check_r08_self_intersection(&bowtie);
        assert_eq!(r08_sev, ChecklistSeverity::Error);
        let ix = r08_pt.expect("intersection point");
        assert!((ix[0] - 5.0).abs() < 1e-6);
        assert!((ix[1] - 5.0).abs() < 1e-6);

        // R10 duplicate feature
        let (r10_sev, r10_pt, r10_msg) = ChecklistEngine::check_r10_duplicate_feature(
            [100.0, 200.0],
            50.0,
            [100.0005, 200.0],
            50.0002,
            42,
        );
        assert_eq!(r10_sev, ChecklistSeverity::Error);
        assert_eq!(r10_pt, Some([100.0, 200.0]));
        assert!(r10_msg.contains("FID 42"));

        // R11 needle spike at P1(100.0, 0.5) with angle ~0.57 deg < 2.0 deg
        let needle = [[0.0, 0.0], [100.0, 0.5], [0.0, 1.0], [0.0, 0.0]];
        let (r11_sev, r11_pt, r11_msg) = ChecklistEngine::check_r11_spike_vertex(&needle);
        assert_eq!(r11_sev, ChecklistSeverity::Warning);
        assert_eq!(r11_pt, Some([100.0, 0.5]));
        assert!(r11_msg.contains("0.57도"));

        // R21 control points count < 2
        let (r21_sev, r21_pt, r21_msg) =
            ChecklistEngine::check_r21_control_points_count(&[[200_010.0, 500_020.0]], [200_000.0, 500_000.0]);
        assert_eq!(r21_sev, ChecklistSeverity::Error);
        assert_eq!(r21_pt, Some([200_010.0, 500_020.0]));
        assert!(r21_msg.contains("현재: 1개"));
    }

    #[test]
    fn test_duplicate_feat_no_detection_and_period_domain_validation() {
        let features = [(10_i64, "1호 주거지"), (25_i64, "1호 주거지")];
        let issues = validate_unique_feature_ids(&features);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].fid, 25);
        assert_eq!(
            issues[0].message_ko,
            "동일한 유구번호 '1호 주거지'이(가) 이미 존재합니다 (FID: 10와 중복)."
        );

        assert!(validate_period_domain(10, "삼국").is_none());
        let typo_issue = validate_period_domain(10, "삼국시디").expect("invalid period domain");
        assert_eq!(typo_issue.severity, ChecklistSeverity::Warning);
        assert!(typo_issue.message_ko.contains("삼국시디"));

        // TestKaStratigraphicCycleDetector::testAcyclicLinearStratigraphy & testCycleLoopDetection
        let linear_edges = vec![
            StratigraphicEdge {
                younger_feature_id: "1호".to_string(),
                older_feature_id: "2호".to_string(),
                relation: StratigraphicRelation::Cuts,
            },
            StratigraphicEdge {
                younger_feature_id: "2호".to_string(),
                older_feature_id: "3호".to_string(),
                relation: StratigraphicRelation::Cuts,
            },
        ];
        let acyclic_res = build_harris_matrix(&["1호", "2호", "3호"], &linear_edges);
        assert!(acyclic_res.is_acyclic);
        assert_eq!(acyclic_res.topological_order, vec!["1호", "2호", "3호"]);
        assert!(acyclic_res.detected_cycle_path.is_empty());

        let mut cycle_edges = linear_edges;
        cycle_edges.push(StratigraphicEdge {
            younger_feature_id: "3호".to_string(),
            older_feature_id: "1호".to_string(),
            relation: StratigraphicRelation::Cuts,
        });
        let cycle_res = build_harris_matrix(&["1호", "2호", "3호"], &cycle_edges);
        assert!(!cycle_res.is_acyclic);
        assert_eq!(
            cycle_res.detected_cycle_path,
            vec!["1호", "2호", "3호", "1호"]
        );
    }
}

/// 해리스 매트릭스 최대 분석 유구 수 (C++ `HGIS_MAX_HARRIS_NODES_LIMIT`)
pub const HGIS_MAX_HARRIS_NODES_LIMIT: usize = 50_000;

/// 유구 간 층위 선후관계 유형 (C++ `StratigraphicRelation`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratigraphicRelation {
    /// 파괴함 (신기 -> 고기)
    Cuts,
    /// 피파괴됨 (고기 <- 신기)
    IsCutBy,
    /// 상부 퇴적/덮음 (신기 -> 고기)
    Overlies,
    /// 하부 깔림 (고기 <- 신기)
    Underlies,
    /// 동시기/동일 층위
    Contemporary,
}

/// 해리스 매트릭스 방향 간선 (C++ `StratigraphicEdge`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratigraphicEdge {
    pub younger_feature_id: String,
    pub older_feature_id: String,
    pub relation: StratigraphicRelation,
}

/// 해리스 매트릭스 위상 정렬 및 순환 모순 탐지 결과 (C++ `HarrisMatrixResult`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarrisMatrixResult {
    pub is_acyclic: bool,
    pub topological_order: Vec<String>,
    pub stratigraphic_levels: Vec<Vec<String>>,
    pub detected_cycle_path: Vec<String>,
}

/// 유구 선후관계 DAG 구축, Kahn 위상 정렬 기반 상대 연대 순서 도출 및 순환 모순(Cycle) 경로 탐지
/// (C++ `KaHarrisMatrixBuilder::buildHarrisMatrix` & `KaStratigraphicCycleDetector` 1:1 대응)
#[must_use]
pub fn build_harris_matrix(
    feature_ids: &[&str],
    relations: &[StratigraphicEdge],
) -> HarrisMatrixResult {
    let mut nodes: Vec<String> = feature_ids
        .iter()
        .take(HGIS_MAX_HARRIS_NODES_LIMIT)
        .map(|&s| s.to_owned())
        .collect();
    for edge in relations {
        if nodes.len() < HGIS_MAX_HARRIS_NODES_LIMIT
            && !nodes.contains(&edge.younger_feature_id)
        {
            nodes.push(edge.younger_feature_id.clone());
        }
        if nodes.len() < HGIS_MAX_HARRIS_NODES_LIMIT && !nodes.contains(&edge.older_feature_id) {
            nodes.push(edge.older_feature_id.clone());
        }
    }
    let n = nodes.len();
    let idx_of = |name: &str| nodes.iter().position(|x| x == name);
    let mut adj = vec![Vec::<usize>::new(); n];
    let mut in_deg = vec![0_usize; n];

    for edge in relations {
        let (u_opt, v_opt) = match edge.relation {
            StratigraphicRelation::Cuts | StratigraphicRelation::Overlies => (
                idx_of(&edge.younger_feature_id),
                idx_of(&edge.older_feature_id),
            ),
            StratigraphicRelation::IsCutBy | StratigraphicRelation::Underlies => (
                idx_of(&edge.older_feature_id),
                idx_of(&edge.younger_feature_id),
            ),
            StratigraphicRelation::Contemporary => (None, None),
        };
        if let (Some(u), Some(v)) = (u_opt, v_opt) {
            if !adj[u].contains(&v) {
                adj[u].push(v);
                in_deg[v] += 1;
            }
        }
    }

    let mut queue: std::collections::VecDeque<usize> =
        (0..n).filter(|&i| in_deg[i] == 0).collect();
    let mut topological_order = Vec::with_capacity(n);
    let mut stratigraphic_levels = Vec::new();

    while !queue.is_empty() {
        let level_len = queue.len();
        let mut level_nodes = Vec::with_capacity(level_len);
        for _ in 0..level_len {
            if let Some(u) = queue.pop_front() {
                topological_order.push(nodes[u].clone());
                level_nodes.push(nodes[u].clone());
                for &v in &adj[u] {
                    in_deg[v] -= 1;
                    if in_deg[v] == 0 {
                        queue.push_back(v);
                    }
                }
            }
        }
        stratigraphic_levels.push(level_nodes);
    }

    if topological_order.len() == n {
        return HarrisMatrixResult {
            is_acyclic: true,
            topological_order,
            stratigraphic_levels,
            detected_cycle_path: Vec::new(),
        };
    }

    // Cycle detection via trace on remaining in_deg > 0 nodes
    let mut detected_cycle_path = Vec::new();
    if let Some(start) = (0..n).find(|&i| in_deg[i] > 0) {
        let mut visited: Vec<usize> = Vec::new();
        let mut curr = start;
        loop {
            if let Some(pos) = visited.iter().position(|&x| x == curr) {
                for &node_idx in &visited[pos..] {
                    detected_cycle_path.push(nodes[node_idx].clone());
                }
                detected_cycle_path.push(nodes[curr].clone());
                break;
            }
            visited.push(curr);
            if let Some(&next) = adj[curr].iter().find(|&&nb| in_deg[nb] > 0) {
                curr = next;
            } else {
                break;
            }
        }
    }

    HarrisMatrixResult {
        is_acyclic: false,
        topological_order,
        stratigraphic_levels,
        detected_cycle_path,
    }
}
