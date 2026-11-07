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

pub const HGIS_DEFAULT_ATLAS_OVERLAP_PCT: f64 = 15.0;
pub const HGIS_MATCH_LINE_PEN_WIDTH_MM: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct AtlasConfig {
    pub scale_denominator: f64,
    pub paper_size_mm: [f64; 2],
    pub overlap_percent: f64,
    pub custom_grid_angle_deg: f64,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            scale_denominator: 500.0,
            paper_size_mm: [420.0, 297.0],
            overlap_percent: HGIS_DEFAULT_ATLAS_OVERLAP_PCT,
            custom_grid_angle_deg: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtlasSheetTile {
    pub page_number: usize,
    pub sheet_name: String,
    pub center_xy: [f64; 2],
    pub tile_extent: [f64; 4],
    pub left_sheet_no: usize,
    pub right_sheet_no: usize,
    pub top_sheet_no: usize,
    pub bottom_sheet_no: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasTileError {
    EmptyLinearRoute,
}

/// C++ `KaAtlasSheetGenerator::generateLinearAtlasTiles` — 선형 유적 노선 중심선을 따라 `D_step = W_ground * (1.0 - overlap_pct / 100.0)` 간격으로 아틀라스 도엽 생성 및 인접 도엽 번호 부여
pub fn generate_linear_atlas_tiles(
    linear_route: &[[f64; 2]],
    config: &AtlasConfig,
) -> Result<Vec<AtlasSheetTile>, AtlasTileError> {
    if linear_route.len() < 2 {
        return Err(AtlasTileError::EmptyLinearRoute);
    }
    let mut seg_lengths = Vec::with_capacity(linear_route.len() - 1);
    let mut total_len = 0.0;
    for pair in linear_route.windows(2) {
        let len = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
        seg_lengths.push(len);
        total_len += len;
    }
    if total_len <= 1e-9 {
        return Err(AtlasTileError::EmptyLinearRoute);
    }

    let scale = config.scale_denominator.max(1.0);
    let w_ground = (config.paper_size_mm[0].max(10.0) * scale) / 1000.0;
    let h_ground = (config.paper_size_mm[1].max(10.0) * scale) / 1000.0;
    let overlap_ratio = (config.overlap_percent / 100.0).clamp(0.0, 0.8);
    let d_step = (w_ground * (1.0 - overlap_ratio)).max(1.0);

    let interpolate_along = |dist: f64| -> [f64; 2] {
        let clamped = dist.clamp(0.0, total_len);
        let mut acc = 0.0;
        for (idx, &seg_len) in seg_lengths.iter().enumerate() {
            if acc + seg_len >= clamped || idx + 1 == seg_lengths.len() {
                let t = if seg_len > 1e-12 {
                    ((clamped - acc) / seg_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let p0 = linear_route[idx];
                let p1 = linear_route[idx + 1];
                return [p0[0] + t * (p1[0] - p0[0]), p0[1] + t * (p1[1] - p0[1])];
            }
            acc += seg_len;
        }
        linear_route[linear_route.len() - 1]
    };

    let tile_count = ((total_len / d_step).ceil() as usize).max(1);
    let mut tiles = Vec::with_capacity(tile_count);
    for idx in 0..tile_count {
        let page_no = idx + 1;
        let dist = (idx as f64 * d_step).min(total_len);
        let c = interpolate_along(dist);
        let half_w = w_ground * 0.5;
        let half_h = h_ground * 0.5;
        tiles.push(AtlasSheetTile {
            page_number: page_no,
            sheet_name: format!("제{page_no}도엽"),
            center_xy: c,
            tile_extent: [c[0] - half_w, c[1] - half_h, c[0] + half_w, c[1] + half_h],
            left_sheet_no: page_no.saturating_sub(1),
            right_sheet_no: if page_no < tile_count { page_no + 1 } else { 0 },
            top_sheet_no: 0,
            bottom_sheet_no: 0,
        });
    }

    Ok(tiles)
}

/// C++ `KaMatchLineIndicator::renderMatchLineOverlay` — 인접 도엽 번호에 따른 접합선 라벨 생성 (`"제N도엽 접합선"`)
#[must_use]
pub fn format_match_line_label_ko(neighbor_sheet_no: usize) -> Option<String> {
    if neighbor_sheet_no == 0 {
        None
    } else {
        Some(format!("제{neighbor_sheet_no}도엽 접합선"))
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
        assert!(err
            .to_string()
            .contains("프린터가 A3 세로 용지로 바꾸지 않고 A4 세로로 두었습니다"));
        assert_eq!(
            format_print_dispatched_message(SheetPaperSize::A3, SheetOrientation::Portrait, 10),
            "A3 세로 10쪽을 프린터로 보냈습니다"
        );
    }

    #[test]
    fn test_tile_print_grid_calculation_a1_landscape_on_a3_printable_400x277_yields_3x3_9_tiles_and_tolerance(
    ) {
        let plan = compute_tile_print_grid_from_printable_mm(841.0, 594.0, 400.0, 277.0, 10.0);
        assert_eq!(plan.cols, 3);
        assert_eq!(plan.rows, 3);
        assert_eq!(plan.cols * plan.rows, 9);

        assert!(is_printer_page_size_within_tolerance(
            420.0, 297.0, 419.5, 297.0
        ));
        assert!(!is_printer_page_size_within_tolerance(
            841.0, 594.0, 420.0, 297.0
        ));
        assert_eq!(
            format_tile_fallback_prompt_ko(SheetPaperSize::A3, 9),
            "단일 대형 인쇄 불가: 타일 분할 인쇄(A3 9장)로 전환하시겠습니까?"
        );

        // 과도한 겹침 여백(overlap >= printable) 입력 시 50% 상한 클램핑 방어 및 5mm 재단선 검증
        let clamped_plan =
            compute_tile_print_grid_from_printable_mm(841.0, 594.0, 400.0, 277.0, 500.0);
        assert!((clamped_plan.overlap_mm - 138.5).abs() < 1e-6);
        assert!(clamped_plan.cols >= 2 && clamped_plan.rows >= 2);

        let marks = generate_tile_crop_marks_mm([10.0, 10.0, 390.0, 267.0]);
        assert_eq!(marks.len(), 8);
        let mark0_len = (marks[0][1][0] - marks[0][0][0]).hypot(marks[0][1][1] - marks[0][0][1]);
        assert!((mark0_len - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_15_percent_overlap_step_advance_and_neighbor_sheet_numbering() {
        // TestKaAtlasSheetGenerator::test15PercentOverlapStepAdvance
        // W_ground = 200.0 m (400mm * 500 / 1000 = 200m), overlap = 15% -> D_step = 170.0 m
        let cfg = AtlasConfig {
            scale_denominator: 500.0,
            paper_size_mm: [400.0, 277.0],
            overlap_percent: 15.0,
            custom_grid_angle_deg: 0.0,
        };
        let route = [[0.0, 0.0], [450.0, 0.0]];
        let tiles = generate_linear_atlas_tiles(&route, &cfg).expect("valid linear route");
        assert_eq!(tiles.len(), 3);
        let dist_1_2 = (tiles[1].center_xy[0] - tiles[0].center_xy[0])
            .hypot(tiles[1].center_xy[1] - tiles[0].center_xy[1]);
        assert!((dist_1_2 - 170.0).abs() < 1e-6);

        // TestKaMatchLineIndicator::testNeighborSheetNumbering
        assert_eq!(tiles[1].left_sheet_no, 1);
        assert_eq!(tiles[1].right_sheet_no, 3);
        assert_eq!(
            format_match_line_label_ko(tiles[1].right_sheet_no).as_deref(),
            Some("제3도엽 접합선")
        );
        assert_eq!(
            generate_linear_atlas_tiles(&[[10.0, 10.0], [10.0, 10.0]], &cfg),
            Err(AtlasTileError::EmptyLinearRoute)
        );
    }
}
