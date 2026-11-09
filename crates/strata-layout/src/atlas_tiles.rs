//! 선형 아틀라스 타일 분할과 접합선 라벨.
//!
//! `tile_print` 에서 분리했다.

pub const HGIS_DEFAULT_ATLAS_OVERLAP_PCT: f64 = 15.0;

pub const HGIS_MATCH_LINE_PEN_WIDTH_MM: f64 = 0.5;

#[derive(Debug, Clone, PartialEq)]
pub struct AtlasConfig {
    pub scale_denominator: f64,
    pub paper_size_mm: [f64; 2],
    pub overlap_percent: f64,
    pub custom_grid_angle_deg: f64,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            scale_denominator: 500.0,
            paper_size_mm: [420.0, 297.0],
            overlap_percent: HGIS_DEFAULT_ATLAS_OVERLAP_PCT,
            custom_grid_angle_deg: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtlasSheetTile {
    pub page_number: usize,
    pub sheet_name: String,
    pub center_xy: [f64; 2],
    pub tile_extent: [f64; 4],
    pub left_sheet_no: usize,
    pub right_sheet_no: usize,
    pub top_sheet_no: usize,
    pub bottom_sheet_no: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasTileError {
    EmptyLinearRoute,
}

/// C++ `KaAtlasSheetGenerator::generateLinearAtlasTiles` — 선형 유적 노선 중심선을 따라 `D_step = W_ground * (1.0 - overlap_pct / 100.0)` 간격으로 아틀라스 도엽 생성 및 인접 도엽 번호 부여
pub fn generate_linear_atlas_tiles(
    linear_route: &[[f64; 2]],
    config: &AtlasConfig,
) -> Result<Vec<AtlasSheetTile>, AtlasTileError> {
    if linear_route.len() < 2 {
        return Err(AtlasTileError::EmptyLinearRoute);
    }
    let mut seg_lengths = Vec::with_capacity(linear_route.len() - 1);
    let mut total_len = 0.0;
    for pair in linear_route.windows(2) {
        let len = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
        seg_lengths.push(len);
        total_len += len;
    }
    if total_len <= 1e-9 {
        return Err(AtlasTileError::EmptyLinearRoute);
    }

    let scale = config.scale_denominator.max(1.0);
    let w_ground = (config.paper_size_mm[0].max(10.0) * scale) / 1000.0;
    let h_ground = (config.paper_size_mm[1].max(10.0) * scale) / 1000.0;
    let overlap_ratio = (config.overlap_percent / 100.0).clamp(0.0, 0.8);
    let d_step = (w_ground * (1.0 - overlap_ratio)).max(1.0);

    let interpolate_along = |dist: f64| -> [f64; 2] {
        let clamped = dist.clamp(0.0, total_len);
        let mut acc = 0.0;
        for (idx, &seg_len) in seg_lengths.iter().enumerate() {
            if acc + seg_len >= clamped || idx + 1 == seg_lengths.len() {
                let t = if seg_len > 1e-12 {
                    ((clamped - acc) / seg_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let p0 = linear_route[idx];
                let p1 = linear_route[idx + 1];
                return [p0[0] + t * (p1[0] - p0[0]), p0[1] + t * (p1[1] - p0[1])];
            }
            acc += seg_len;
        }
        linear_route[linear_route.len() - 1]
    };

    let tile_count = ((total_len / d_step).ceil() as usize).max(1);
    let mut tiles = Vec::with_capacity(tile_count);
    for idx in 0..tile_count {
        let page_no = idx + 1;
        let dist = (idx as f64 * d_step).min(total_len);
        let c = interpolate_along(dist);
        let half_w = w_ground * 0.5;
        let half_h = h_ground * 0.5;
        tiles.push(AtlasSheetTile {
            page_number: page_no,
            sheet_name: format!("제{page_no}도엽"),
            center_xy: c,
            tile_extent: [c[0] - half_w, c[1] - half_h, c[0] + half_w, c[1] + half_h],
            left_sheet_no: page_no.saturating_sub(1),
            right_sheet_no: if page_no < tile_count { page_no + 1 } else { 0 },
            top_sheet_no: 0,
            bottom_sheet_no: 0,
        });
    }

    Ok(tiles)
}

/// C++ `KaMatchLineIndicator::renderMatchLineOverlay` — 인접 도엽 번호에 따른 접합선 라벨 생성 (`"제N도엽 접합선"`)
#[must_use]
pub fn format_match_line_label_ko(neighbor_sheet_no: usize) -> Option<String> {
    if neighbor_sheet_no == 0 {
        None
    } else {
        Some(format!("제{neighbor_sheet_no}도엽 접합선"))
    }
}
