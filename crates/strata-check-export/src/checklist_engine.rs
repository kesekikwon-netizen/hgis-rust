use strata_layout::sheet_composer::DrawingSheet;
use strata_types::crs::SubmitCrs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistFinding {
    pub rule_id: String,
    pub message_ko: String,
    pub passed: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ChecklistEngine;

impl ChecklistEngine {
    #[must_use]
    pub fn evaluate_sheet(
        sheet: &DrawingSheet,
        submit_crs: SubmitCrs,
        has_site_boundary: bool,
    ) -> Vec<ChecklistFinding> {
        vec![
            ChecklistFinding {
                rule_id: "R-SUBMIT-CRS-5179".to_owned(),
                message_ko: format!("제출 좌표계 {} 확인", submit_crs.auth_id()),
                passed: submit_crs == SubmitCrs::Epsg5179,
            },
            ChecklistFinding {
                rule_id: "R-SITE-BOUNDARY".to_owned(),
                message_ko: "사업대상지 경계 폴리곤 존재 여부".to_owned(),
                passed: has_site_boundary,
            },
            ChecklistFinding {
                rule_id: "R-SHEET-BOUNDS".to_owned(),
                message_ko: "도면 장식 요소(축척막대·방위표·좌표계) 용지 내부 배치 여부".to_owned(),
                passed: sheet.all_decorations_inside_sheet(),
            },
        ]
    }
}
