use strata_types::error::StrataResult;

use crate::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use crate::pdf_geotiff::*;

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
    assert!(
        export_geotiff_with_cancel(&cancel_tif, 2480, 3508, 5186, Some(&cancelled)).is_err()
    );
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

#[test]
fn test_percentile_2_to_98_stretch_and_nodata_alpha_masking() {
    // TestKaHistogramStretchEngine::testPercentile2To98Stretch
    // 100 valid pixels: 2 bottom = 0.0, 3rd (2% index) = 10.0, 98th = 90.0, 2 top = 100.0
    let mut band = Vec::with_capacity(101);
    band.push(0.0_f32);
    band.push(0.0_f32);
    for i in 0..96 {
        let val = 10.0 + (80.0 * (i as f32) / 95.0);
        band.push(val);
    }
    band.push(100.0_f32);
    band.push(100.0_f32);
    // Append a mid-value pixel 50.0 and a NoData pixel -9999.0
    band[50] = 50.0_f32;
    band.push(-9999.0_f32);

    let tile = stretch_and_mask_raster_tile(&band, -9999.0, StretchMode::Percentile2To98);
    assert!(tile.has_valid_data);
    assert!((tile.min_stretched - 10.0).abs() < 1e-3);
    assert!((tile.max_stretched - 90.0).abs() < 1e-3);
    // Pixel 50 (value 50.0) -> round((50 - 10) / 80 * 255) = round(127.5) = 128
    assert_eq!(tile.rgba_bytes[50 * 4], 128);
    assert_eq!(tile.rgba_bytes[50 * 4 + 3], 255);
    // Last pixel (-9999.0 NoData) -> alpha == 0
    let last_alpha = tile.rgba_bytes[100 * 4 + 3];
    assert_eq!(last_alpha, 0);
    assert_eq!(select_raster_overview_level(0.4, 0.05, 5), 3);

    // TestKaGeoTiffExporter::testTfwWorldFileParameters:
    // BBOX [100.0, 200.0, 300.0, 400.0], 1000x1000px -> A=0.20, E=-0.20, C=100.10, F=399.90
    let tfw = compute_tfw_world_file_params([100.0, 200.0, 300.0, 400.0], 1000, 1000);
    let lines = tfw.to_tfw_lines();
    assert_eq!(lines[0], "0.20");
    assert_eq!(lines[3], "-0.20");
    assert_eq!(lines[4], "100.10");
    assert_eq!(lines[5], "399.90");
    assert!(validate_export_image_buffer_size(4960, 3508).is_ok());
    assert!(validate_export_image_buffer_size(9000, 9000).is_err());
}
