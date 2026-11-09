//! 래스터 타일 늘리기(percentile stretch)·NoData 마스킹·TFW 파라미터.
//!
//! `pdf_geotiff` 에서 분리했다.

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
