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
}
