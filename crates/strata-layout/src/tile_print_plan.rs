//! 인쇄 장치 용지 검사와 타일 인쇄 계획·재단 표시.
//!
//! `tile_print` 에서 분리했다.

use strata_types::error::{StrataError, StrataResult};

use crate::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};

pub const HGIS_PRINT_OVERLAP_DEFAULT_MM: f64 = 10.0;

pub const HGIS_CROP_MARK_LENGTH_MM: f64 = 5.0;

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
pub fn compute_tile_print_plan(
    sheet: &DrawingSheet,
    printer_paper: SheetPaperSize,
) -> TilePrintPlan {
    compute_tile_print_plan_with_overlap(sheet, printer_paper, HGIS_PRINT_OVERLAP_DEFAULT_MM)
}

/// `KaPrintDialog.cpp:131-134` — 같은 분할 계산에 「겹침」 칸에서 고른 폭(`0..=30 mm`)을 쓴다
#[must_use]
pub fn compute_tile_print_plan_with_overlap(
    sheet: &DrawingSheet,
    printer_paper: SheetPaperSize,
    overlap_mm: f64,
) -> TilePrintPlan {
    let (pw, ph) = printer_paper.portrait_dimensions_mm();
    let overlap_mm = overlap_mm.clamp(0.0, 30.0);
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

/// C++ `KaTilePrintEngine::calculateTileGrid` — 인쇄 가능 영역(`printable_w`, `printable_h`) 및 여백 50% 상한 클램핑 가드(`overlap_mm <= 0.5 * min(pw, ph)`)를 포함한 행렬 분할 계산 1:1
#[must_use]
pub fn compute_tile_print_grid_from_printable_mm(
    sheet_width_mm: f64,
    sheet_height_mm: f64,
    printable_width_mm: f64,
    printable_height_mm: f64,
    overlap_mm: f64,
) -> TilePrintPlan {
    let pw = printable_width_mm.max(20.0);
    let ph = printable_height_mm.max(20.0);
    let max_overlap = 0.5 * pw.min(ph);
    let safe_overlap = overlap_mm.clamp(0.0, max_overlap);
    let step_w = (pw - safe_overlap).max(pw * 0.5);
    let step_h = (ph - safe_overlap).max(ph * 0.5);
    let cols = if sheet_width_mm <= pw {
        1
    } else {
        1 + ((sheet_width_mm - pw) / step_w).ceil().max(0.0) as u32
    };
    let rows = if sheet_height_mm <= ph {
        1
    } else {
        1 + ((sheet_height_mm - ph) / step_h).ceil().max(0.0) as u32
    };
    TilePrintPlan {
        rows,
        cols,
        overlap_mm: safe_overlap,
    }
}

/// C++ `KaTilePrintEngine::renderCropMarks` — 분할 페이지 4개 모서리에 5.0mm 재단선 십자 마크(총 8개 선분) 생성
#[must_use]
pub fn generate_tile_crop_marks_mm(tile_rect_mm: [f64; 4]) -> [[[f64; 2]; 2]; 8] {
    let [x0, y0, x1, y1] = tile_rect_mm;
    let l = HGIS_CROP_MARK_LENGTH_MM;
    [
        [[x0 - l, y0], [x0 + l, y0]],
        [[x0, y0 - l], [x0, y0 + l]],
        [[x1 - l, y0], [x1 + l, y0]],
        [[x1, y0 - l], [x1, y0 + l]],
        [[x0 - l, y1], [x0 + l, y1]],
        [[x0, y1 - l], [x0, y1 + l]],
        [[x1 - l, y1], [x1 + l, y1]],
        [[x1, y1 - l], [x1, y1 + l]],
    ]
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
