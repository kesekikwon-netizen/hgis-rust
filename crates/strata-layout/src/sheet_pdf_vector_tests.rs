//! The sheet PDF is a page of text and paths, at the C++ paper sizes.
//!
//! Page boxes: A4 portrait 210×297 mm (`KaDrawingStudio.h:58-59`), the field sheet
//! 317×220 mm (`KaDrawingStudio.h:60-67`), A3 297×420 mm (`KaPaperDialog.cpp:162-164`).
//! 1 mm = 72/25.4 pt. A picture page has no text operators, so it fails here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::picture_pdf::picture_page_pdf;
use crate::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use crate::sheet_pdf_export::mm_to_pdf_points;
use crate::sheet_pdf_vector::{
    extracted_text, render_vector_sheet_pdf, SheetVectorRing, VectorSheetPdf,
};

fn pt(mm: f64) -> String {
    format!("{:.2}", mm_to_pdf_points(mm))
}

fn page(paper: SheetPaperSize, orientation: SheetOrientation) -> DrawingSheet {
    DrawingSheet::new(paper, orientation)
}

fn paint<'a>(
    sheet: &'a DrawingSheet,
    title: &'a str,
    lines: &'a [String],
    rings: &'a [SheetVectorRing],
    center: [f64; 2],
) -> Vec<u8> {
    render_vector_sheet_pdf(&VectorSheetPdf {
        sheet,
        title,
        lines,
        scale_label: "축척 1 : 5000",
        crs_label: "동부원점(GRS80) · EPSG:5187",
        legend_title: "범례",
        legend_rows: &["조사구역".to_owned()],
        north_label: "N",
        map_center_xy: center,
        rings,
    })
    .unwrap()
}

fn media_box(pdf: &[u8]) -> String {
    let text = String::from_utf8_lossy(pdf);
    let at = text.find("/MediaBox [0 0 ").unwrap();
    text[at..].split_once(']').unwrap().0.to_owned() + "]"
}

#[test]
fn vector_pdf_has_extractable_text_paths_and_cpp_page_boxes() {
    let a4 = page(SheetPaperSize::A4, SheetOrientation::Portrait);
    let lines = ["도면명   유구배치도".to_owned()];
    let pdf = paint(&a4, "유구배치도", &lines, &[], [200_000.0, 400_000.0]);
    let text = extracted_text(&pdf);
    assert!(text.contains("유구배치도"), "title is not text: {text}");
    assert!(text.contains("도면명   유구배치도"), "{text}");
    assert!(text.contains("축척 1 : 5000"), "{text}");
    assert!(text.contains("동부원점(GRS80) · EPSG:5187"), "{text}");
    assert!(text.contains("범례"), "{text}");
    assert!(text.contains("조사구역"), "{text}");
    assert!(text.contains('N'), "{text}");
    let raw = String::from_utf8_lossy(&pdf);
    assert!(
        raw.contains("BT") && raw.contains("Tj") && raw.contains("ET"),
        "no text operators"
    );
    assert!(
        raw.contains("Malgun Gothic"),
        "layout face is not Malgun Gothic"
    );
    assert!(
        raw.contains("15 Tf"),
        "title is not 15 pt (LayoutService.cpp:416)"
    );
    assert!(
        !raw.contains("/DCTDecode") && !raw.contains("/Subtype /Image"),
        "the page is a picture"
    );
    assert!(raw.contains("re"), "the frame is not a path");
    assert_eq!(
        media_box(&pdf),
        format!("/MediaBox [0 0 {} {}]", pt(210.0), pt(297.0))
    );
    assert!((a4.map_frame.x_mm - 10.0).abs() < 1e-9 && (a4.map_frame.y_mm - 10.0).abs() < 1e-9);
    let (x, y, w, h) = (
        pt(a4.map_frame.x_mm),
        pt(a4.height_mm - a4.map_frame.y_mm - a4.map_frame.height_mm),
        pt(a4.map_frame.width_mm),
        pt(a4.map_frame.height_mm),
    );
    assert!(
        raw.contains(&format!("{x} {y} {w} {h} re")),
        "map frame is not the 10 mm margin"
    );

    let field = page(SheetPaperSize::A4, SheetOrientation::Landscape);
    assert_eq!((field.width_mm, field.height_mm), (317.0, 220.0));
    let field_pdf = paint(&field, "현장", &[], &[], [0.0, 0.0]);
    assert_eq!(
        media_box(&field_pdf),
        format!("/MediaBox [0 0 {} {}]", pt(317.0), pt(220.0))
    );

    let a3 = page(SheetPaperSize::A3, SheetOrientation::Portrait);
    assert_eq!((a3.width_mm, a3.height_mm), (297.0, 420.0));
    let a3_pdf = paint(&a3, "A3", &[], &[], [0.0, 0.0]);
    assert_eq!(
        media_box(&a3_pdf),
        format!("/MediaBox [0 0 {} {}]", pt(297.0), pt(420.0))
    );
}

#[test]
fn a_survey_ring_is_a_path_at_the_paper_scale() {
    // Work CRS EPSG:5187, metres. At 1:5000, 1 m is 0.2 mm on the paper (1000/denominator).
    let mut sheet = page(SheetPaperSize::A4, SheetOrientation::Portrait);
    sheet.scale_denominator = 5_000;
    let center = [200_000.0, 400_000.0];
    let east = [center[0] + 100.0, center[1]];
    let ring = SheetVectorRing {
        xy: vec![center, east, [center[0] + 100.0, center[1] + 50.0]],
        filled: true,
    };
    let pdf = paint(&sheet, "유구", &[], &[ring], center);
    let raw = String::from_utf8_lossy(&pdf);
    assert!(
        raw.contains(" m") && raw.contains(" l"),
        "the ring is not a path: {raw}"
    );
    let cx = sheet.map_frame.x_mm + sheet.map_frame.width_mm * 0.5;
    let cy = sheet.map_frame.y_mm + sheet.map_frame.height_mm * 0.5;
    let x_mm = cx + 100.0 * (1000.0 / 5000.0);
    let y_mm = cy;
    let x_pt = pt(x_mm);
    let y_pt = pt(sheet.height_mm - y_mm);
    assert!(
        raw.contains(&format!("{x_pt} {y_pt} l")),
        "100 m east is not 20 mm on the page"
    );
}

#[test]
fn a_picture_page_has_no_extractable_layout_text() {
    let pdf = picture_page_pdf(
        [210.0, 297.0],
        "유구배치도",
        &[0xFF, 0xD8, 0xFF, 0xD9],
        [1, 1],
    );
    let text = extracted_text(&pdf);
    assert!(!text.contains("유구배치도"), "{text}");
    assert!(pdf.windows(10).any(|w| w == b"/DCTDecode"));
}
