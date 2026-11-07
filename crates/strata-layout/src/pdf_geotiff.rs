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

pub const HGIS_DEFAULT_RASTER_TILE_SIZE: usize = 256;
pub const HGIS_HISTO_PERCENTILE_LOW: f64 = 0.02;
pub const HGIS_HISTO_PERCENTILE_HIGH: f64 = 0.98;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StretchMode {
    MinMax,
    Percentile2To98,
    StandardDeviation2Sigma,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StretchedTileBuffer {
    pub rgba_bytes: Vec<u8>,
    pub has_valid_data: bool,
    pub min_stretched: f32,
    pub max_stretched: f32,
}

/// C++ `KaRasterPyramidReader` — 현재 뷰포트 해상도(`view_res_m`)와 원본 픽셀 해상도(`native_res_m`) 비율로부터 피라미드 오버뷰 레벨 `L = max(0, floor(log2(view_res_m / native_res_m)))` 산출
#[must_use]
pub fn select_raster_overview_level(view_res_m: f64, native_res_m: f64, max_level: u32) -> u32 {
    if view_res_m <= 0.0
        || native_res_m <= 0.0
        || !view_res_m.is_finite()
        || !native_res_m.is_finite()
    {
        return 0;
    }
    let ratio = view_res_m / native_res_m;
    if ratio <= 1.0 {
        0
    } else {
        (ratio.log2().floor().max(0.0) as u32).min(max_level)
    }
}

/// C++ `KaRasterPyramidReader::readAndStretchTile` & `KaHistogramStretchEngine` — 2% ~ 98% 퍼센타일 대비 스트레칭, 플랫 히스토그램(`|vmax - vmin| <= 1e-6` -> `128`) 방어 및 NoData 완전 투명(`alpha = 0`) 마스킹
#[must_use]
pub fn stretch_and_mask_raster_tile(
    raw_band: &[f32],
    no_data_val: f32,
    mode: StretchMode,
) -> StretchedTileBuffer {
    let mut valid_vals: Vec<f32> = raw_band
        .iter()
        .copied()
        .filter(|&v| v.is_finite() && (v - no_data_val).abs() > 1e-5)
        .collect();

    if valid_vals.is_empty() {
        return StretchedTileBuffer {
            rgba_bytes: vec![0_u8; raw_band.len() * 4],
            has_valid_data: false,
            min_stretched: 0.0,
            max_stretched: 0.0,
        };
    }

    valid_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = valid_vals.len();
    let (v_min, v_max) = match mode {
        StretchMode::MinMax => (valid_vals[0], valid_vals[n - 1]),
        StretchMode::Percentile2To98 => {
            let low_idx = ((n as f64 * HGIS_HISTO_PERCENTILE_LOW).floor() as usize).min(n - 1);
            let high_idx = ((n as f64 * HGIS_HISTO_PERCENTILE_HIGH).floor().max(0.0) as usize)
                .saturating_sub(1)
                .min(n - 1);
            (valid_vals[low_idx], valid_vals[high_idx.max(low_idx)])
        }
        StretchMode::StandardDeviation2Sigma => {
            let mean = valid_vals.iter().map(|&v| f64::from(v)).sum::<f64>() / n as f64;
            let var = valid_vals
                .iter()
                .map(|&v| (f64::from(v) - mean).powi(2))
                .sum::<f64>()
                / n as f64;
            let sigma = var.sqrt();
            ((mean - 2.0 * sigma) as f32, (mean + 2.0 * sigma) as f32)
        }
    };

    let span = f64::from(v_max) - f64::from(v_min);
    let is_flat = span.abs() <= 1e-6;
    let mut rgba_bytes = Vec::with_capacity(raw_band.len() * 4);
    for &v in raw_band {
        if !v.is_finite() || (v - no_data_val).abs() <= 1e-5 {
            rgba_bytes.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let byte_val = if is_flat {
                128_u8
            } else {
                let norm = ((f64::from(v) - f64::from(v_min)) / span * 255.0)
                    .round()
                    .clamp(0.0, 255.0);
                norm as u8
            };
            rgba_bytes.extend_from_slice(&[byte_val, byte_val, byte_val, 255]);
        }
    }

    StretchedTileBuffer {
        rgba_bytes,
        has_valid_data: true,
        min_stretched: v_min,
        max_stretched: v_max,
    }
}

/// C++ `HGIS_MAX_IMAGE_BUFFER_BYTES` — 최대 허용 렌더 버퍼 크기 (`256 MB` = `0x1000_0000`)
pub const HGIS_MAX_IMAGE_BUFFER_BYTES: usize = 0x1000_0000;

/// C++ `KaGeoTiffExporter` World File (`.tfw`) 6-파라미터 (`A, D, B, E, C, F`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TfwWorldFileParams {
    pub a_pixel_size_x: f64,
    pub d_rot_y: f64,
    pub b_rot_x: f64,
    pub e_pixel_size_y: f64,
    pub c_origin_center_x: f64,
    pub f_origin_center_y: f64,
}

impl TfwWorldFileParams {
    /// `.tfw` 6줄 텍스트 포맷 반환
    #[must_use]
    pub fn to_tfw_lines(self) -> [String; 6] {
        [
            format!("{:.2}", self.a_pixel_size_x),
            format!("{:.2}", self.d_rot_y),
            format!("{:.2}", self.b_rot_x),
            format!("{:.2}", self.e_pixel_size_y),
            format!("{:.2}", self.c_origin_center_x),
            format!("{:.2}", self.f_origin_center_y),
        ]
    }
}

/// C++ `KaGeoTiffExporter::exportMapToGeoTiff` — BBOX `[min_x, min_y, max_x, max_y]`와 픽셀 크기 `(width_px, height_px)`로부터 `.tfw` 6파라미터 산출
#[must_use]
pub fn compute_tfw_world_file_params(
    bbox_min_max_xy: [f64; 4],
    width_px: u32,
    height_px: u32,
) -> TfwWorldFileParams {
    let w = f64::from(width_px.max(1));
    let h = f64::from(height_px.max(1));
    let a = (bbox_min_max_xy[2] - bbox_min_max_xy[0]) / w;
    let e = -((bbox_min_max_xy[3] - bbox_min_max_xy[1]) / h);
    let c = bbox_min_max_xy[0] + a * 0.5;
    let f = bbox_min_max_xy[3] + e * 0.5;
    TfwWorldFileParams {
        a_pixel_size_x: a,
        d_rot_y: 0.0,
        b_rot_x: 0.0,
        e_pixel_size_y: e,
        c_origin_center_x: c,
        f_origin_center_y: f,
    }
}

/// C++ `KaGeoTiffExporter` / `KaClipboardImageExporter` — 고해상도 래스터 버퍼 OOM 사전 검사 (`width * height * 4 <= 256MB`)
pub fn validate_export_image_buffer_size(
    width_px: usize,
    height_px: usize,
) -> Result<usize, &'static str> {
    let bytes = width_px.saturating_mul(height_px).saturating_mul(4);
    if bytes == 0 || bytes > HGIS_MAX_IMAGE_BUFFER_BYTES {
        return Err("요청한 해상도가 시스템 메모리 허용 한계를 초과했습니다. DPI를 낮춰주세요.");
    }
    Ok(bytes)
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
}
