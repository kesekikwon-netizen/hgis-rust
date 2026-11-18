//! A sheet as a vector PDF: text stays text and survey geometry stays paths.
//!
//! C++ `KaPdfExport` sets `forceVectorOutput = true`, `rasterizeWholeImage = false`
//! and `textRenderFormat = PreferText` (`target/cpp-ref/src/core/PdfExportSettings.cpp:9-26`).
//! `KaDrawingStudio::exportDrawingPdf` writes that file (`KaDrawingStudio.cpp:3793-3799`).
//! The face is Malgun Gothic (`LayoutService.cpp:394`); the sheet title is 15 pt
//! (`LayoutService.cpp:414-418`).

use strata_types::error::StrataResult;

use crate::sheet_composer::{nice_scale_bar_segment_meters, DrawingSheet, SheetRectMm};
use crate::sheet_pdf_export::mm_to_pdf_points;

/// One ring in the work CRS (EPSG:5186 or EPSG:5187), metres. North is up.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetVectorRing {
    pub xy: Vec<[f64; 2]>,
    pub filled: bool,
}

/// What one sheet PDF draws. `map_center_xy` and `rings` are work-CRS metres.
#[derive(Debug, Clone, Copy)]
pub struct VectorSheetPdf<'a> {
    pub sheet: &'a DrawingSheet,
    pub title: &'a str,
    pub lines: &'a [String],
    pub scale_label: &'a str,
    pub crs_label: &'a str,
    pub legend_title: &'a str,
    pub legend_rows: &'a [String],
    pub north_label: &'a str,
    pub map_center_xy: [f64; 2],
    pub rings: &'a [SheetVectorRing],
}

#[derive(Debug)]
pub(crate) struct Run {
    size: f64,
    x: f64,
    y: f64,
    text: String,
}

/// Bytes of the vector sheet. A picture of the page is not this file.
pub fn render_vector_sheet_pdf(doc: &VectorSheetPdf<'_>) -> StrataResult<Vec<u8>> {
    let (runs, body) = page_body(doc);
    let (cids, cmap) = encode::unicode_map(&runs);
    let mut stream = body;
    for run in &runs {
        let hex: String = run
            .text
            .chars()
            .filter_map(|ch| cids.get(&ch))
            .map(|id| format!("{id:04X}"))
            .collect();
        if hex.is_empty() {
            continue;
        }
        let size = format!("{:.2}", run.size)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned();
        stream.push_str(&format!(
            "BT /F1 {size} Tf {:.2} {:.2} Td <{hex}> Tj ET\n",
            run.x, run.y
        ));
    }
    Ok(encode::pdf_file(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            mm_to_pdf_points(doc.sheet.width_mm),
            mm_to_pdf_points(doc.sheet.height_mm)
        )
        .into_bytes(),
        encode::stream_obj(&stream),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /MalgunGothic /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /MalgunGothic /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 1000 >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /MalgunGothic /FontFamily (Malgun Gothic) /Flags 32 /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 700 /StemV 80 >>".to_vec(),
        encode::stream_obj(&cmap),
        b"<< /Producer (Strata) >>".to_vec(),
    ]))
}

fn page_body(doc: &VectorSheetPdf<'_>) -> (Vec<Run>, String) {
    let sheet = doc.sheet;
    let h = sheet.height_mm;
    let mut body = String::from("0.85 w 0.067 0.094 0.153 RG 0.067 0.094 0.153 rg\n");
    body.push_str(&rect_op(h, &sheet.map_frame, "S"));
    body.push_str(&rect_op(h, &sheet.scale_bar_rect, "S"));
    body.push_str(&scale_boxes(h, sheet));
    body.push_str(&rect_op(h, &sheet.legend_rect, "S"));
    body.push_str(&rect_op(h, &sheet.title_block_rect, "S"));
    body.push_str(&north_path(h, &sheet.north_arrow_rect));
    body.push_str(&rings_path(sheet, doc.map_center_xy, doc.rings));
    let mut runs = Vec::new();
    // LayoutService.cpp:414 — title band, 15 pt, 10 mm from the left and 5 mm from the top.
    text_at(&mut runs, h, 10.0, 5.0, 15.0, doc.title);
    let step = sheet.title_block_rect.height_mm / (doc.lines.len().max(1) as f64);
    for (i, line) in doc.lines.iter().enumerate() {
        text_at(
            &mut runs,
            h,
            sheet.title_block_rect.x_mm + 1.2,
            sheet.title_block_rect.y_mm + step * i as f64,
            6.5,
            line,
        );
    }
    // KaDrawingStudio.cpp:2915 — the words under the scale bar.
    text_at(
        &mut runs,
        h,
        sheet.scale_text_rect.x_mm,
        sheet.scale_text_rect.y_mm,
        9.0,
        doc.scale_label,
    );
    text_at(
        &mut runs,
        h,
        sheet.crs_label_rect.x_mm,
        sheet.crs_label_rect.y_mm,
        8.0,
        doc.crs_label,
    );
    text_at(
        &mut runs,
        h,
        sheet.north_arrow_rect.x_mm,
        sheet.north_arrow_rect.y_mm,
        12.0,
        doc.north_label,
    );
    text_at(
        &mut runs,
        h,
        sheet.legend_rect.x_mm + 1.5,
        sheet.legend_rect.y_mm + 1.0,
        f64::from(sheet.legend_font_pt.max(5)),
        doc.legend_title,
    );
    let row = (f64::from(sheet.legend_font_pt.max(6)) - 1.0).max(5.0);
    for (i, name) in doc.legend_rows.iter().enumerate() {
        text_at(
            &mut runs,
            h,
            sheet.legend_rect.x_mm + 1.5,
            sheet.legend_rect.y_mm + 6.0 + row * 0.35 * i as f64,
            row,
            name,
        );
    }
    (runs, body)
}

