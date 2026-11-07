use strata_check_export::checklist_engine::{ChecklistEngine, ChecklistFinding};
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

#[derive(Debug, Clone)]
pub struct DrawingTabState {
    pub title_ko: String,
    pub sheet: DrawingSheet,
    pub last_findings: Vec<ChecklistFinding>,
}

impl Default for DrawingTabState {
    fn default() -> Self {
        let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
        let last_findings = ChecklistEngine::evaluate_sheet(&sheet, SubmitCrs::Epsg5179, true);
        Self {
            title_ko: "유적 지표·시굴조사 위치 및 트렌치 배치도".to_owned(),
            sheet,
            last_findings,
        }
    }
}
