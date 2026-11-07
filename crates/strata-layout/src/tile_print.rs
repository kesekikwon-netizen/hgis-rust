use strata_types::error::{StrataError, StrataResult};

use crate::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};

#[derive(Debug, Clone, PartialEq)]
pub struct TilePrintPlan {
    pub rows: u32,
    pub cols: u32,
    pub overlap_mm: f64,
}

pub fn verify_printer_paper_match(
    sheet: &DrawingSheet,
    printer_paper: SheetPaperSize,
    printer_orientation: SheetOrientation,
) -> StrataResult<()> {
    if sheet.paper_size != printer_paper || sheet.orientation != printer_orientation {
        return Err(StrataError::Export(format!(
            "프린터 용지({printer_paper:?}/{printer_orientation:?})가 도면 용지({:?}/{:?})와 일치하지 않습니다",
            sheet.paper_size, sheet.orientation
        )));
    }
    Ok(())
}

#[must_use]
pub fn compute_tile_print_plan(sheet: &DrawingSheet, printer_paper: SheetPaperSize) -> TilePrintPlan {
    let (pw, ph) = printer_paper.portrait_dimensions_mm();
    let overlap_mm = 10.0;
    let eff_w = (pw - overlap_mm).max(50.0);
    let eff_h = (ph - overlap_mm).max(50.0);
    let cols = (sheet.width_mm / eff_w).ceil().max(1.0) as u32;
    let rows = (sheet.height_mm / eff_h).ceil().max(1.0) as u32;
    TilePrintPlan {
        rows,
        cols,
        overlap_mm,
    }
}
