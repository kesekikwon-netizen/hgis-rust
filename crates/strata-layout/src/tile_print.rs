use strata_types::error::{StrataError, StrataResult};

use crate::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};

#[derive(Debug, Clone, PartialEq)]
pub struct TilePrintPlan {
    pub rows: u32,
    pub cols: u32,
    pub overlap_mm: f64,
}

/// `TilePrint::preparePage` (`e85de5a`) — 프린터가 요청한 용지·방향으로 바뀌지 않으면 인쇄를 멈추고 안내
pub fn verify_printer_paper_match(
    sheet: &DrawingSheet,
    printer_paper: SheetPaperSize,
    printer_orientation: SheetOrientation,
) -> StrataResult<()> {
    verify_requested_device_paper(
        sheet.paper_size,
        sheet.orientation,
        printer_paper,
        printer_orientation,
    )
}

pub fn verify_requested_device_paper(
    requested_paper: SheetPaperSize,
    requested_orientation: SheetOrientation,
    device_paper: SheetPaperSize,
    device_orientation: SheetOrientation,
) -> StrataResult<()> {
    if requested_paper != device_paper || requested_orientation != device_orientation {
        return Err(StrataError::Export(format!(
            "프린터가 {} {} 용지로 바꾸지 않고 {} {}로 두었습니다",
            requested_paper.label_ko(),
            requested_orientation.label_ko(),
            device_paper.label_ko(),
            device_orientation.label_ko()
        )));
    }
    Ok(())
}

/// `KaPrintDialog.cpp` (`e85de5a`) — 인쇄 완료 시 보낸 용지와 방향·쪽수를 알림 (`"A3 세로 10쪽을 프린터로 보냈습니다"`)
#[must_use]
pub fn format_print_dispatched_message(
    paper: SheetPaperSize,
    orientation: SheetOrientation,
    page_count: u32,
) -> String {
    format!(
        "{} {} {}쪽을 프린터로 보냈습니다",
        paper.label_ko(),
        orientation.label_ko(),
        page_count.max(1)
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_stop_when_device_keeps_another_paper_and_report_both_papers() {
        let err = verify_requested_device_paper(
            SheetPaperSize::A3,
            SheetOrientation::Portrait,
            SheetPaperSize::A4,
            SheetOrientation::Portrait,
        )
        .expect_err("must stop when printer keeps A4 instead of A3");
        assert!(err.to_string().contains("프린터가 A3 세로 용지로 바꾸지 않고 A4 세로로 두었습니다"));
        assert_eq!(
            format_print_dispatched_message(SheetPaperSize::A3, SheetOrientation::Portrait, 10),
            "A3 세로 10쪽을 프린터로 보냈습니다"
        );
    }
}
