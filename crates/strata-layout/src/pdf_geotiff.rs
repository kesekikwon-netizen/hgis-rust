use strata_types::error::StrataResult;

use crate::sheet_composer::{DrawingSheet, SheetRectMm};

pub const HGIS_MM_PER_INCH: f64 = 25.4;
pub const HGIS_POINTS_PER_INCH: f64 = 72.0;
pub const HGIS_TIFFTAG_MODELPIXELSCALE: u16 = 33550;
pub const HGIS_TIFFTAG_MODELTIEPOINT: u16 = 33922;
pub const HGIS_TIFFTAG_GEOKEYDIRECTORY: u16 = 34735;
pub const HGIS_GEOKEY_PROJECTED_CS_TYPE: u16 = 3072;

/// C++ `KaPdfExporter` — 밀리미터(mm) 단위를 PostScript/PDF 포인트(`mm * 72.0 / 25.4`)로 변환
#[must_use]
pub fn mm_to_pdf_points(mm: f64) -> f64 {
    mm * HGIS_POINTS_PER_INCH / HGIS_MM_PER_INCH
}

pub fn render_sheet_pdf_bytes(sheet: &DrawingSheet, title: &str) -> StrataResult<Vec<u8>> {
    let pt_w = mm_to_pdf_points(sheet.width_mm);
    let pt_h = mm_to_pdf_points(sheet.height_mm);
    let rect_cmd = |r: &SheetRectMm| -> String {
        let x = mm_to_pdf_points(r.x_mm);
        let y = mm_to_pdf_points(sheet.height_mm - r.y_mm - r.height_mm);
        let w = mm_to_pdf_points(r.width_mm);
        let h = mm_to_pdf_points(r.height_mm);
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
    let mut bytes = Vec::with_capacity(96);
    // Little-endian TIFF magic ("II*\0") + IFD offset 8
    bytes.extend_from_slice(b"II*\0");
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    // Number of directory entries = 5 (256, 257, 33550, 33922, 34735)
    bytes.extend_from_slice(&5_u16.to_le_bytes());
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
    // Tag 33550 (ModelPixelScaleTag)
    bytes.extend_from_slice(&33550_u16.to_le_bytes());
    bytes.extend_from_slice(&12_u16.to_le_bytes());
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    // Tag 33922 (ModelTiepointTag)
    bytes.extend_from_slice(&33922_u16.to_le_bytes());
    bytes.extend_from_slice(&12_u16.to_le_bytes());
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    // Tag 34735 (0x87AF GeoKeyDirectoryTag) + EPSG code
    bytes.extend_from_slice(&34735_u16.to_le_bytes());
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes.extend_from_slice(&epsg_code.to_le_bytes());
    // GeoKeyDirectory payload: GTModelTypeGeoKey(1024)=1 (ModelTypeProjected), ProjectedCSTypeGeoKey(3072)=epsg_code
    for &val in &[1_u16, 1, 0, 2, 1024, 0, 1, 1, 3072, 0, 1, epsg_code as u16] {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// C++ `SheetComposer::exportGeoTiff` — `std::atomic<bool> m_isCancelled` 취소 시 `.tif.tmp` 소거 및 Tag 34735 내장
pub fn export_geotiff_with_cancel(
    out_path: &std::path::Path,
    width_px: u32,
    height_px: u32,
    epsg_code: u32,
    is_cancelled: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(), &'static str> {
    use std::sync::atomic::Ordering;

    let tmp_path = out_path.with_extension("tif.tmp");
    let hdr = render_geotiff_header_bytes(width_px, height_px, epsg_code);
    if std::fs::write(&tmp_path, &hdr).is_err() {
        return Err("GeoTIFF 임시 파일 기록에 실패했습니다");
    }
    if is_cancelled.is_some_and(|c| c.load(Ordering::Relaxed)) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("사용자에 의해 GeoTIFF 내보내기가 취소되었습니다");
    }
    std::fs::rename(&tmp_path, out_path).map_err(|_| {
        let _ = std::fs::remove_file(&tmp_path);
        "GeoTIFF 최종 파일 교체에 실패했습니다"
    })
}

/// C++ `KaPdfExporter::exportLayoutToPdf` & `cleanupPartialFile` — 쓰기 실패 또는 디스크 용량 부족 시 불완전 `.pdf` / `.pdf.tmp` 즉시 삭제
pub fn export_sheet_pdf_atomic(
    sheet: &DrawingSheet,
    title: &str,
    out_pdf_path: &std::path::Path,
    simulate_disk_full: bool,
) -> Result<(), &'static str> {
    let tmp_path = out_pdf_path.with_extension("pdf.tmp");
    let bytes = render_sheet_pdf_bytes(sheet, title).map_err(|_| "PDF 스트림 생성 실패")?;
    if std::fs::write(&tmp_path, &bytes).is_err() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("PDF 임시 파일 쓰기 실패");
    }
    if simulate_disk_full {
        let _ = std::fs::remove_file(&tmp_path);
        let _ = std::fs::remove_file(out_pdf_path);
        return Err("디스크 용량 부족으로 불완전 PDF 파일을 삭제했습니다");
    }
    std::fs::rename(&tmp_path, out_pdf_path).map_err(|_| {
        let _ = std::fs::remove_file(&tmp_path);
        "PDF 최종 파일 교체 실패"
    })
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

    #[test]
    fn test_geotiff_tag34735_export_embeds_model_type_1_and_epsg_5186_and_cleans_tmp_on_cancel(
    ) -> StrataResult<()> {
        use std::sync::atomic::AtomicBool;

        let dir = std::env::temp_dir().join(format!("ka_geotiff_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let out_tif = dir.join("plate_5186.tif");
        let not_cancelled = AtomicBool::new(false);
        export_geotiff_with_cancel(&out_tif, 2480, 3508, 5186, Some(&not_cancelled))
            .expect("geotiff export should succeed");
        let bytes = std::fs::read(&out_tif)?;
        // Check Tag 34735 (0x87AF), GTModelTypeGeoKey(1024 = 0x0400) = 1, ProjectedCSTypeGeoKey(3072 = 0x0C00) = 5186 (0x1442)
        assert!(bytes.windows(2).any(|w| w == 34735_u16.to_le_bytes()));
        assert!(bytes
            .windows(8)
            .any(|w| w == [0x00, 0x04, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00]));
        assert!(bytes
            .windows(8)
            .any(|w| w == [0x00, 0x0C, 0x00, 0x00, 0x01, 0x00, 0x42, 0x14]));

        let cancel_tif = dir.join("cancelled.tif");
        let cancelled = AtomicBool::new(true);
        assert!(export_geotiff_with_cancel(&cancel_tif, 2480, 3508, 5186, Some(&cancelled)).is_err());
        assert!(!cancel_tif.exists());
        assert!(!cancel_tif.with_extension("tif.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn test_a4_landscape_mediabox_84189_59528_epsg5187_geokey_and_cleanup_on_disk_full(
    ) -> StrataResult<()> {
        let sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Landscape);
        assert!((mm_to_pdf_points(sheet.width_mm) - 841.89).abs() <= 0.01);
        assert!((mm_to_pdf_points(sheet.height_mm) - 595.28).abs() <= 0.01);

        let pdf = render_sheet_pdf_bytes(&sheet, "A4 가로 도면")?;
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.contains("/MediaBox [0 0 841.89 595.28]"));

        // EPSG:5187 (0x1443) GeoKey 3072 (0x0C00) 인코딩 검증
        let tiff_5187 = render_geotiff_header_bytes(512, 512, 5187);
        assert!(tiff_5187
            .windows(8)
            .any(|w| w == [0x00, 0x0C, 0x00, 0x00, 0x01, 0x00, 0x43, 0x14]));

        let dir = std::env::temp_dir().join(format!("ka_pdf_cleanup_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let target_pdf = dir.join("partial_test.pdf");
        assert!(export_sheet_pdf_atomic(&sheet, "테스트", &target_pdf, true).is_err());
        assert!(!target_pdf.exists());
        assert!(!target_pdf.with_extension("pdf.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
