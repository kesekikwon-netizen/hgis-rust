//! WFS 타일 분할 계획 (C++ `BasemapOps.cpp` 의 타일 나눔 상한).
//!
//! `vworld_client` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;

/// 대용량 WFS/WMS 타일 분할 수집 계획 (C++ `TiledFetchPlan`)
#[derive(Debug, Clone, PartialEq)]
pub struct TiledFetchPlan {
    pub total_extent: [f64; 4],
    pub num_tiles_x: usize,
    pub num_tiles_y: usize,
    pub sub_tile_extents: Vec<[f64; 4]>,
}

/// 광역 조사구역 BBOX를 단일 타일 최대 폭/높이(`max_tile_width_m`, `max_tile_height_m`) 이하의 격자 서브 타일로 분할
/// (C++ `ReferenceTiledFetch::calculateTileSplits` 1:1 대응)
#[must_use]
pub fn calculate_tile_splits(
    target_extent: [f64; 4],
    max_tile_width_m: f64,
    max_tile_height_m: f64,
) -> TiledFetchPlan {
    let min_x = target_extent[0].min(target_extent[2]);
    let min_y = target_extent[1].min(target_extent[3]);
    let max_x = target_extent[0].max(target_extent[2]);
    let max_y = target_extent[1].max(target_extent[3]);
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    let tile_w = max_tile_width_m.max(1.0);
    let tile_h = max_tile_height_m.max(1.0);

    let nx = ((width / tile_w).ceil() as usize).clamp(1, HGIS_WFS_MAX_TILE_SPLITS);
    let ny = ((height / tile_h).ceil() as usize).clamp(1, HGIS_WFS_MAX_TILE_SPLITS);
    let step_x = width / (nx as f64);
    let step_y = height / (ny as f64);

    let mut sub_tile_extents = Vec::with_capacity(nx * ny);
    for iy in 0..ny {
        let y0 = min_y + (iy as f64) * step_y;
        let y1 = if iy + 1 == ny { max_y } else { y0 + step_y };
        for ix in 0..nx {
            let x0 = min_x + (ix as f64) * step_x;
            let x1 = if ix + 1 == nx { max_x } else { x0 + step_x };
            sub_tile_extents.push([x0, y0, x1, y1]);
        }
    }

    TiledFetchPlan {
        total_extent: [min_x, min_y, max_x, max_y],
        num_tiles_x: nx,
        num_tiles_y: ny,
        sub_tile_extents,
    }
}