fn text_at(runs: &mut Vec<Run>, page_h: f64, x_mm: f64, top_mm: f64, size: f64, text: &str) {
    if text.is_empty() {
        return;
    }
    runs.push(Run {
        size,
        x: mm_to_pdf_points(x_mm),
        y: mm_to_pdf_points(page_h - top_mm) - size * 0.8,
        text: text.to_owned(),
    });
}

fn rect_op(page_h: f64, r: &SheetRectMm, op: &str) -> String {
    format!(
        "{:.2} {:.2} {:.2} {:.2} re {op}\n",
        mm_to_pdf_points(r.x_mm),
        mm_to_pdf_points(page_h - r.y_mm - r.height_mm),
        mm_to_pdf_points(r.width_mm),
        mm_to_pdf_points(r.height_mm)
    )
}

/// Four double-box segments (`LayoutService.cpp:598-627`, style "Double Box").
fn scale_boxes(page_h: f64, sheet: &DrawingSheet) -> String {
    let denom = f64::from(sheet.scale_denominator.max(1));
    let seg = nice_scale_bar_segment_meters(sheet.map_frame.width_mm, denom, 4);
    let width = (seg * 4.0 * 1000.0 / denom)
        .min(sheet.scale_bar_rect.width_mm)
        .max(1.0);
    let band = sheet.scale_bar_rect.height_mm / 2.0;
    let mut out = String::new();
    for row in 0..2 {
        for col in 0..4 {
            let cell = SheetRectMm {
                x_mm: sheet.scale_bar_rect.x_mm + width / 4.0 * f64::from(col),
                y_mm: sheet.scale_bar_rect.y_mm + band * f64::from(row),
                width_mm: width / 4.0,
                height_mm: band,
            };
            out.push_str(&rect_op(
                page_h,
                &cell,
                if (col + row) % 2 == 0 { "f" } else { "S" },
            ));
        }
    }
    out
}

fn north_path(page_h: f64, r: &SheetRectMm) -> String {
    let (cx, top, wing) = (r.x_mm + r.width_mm * 0.5, r.y_mm + 1.5, r.width_mm * 0.28);
    let pts = [
        [cx, top],
        [cx - wing, r.bottom() - 1.5],
        [cx + wing, r.bottom() - 1.5],
    ];
    path_mm(page_h, &pts, true, "f")
}

fn rings_path(sheet: &DrawingSheet, center: [f64; 2], rings: &[SheetVectorRing]) -> String {
    let mut out = String::new();
    let (x, y, w, h) = (
        mm_to_pdf_points(sheet.map_frame.x_mm),
        mm_to_pdf_points(sheet.height_mm - sheet.map_frame.y_mm - sheet.map_frame.height_mm),
        mm_to_pdf_points(sheet.map_frame.width_mm),
        mm_to_pdf_points(sheet.map_frame.height_mm),
    );
    if !rings.is_empty() {
        out.push_str(&format!("q {x:.2} {y:.2} {w:.2} {h:.2} re W n\n"));
    }
    for ring in rings {
        if ring.xy.len() < 2 {
            continue;
        }
        let paper: Vec<[f64; 2]> = ring
            .xy
            .iter()
            .map(|xy| sheet.world_to_sheet_mm(center, *xy))
            .collect();
        out.push_str(&path_mm(
            sheet.height_mm,
            &paper,
            ring.filled,
            if ring.filled { "f" } else { "S" },
        ));
    }
    if !rings.is_empty() {
        out.push_str("Q\n");
    }
    out
}

fn path_mm(page_h: f64, pts: &[[f64; 2]], close: bool, op: &str) -> String {
    let mut out = String::new();
    for (i, p) in pts.iter().enumerate() {
        let cmd = if i == 0 { "m" } else { "l" };
        out.push_str(&format!(
            "{:.2} {:.2} {cmd}\n",
            mm_to_pdf_points(p[0]),
            mm_to_pdf_points(page_h - p[1])
        ));
    }
    if close {
        out.push_str("h\n");
    }
    out.push_str(op);
    out.push('\n');
    out
}

#[path = "sheet_pdf_encode.rs"]
mod encode;

pub use encode::extracted_text;

#[cfg(test)]
#[path = "sheet_pdf_vector_tests.rs"]
mod tests;
