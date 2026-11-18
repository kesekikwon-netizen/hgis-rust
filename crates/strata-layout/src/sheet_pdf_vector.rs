//! A sheet as a vector PDF: text stays text and survey geometry stays paths.
//!
//! C++ `KaPdfExport` sets `forceVectorOutput = true`, `rasterizeWholeImage = false`
//! and `textRenderFormat = PreferText` (`target/cpp-ref/src/core/PdfExportSettings.cpp:9-26`).
//! `KaDrawingStudio::exportDrawingPdf` writes that file (`KaDrawingStudio.cpp:3793-3799`).
//! The face is Malgun Gothic (`LayoutService.cpp:394`); the sheet title is 15 pt
//! (`LayoutService.cpp:414-418`).

use strata_types::error::StrataResult;

use crate::sheet_composer::DrawingSheet;
use crate::sheet_pdf_export::mm_to_pdf_points;

/// One ring in the work CRS (EPSG:5186 or EPSG:5187), metres. North is up.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetVectorRing {
    pub xy: Vec<[f64; 2]>,
    pub filled: bool,
}

/// A feature label in the work CRS (EPSG:5186 or EPSG:5187), metres.
/// C++ prints it as text (`PdfExportSettings.cpp:17` PreferText, `LabelOps.cpp:67-130`).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPdfLabel {
    pub text: String,
    pub xy: [f64; 2],
    pub size_pt: f64,
}

/// What one sheet PDF draws. `map_center_xy`, `rings` and `labels` are work-CRS metres.
/// `map_jpeg` is the basemap painted inside the map frame, when one was drawn.
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
    pub labels: &'a [SheetPdfLabel],
    pub map_jpeg: Option<&'a [u8]>,
    pub map_jpeg_px: [u32; 2],
}

#[derive(Debug)]
pub(crate) struct Run {
    size: f64,
    x: f64,
    y: f64,
    text: String,
}

/// Bytes of the vector sheet. A picture of the whole page is not this file.
pub fn render_vector_sheet_pdf(doc: &VectorSheetPdf<'_>) -> StrataResult<Vec<u8>> {
    let (mut runs, mut body) = page_body(doc);
    for label in doc.labels {
        let at = doc.sheet.world_to_sheet_mm(doc.map_center_xy, label.xy);
        text_at(&mut runs, doc.sheet.height_mm, at[0], at[1], label.size_pt, &label.text);
    }
    let face = font::embed_face(runs.iter().flat_map(|run| run.text.chars()));
    let assigned = face.as_ref().map(|face| face.cid.clone()).unwrap_or_default();
    let (cids, cmap) = encode::unicode_map(&runs, &assigned);
    for run in &runs {
        let hex: String = run.text.chars().filter_map(|ch| cids.get(&ch)).map(|id| format!("{id:04X}")).collect();
        if hex.is_empty() {
            continue;
        }
        let size = format!("{:.2}", run.size).trim_end_matches('0').trim_end_matches('.').to_owned();
        body.push_str(&format!("BT /F1 {size} Tf {:.2} {:.2} Td <{hex}> Tj ET\n", run.x, run.y));
    }
    let image = doc.map_jpeg.filter(|bytes| !bytes.is_empty());
    let font_obj = 9;
    let image_obj = 10;
    let info = if image.is_some() { 11 } else { 10 };
    if image.is_some() {
        let frame = &doc.sheet.map_frame;
        let (x, y, w, h) = (
            mm_to_pdf_points(frame.x_mm),
            mm_to_pdf_points(doc.sheet.height_mm - frame.y_mm - frame.height_mm),
            mm_to_pdf_points(frame.width_mm),
            mm_to_pdf_points(frame.height_mm),
        );
        // PdfExportSettings.cpp:25 disables tiled raster draws; the basemap is still
        // painted in the map. It sits under the vectors, clipped to the frame.
        body = format!("q {x:.2} {y:.2} {w:.2} {h:.2} re W n q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /Im0 Do Q Q\n{body}");
    }
    let (family, widths, file) = match &face {
        Some(face) => (face.family.clone(), encode::width_array(&face.width), Some(face.ttf.clone())),
        None => ("Malgun Gothic".to_owned(), String::new(), None),
    };
    let base = family.replace(' ', "");
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        encode::page_dict(doc, image.is_some(), image_obj),
        encode::stream_obj(&body),
        format!("<< /Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>").into_bytes(),
        format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{base} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity {widths} >>").into_bytes(),
        format!("<< /Type /FontDescriptor /FontName /{base} /FontFamily ({family}) /Flags 32 /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 700 /StemV 80 /FontFile2 {font_obj} 0 R >>").into_bytes(),
        encode::stream_obj(&cmap),
    ];
    if let Some(bytes) = file {
        objects.push(encode::stream_bytes(&bytes));
    } else {
        objects.push(encode::stream_bytes(b""));
    }
    if let Some(jpeg) = image {
        objects.push(encode::image_obj_bytes(jpeg, doc.map_jpeg_px));
    }
    objects.push(b"<< /Producer (Strata) >>".to_vec());
    Ok(encode::pdf_file(&objects, info))
}

fn page_body(doc: &VectorSheetPdf<'_>) -> (Vec<Run>, String) {
    let sheet = doc.sheet;
    let h = sheet.height_mm;
    let mut body = String::from("0.85 w 0.067 0.094 0.153 RG 0.067 0.094 0.153 rg\n");
    body.push_str(&paths::rect_op(h, &sheet.map_frame, "S"));
    body.push_str(&paths::rect_op(h, &sheet.scale_bar_rect, "S"));
    body.push_str(&paths::scale_boxes(h, sheet));
    body.push_str(&paths::rect_op(h, &sheet.legend_rect, "S"));
    body.push_str(&paths::rect_op(h, &sheet.title_block_rect, "S"));
    body.push_str(&paths::north_path(h, &sheet.north_arrow_rect));
    body.push_str(&paths::rings_path(sheet, doc.map_center_xy, doc.rings));
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

#[path = "sheet_pdf_paths.rs"]
mod paths;
#[path = "sheet_pdf_encode.rs"]
mod encode;
#[path = "sheet_pdf_font.rs"]
mod font;

pub use encode::extracted_text;

#[cfg(test)]
#[path = "sheet_pdf_vector_tests.rs"]
mod tests;
