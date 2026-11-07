use std::path::PathBuf;
use strata_check_export::checklist_engine::{ChecklistEngine, ChecklistFinding};
use strata_layout::pdf_geotiff::render_sheet_pdf_bytes;
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
        let sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        let last_findings = ChecklistEngine::evaluate_sheet(&sheet, SubmitCrs::Epsg5179, true);
        Self {
            title_ko: "유적 지표·시굴조사 위치 및 트렌치 배치도".to_owned(),
            sheet,
            last_findings,
        }
    }
}

impl DrawingTabState {
    pub fn set_paper_and_orientation(&mut self, size: SheetPaperSize, ori: SheetOrientation) {
        self.sheet.change_paper(size, ori);
        self.last_findings = ChecklistEngine::evaluate_sheet(&self.sheet, SubmitCrs::Epsg5179, true);
    }

    pub fn export_pdf_to_temp(&self) -> std::io::Result<PathBuf> {
        let bytes = render_sheet_pdf_bytes(&self.sheet, &self.title_ko)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let out = std::env::temp_dir().join("조사도면.pdf");
        std::fs::write(&out, bytes)?;
        Ok(out)
    }
}
