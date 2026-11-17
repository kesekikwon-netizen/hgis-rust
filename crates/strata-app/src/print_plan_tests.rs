//! Exact C++ page counts/coordinates, target/cpp-ref/tests/test_tile_print.cpp:65-115.
use super::*;

fn a3() -> DevicePaper {
    DevicePaper { name: "A3".to_owned(), width_mm: 297.0, height_mm: 420.0, portrait: [5.0, 5.0, 287.0, 410.0], landscape: [5.0, 5.0, 410.0, 287.0] }
}

#[test]
fn a0_drawing_needs_nine_a3_sheets_with_real_overlap() {
    let plan = build_plan([841.0, 1189.0], &a3(), PrintMode::Tiled { overlap_mm: 10.0, marks: true, overview: true }).expect("valid recorded paper");
    assert!(!plan.landscape);
    assert_eq!((plan.rows, plan.cols, plan.pages.len()), (3, 3, 9));
    assert_eq!(plan.pages[0].source_mm, [0.0, 0.0, 287.0, 404.0]);
    assert_eq!(plan.pages[1].source_mm[0], 277.0);
    assert_eq!(plan.pages[3].source_mm[1], 394.0);
    let last = &plan.pages[8];
    assert!(last.source_mm[0] + last.source_mm[2] >= 841.0);
    assert!(last.source_mm[1] + last.source_mm[3] >= 1189.0);
}

#[test]
fn small_drawing_is_one_real_page_and_ties_choose_portrait() {
    let plan = build_plan([210.0, 297.0], &a3(), PrintMode::Tiled { overlap_mm: 10.0, marks: true, overview: false }).expect("valid recorded paper");
    assert_eq!(plan.pages.len(), 1);
    assert!(!plan.landscape);
}

#[test]
fn single_sheet_keeps_actual_size_when_paper_fits_and_reports_shrink_otherwise() {
    let same = build_plan([297.0, 420.0], &a3(), PrintMode::Single).expect("same paper");
    assert_eq!(same.ratio, 1.0);
    assert_eq!(same.pages[0].target_mm, [0.0, 0.0, 297.0, 420.0]);
    let fit = build_plan([594.0, 840.0], &a3(), PrintMode::Single).expect("fits with reduction");
    // Exact rational ratios of recorded physical mm, tolerance only for f64 arithmetic.
    assert!((fit.ratio - (287.0_f64 / 594.0).min(410.0 / 840.0)).abs() < 1e-12);
    assert_eq!(fit.pages.len(), 1);
    assert!(!fit.overview && !fit.marks);
}

#[test]
fn overlap_wider_than_content_and_invalid_dimensions_are_rejected() {
    assert!(build_plan([841.0, 1189.0], &a3(), PrintMode::Tiled { overlap_mm: 500.0, marks: true, overview: true }).is_err());
    assert!(build_plan([f64::NAN, 100.0], &a3(), PrintMode::Single).is_err());
}
