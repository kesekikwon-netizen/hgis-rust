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

    pub const HGIS_DPI_DRAFT: i32 = 150;
    pub const HGIS_DPI_STANDARD: i32 = 300;
    pub const HGIS_DPI_HIGH: i32 = 600;

    /// C++ `KaDrawingStudio::validateExportDpi` — 150 / 300 / 600 DPI 표준 규격 검증 및 클램핑 1:1
    #[must_use]
    pub const fn validate_export_dpi(requested_dpi: i32) -> i32 {
        if requested_dpi <= 175 {
            Self::HGIS_DPI_DRAFT
        } else if requested_dpi <= 450 {
            Self::HGIS_DPI_STANDARD
        } else {
            Self::HGIS_DPI_HIGH
        }
    }

    /// C++ `KaDrawingStudio::syncMapExtentToFrame(preserveScale)` — 축척 고정 잠금 시 프레임 크기 기준으로 지상 BBOX 재계산 1:1
    #[must_use]
    pub fn sync_map_extent_to_frame(
        center_xy: [f64; 2],
        current_scale_denom: f64,
        frame_w_mm: f64,
        frame_h_mm: f64,
        preserve_scale: bool,
    ) -> ([f64; 4], f64) {
        let scale = if preserve_scale && current_scale_denom > 0.0 {
            current_scale_denom
        } else {
            1_000.0
        };
        let half_w_m = scale * (frame_w_mm.max(1.0) / 1000.0) * 0.5;
        let half_h_m = scale * (frame_h_mm.max(1.0) / 1000.0) * 0.5;
        (
            [
                center_xy[0] - half_w_m,
                center_xy[1] - half_h_m,
                center_xy[0] + half_w_m,
                center_xy[1] + half_h_m,
            ],
            scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawing_studio_dpi_validation_72_200_1200_and_preserve_scale_sync() {
        assert_eq!(DrawingTabState::validate_export_dpi(72), 150);
        assert_eq!(DrawingTabState::validate_export_dpi(200), 300);
        assert_eq!(DrawingTabState::validate_export_dpi(1200), 600);

        let (bbox, scale) = DrawingTabState::sync_map_extent_to_frame(
            [200_000.0, 500_000.0],
            2_500.0,
            200.0,
            100.0,
            true,
        );
        assert!((scale - 2_500.0).abs() < 1e-6);
        assert!((bbox[2] - bbox[0] - 500.0).abs() < 1e-6);
        assert!((bbox[3] - bbox[1] - 250.0).abs() < 1e-6);
    }
}
