//! `sheet_drawing` tests — field-sheet dimensions, legacy A4 landscape migration, and custom mm sizing
//! (`target/cpp-ref/src/app/KaDrawingStudio.h:58-70`, `target/cpp-ref/src/app/KaDrawingStudio.cpp:432-554, 1350-1356`,
//! `target/cpp-ref/src/app/KaPaperDialog.cpp:162-168`; expected values ported from
//! `target/cpp-ref/tests/test_save_open.cpp:2060-2124` and `target/cpp-ref/tests/test_drawing_polish.cpp:125-158`).

use crate::sheet_geometry::{SheetOrientation, SheetPaperSize};

use super::DrawingSheet;

#[test]
fn a4_landscape_is_317_by_220_field_sheet_with_10mm_top_side_margins_and_38mm_bottom_chrome() {
    // Ported from target/cpp-ref/src/app/KaPaperDialog.cpp:162-168,
    // target/cpp-ref/src/app/KaDrawingStudio.h:58-69, KaDrawingStudio.cpp:1350-1356,
    // and target/cpp-ref/tests/test_save_open.cpp:2061-2062, 2089-2094.
    let a4_landscape = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Landscape);
    assert_eq!((a4_landscape.width_mm, a4_landscape.height_mm), (317.0, 220.0));
    assert_eq!(DrawingSheet::FIELD_PAPER_WIDTH_MM, 317.0);
    assert_eq!(DrawingSheet::FIELD_PAPER_HEIGHT_MM, 220.0);

    let a4_portrait = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    assert_eq!((a4_portrait.width_mm, a4_portrait.height_mm), (210.0, 297.0));
    assert!((a4_portrait.map_frame.x_mm - 10.0).abs() < 1e-9);
    assert!((a4_portrait.map_frame.y_mm - 10.0).abs() < 1e-9);
    assert!((a4_portrait.map_frame.width_mm - 190.0).abs() < 1e-9);
    assert!((a4_portrait.map_frame.height_mm - 249.0).abs() < 1e-9);

    assert!((a4_landscape.map_frame.x_mm - 10.0).abs() < 1e-9);
    assert!((a4_landscape.map_frame.y_mm - 10.0).abs() < 1e-9);
    assert!((a4_landscape.map_frame.width_mm - 297.0).abs() < 1e-9);
    assert!((a4_landscape.map_frame.height_mm - 172.0).abs() < 1e-9);
    assert!(a4_landscape.all_decorations_inside_sheet());

    let a3_portrait = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Portrait);
    assert_eq!((a3_portrait.width_mm, a3_portrait.height_mm), (297.0, 420.0));
    let a3_landscape = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    assert_eq!((a3_landscape.width_mm, a3_landscape.height_mm), (420.0, 297.0));
}

#[test]
fn legacy_a4_landscape_detection_and_field_page_grow_and_reopen_bounds() {
    // Ported 1:1 from target/cpp-ref/tests/test_save_open.cpp:2060-2124
    // (drawingStudio_fieldPageGrowsA4LandscapeByOneCentimetre).
    assert!(DrawingSheet::is_legacy_a4_landscape_mm(297.0, 210.0));
    assert!(!DrawingSheet::is_legacy_a4_landscape_mm(317.0, 220.0));
    assert!(!DrawingSheet::is_legacy_a4_landscape_mm(210.0, 297.0));

    let mut studio = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    studio.reset_paper_mm(297.0, 210.0);
    assert_eq!((studio.width_mm, studio.height_mm), (297.0, 210.0));

    studio.apply_field_page_grow();
    assert_eq!((studio.width_mm, studio.height_mm), (317.0, 220.0));
    assert!((studio.map_frame.width_mm - 297.0).abs() < 1.0);
    assert!((studio.map_frame.height_mm - 172.0).abs() < 1.0);
    assert!((studio.map_frame.x_mm - 10.0).abs() < 1.0);

    // Second applyFieldPageGrow is idempotent (test_save_open.cpp:2095-2098).
    studio.apply_field_page_grow();
    assert_eq!((studio.width_mm, studio.height_mm), (317.0, 220.0));

    // Reopen / shrink from 297x420 to 210x297 keeps all decorations inside A4 page
    // (-1..=211 x -1..=298), then 297x210 + apply_field_page_grow -> 317x220 (lines 2102-2123).
    let mut reopen = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    reopen.reset_paper_mm(297.0, 210.0);
    reopen.set_legend_kept_on_page(true);
    reopen.reset_paper_mm(297.0, 420.0);
    reopen.reset_paper_mm(210.0, 297.0);
    for r in [
        reopen.scale_bar_rect,
        reopen.scale_text_rect,
        reopen.crs_label_rect,
        reopen.north_arrow_rect,
        reopen.legend_rect,
    ] {
        assert!(
            r.left() >= -1.0 && r.right() <= 211.0 && r.top() >= -1.0 && r.bottom() <= 298.0,
            "item outside A4 page: {r:?}"
        );
    }
    reopen.reset_paper_mm(297.0, 210.0);
    reopen.apply_field_page_grow();
    assert_eq!((reopen.width_mm, reopen.height_mm), (317.0, 220.0));
}

#[test]
fn custom_paper_mm_updates_map_frame_chrome_and_clamps_orphaned_items() {
    // Ported from target/cpp-ref/tests/test_drawing_polish.cpp:153-157 (123.4 x 297.0)
    // and target/cpp-ref/src/app/KaDrawingStudio.cpp:456-554.
    let mut sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    sheet.legend_rect.x_mm = 360.0;
    sheet.legend_rect.y_mm = 200.0;

    sheet.reset_paper_mm(123.4, 297.0);
    assert!((sheet.width_mm - 123.4).abs() < 1e-9);
    assert!((sheet.height_mm - 297.0).abs() < 1e-9);
    assert!((sheet.map_frame.x_mm - 10.0).abs() < 1e-9);
    assert!((sheet.map_frame.y_mm - 10.0).abs() < 1e-9);
    assert!((sheet.map_frame.width_mm - 103.4).abs() < 1e-9);
    assert!((sheet.map_frame.height_mm - 249.0).abs() < 1e-9);
    assert!(sheet.all_decorations_inside_sheet());
}
