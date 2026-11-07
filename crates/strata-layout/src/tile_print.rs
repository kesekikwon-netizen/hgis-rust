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

/// C++ `TilePrint::preparePage` 물리 프린터 용지 크기 허용 오차(±2.0mm) 검증 1:1
#[must_use]
pub fn is_printer_page_size_within_tolerance(
    layout_width_mm: f64,
    layout_height_mm: f64,
    printer_width_mm: f64,
    printer_height_mm: f64,
) -> bool {
    const TOLERANCE_MM: f64 = 2.0;
    (layout_width_mm - printer_width_mm).abs() <= TOLERANCE_MM
        && (layout_height_mm - printer_height_mm).abs() <= TOLERANCE_MM
}

/// C++ `TilePrint::renderTiledPages` 인쇄 가능 영역(`printable_w`, `printable_h`) 및 오버랩(`overlap_mm`) 기준 행렬 분할 계산 1:1
#[must_use]
pub fn compute_tile_print_grid_from_printable_mm(
    sheet_width_mm: f64,
    sheet_height_mm: f64,
    printable_width_mm: f64,
    printable_height_mm: f64,
    overlap_mm: f64,
) -> TilePrintPlan {
    let eff_w = (printable_width_mm - overlap_mm).max(50.0);
    let eff_h = (printable_height_mm - overlap_mm).max(50.0);
    let cols = (sheet_width_mm / eff_w).ceil().max(1.0) as u32;
    let rows = (sheet_height_mm / eff_h).ceil().max(1.0) as u32;
    TilePrintPlan {
        rows,
        cols,
        overlap_mm,
    }
}

/// C++ `TilePrint::preparePage` 미지원 대형 규격일 때 타일 분할 전환 안내 문구 1:1
#[must_use]
pub fn format_tile_fallback_prompt_ko(paper: SheetPaperSize, total_tiles: u32) -> String {
    format!(
        "단일 대형 인쇄 불가: 타일 분할 인쇄({} {}장)로 전환하시겠습니까?",
        paper.label_ko(),
        total_tiles.max(1)
    )
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

    #[test]
    fn test_tile_print_grid_calculation_a1_landscape_on_a3_printable_400x277_yields_3x3_9_tiles_and_tolerance()
    {
        let plan = compute_tile_print_grid_from_printable_mm(841.0, 594.0, 400.0, 277.0, 10.0);
        assert_eq!(plan.cols, 3);
        assert_eq!(plan.rows, 3);
        assert_eq!(plan.cols * plan.rows, 9);

        assert!(is_printer_page_size_within_tolerance(420.0, 297.0, 419.5, 297.0));
        assert!(!is_printer_page_size_within_tolerance(841.0, 594.0, 420.0, 297.0));
        assert_eq!(
            format_tile_fallback_prompt_ko(SheetPaperSize::A3, 9),
            "단일 대형 인쇄 불가: 타일 분할 인쇄(A3 9장)로 전환하시겠습니까?"
        );
    }
}
