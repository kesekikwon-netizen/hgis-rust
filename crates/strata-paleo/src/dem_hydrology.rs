//! D8 흐름 방향과 하천 셀 임계값 필터.
//!
//! `dem_synthesizer` 에서 분리했다.

use crate::dem_synthesis_io::HGIS_DEM_NODATA_THRESHOLD;

pub const HGIS_DEFAULT_STREAM_THRESH_CELL: i32 = 500;

pub const HGIS_D8_EAST: u8 = 1;

pub const HGIS_D8_SOUTHEAST: u8 = 2;

pub const HGIS_D8_SOUTH: u8 = 4;

pub const HGIS_D8_SOUTHWEST: u8 = 8;

pub const HGIS_D8_WEST: u8 = 16;

pub const HGIS_D8_NORTHWEST: u8 = 32;

pub const HGIS_D8_NORTH: u8 = 64;

pub const HGIS_D8_NORTHEAST: u8 = 128;

/// C++ `KaFlowDirectionD8::computeD8FlowDirection` — 8방위(`[E, SE, S, SW, W, NW, N, NE]`) 중 최대 하강 경사(`(z_c - z_n) / dist > 0`) 방향의 D8 비트마스크 코드 반환 (싱크/평탄면은 `0`)
#[must_use]
pub fn compute_d8_cell_flow_direction(
    center_z: f32,
    neighbors_clockwise_from_east: &[f32; 8],
    cell_size_m: f64,
) -> u8 {
    if !center_z.is_finite() || center_z <= HGIS_DEM_NODATA_THRESHOLD {
        return 0;
    }
    let cs = cell_size_m.max(1e-3);
    let diag = cs * std::f64::consts::SQRT_2;
    let codes: [u8; 8] = [
        HGIS_D8_EAST,
        HGIS_D8_SOUTHEAST,
        HGIS_D8_SOUTH,
        HGIS_D8_SOUTHWEST,
        HGIS_D8_WEST,
        HGIS_D8_NORTHWEST,
        HGIS_D8_NORTH,
        HGIS_D8_NORTHEAST,
    ];
    let mut best_slope = 0.0_f64;
    let mut best_code = 0_u8;
    for (idx, &nz) in neighbors_clockwise_from_east.iter().enumerate() {
        if !nz.is_finite() || nz <= HGIS_DEM_NODATA_THRESHOLD {
            continue;
        }
        let drop = f64::from(center_z - nz);
        if drop <= 0.0 {
            continue;
        }
        let dist = if idx % 2 == 0 { cs } else { diag };
        let slope = drop / dist;
        if slope > best_slope {
            best_slope = slope;
            best_code = codes[idx];
        }
    }
    best_code
}

/// C++ `KaPaleoStreamExtractor::extractPaleoStreamNetwork` — 흐름 누적 셀 수 `acc >= streamThresholdCells` 수계 셀 인덱스 필터링
#[must_use]
pub fn filter_stream_cells_by_threshold(
    accumulation_cells: &[i32],
    threshold_cells: i32,
) -> Vec<usize> {
    let thresh = if threshold_cells > 0 {
        threshold_cells
    } else {
        HGIS_DEFAULT_STREAM_THRESH_CELL
    };
    accumulation_cells
        .iter()
        .enumerate()
        .filter_map(|(idx, &acc)| if acc >= thresh { Some(idx) } else { None })
        .collect()
}
