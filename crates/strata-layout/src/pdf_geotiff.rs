use strata_types::error::StrataResult;

use crate::sheet_composer::{DrawingSheet, SheetRectMm};

pub fn render_sheet_pdf_bytes(sheet: &DrawingSheet, title: &str) -> StrataResult<Vec<u8>> {
    const MM_TO_PT: f64 = 2.834_645_67;
    let pt_w = sheet.width_mm * MM_TO_PT;
    let pt_h = sheet.height_mm * MM_TO_PT;
    let rect_cmd = |r: &SheetRectMm| -> String {
        let x = r.x_mm * MM_TO_PT;
        let y = (sheet.height_mm - r.y_mm - r.height_mm) * MM_TO_PT;
        let w = r.width_mm * MM_TO_PT;
        let h = r.height_mm * MM_TO_PT;
        format!("{x:.2} {y:.2} {w:.2} {h:.2} re S\n")
    };
    let mut stream = String::from("0.8 w 0.14 0.13 0.11 RG\n");
    stream.push_str(&rect_cmd(&sheet.map_frame));
    stream.push_str(&rect_cmd(&sheet.scale_bar_rect));
    stream.push_str(&rect_cmd(&sheet.crs_label_rect));
    stream.push_str(&rect_cmd(&sheet.north_arrow_rect));
    if sheet.legend_kept_on_page {
        stream.push_str(&rect_cmd(&sheet.legend_rect));
    }

    let pdf = format!(
        "%PDF-1.4\n\
         1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
         2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
         3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 {pt_w:.2} {pt_h:.2}] /Contents 4 0 R /Title ({title}) >> endobj\n\
         4 0 obj << /Length {} >>\nstream\n{stream}endstream\nendobj\n\
         xref\n0 5\ntrailer << /Root 1 0 R /Size 5 >>\n%%EOF\n",
        stream.len()
    );
    Ok(pdf.into_bytes())
}

#[must_use]
pub fn render_geotiff_header_bytes(width_px: u32, height_px: u32, epsg_code: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(64);
    // Little-endian TIFF magic ("II*\0") + IFD offset 8
    bytes.extend_from_slice(b"II*\0");
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    // Number of directory entries = 3 (ImageWidth=256, ImageLength=257, GeoKeyDirectoryTag=34735)
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    // Tag 256 (ImageWidth)
    bytes.extend_from_slice(&256_u16.to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&width_px.to_le_bytes());
    // Tag 257 (ImageLength)
    bytes.extend_from_slice(&257_u16.to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&height_px.to_le_bytes());
    // Tag 34735 (0x87AF GeoKeyDirectoryTag) + EPSG code
    bytes.extend_from_slice(&34735_u16.to_le_bytes());
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&epsg_code.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet_composer::{SheetOrientation, SheetPaperSize};

    #[test]
    fn pdf_draws_map_frame_and_geotiff_includes_geokey_directory_tag() -> StrataResult<()> {
        let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
        let pdf = render_sheet_pdf_bytes(&sheet, "경주 시굴조사 도면")?;
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.contains("stream"));
        assert!(pdf_str.contains("re S"));

        let tiff = render_geotiff_header_bytes(1024, 768, 5187);
        assert_eq!(&tiff[0..4], b"II*\0");
        assert!(tiff.windows(2).any(|w| w == [0xAF, 0x87]));
        Ok(())
    }
}
