use crate::sheet_composer::{SheetOrientation, SheetPaperSize};
use crate::tile_print::*;

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
