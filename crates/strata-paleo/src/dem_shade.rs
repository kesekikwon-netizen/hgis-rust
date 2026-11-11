//! 표고 격자를 색 입힌 그림으로 — 표고 색띠 + 음영.
//!
//! 색띠는 C++ `DemPresentation.cpp:34-40` 의 16단계(0 m 청록 → 2000 m 흰색)를 그대로 쓰고
//! 사이는 곧게 잇는다(`ShaderInterpolationMethod::Linear`). 범위를 벗어난 값은 끝 색으로 누른다.
//! 그 위에 북서쪽(방위 315°)·고도 45° 빛의 음영을 곱해 지형의 굴곡이 보이게 한다.

use crate::dem_tiff_read::DemRaster;

/// C++ `kStops` — (표고 m, 색).
pub const ELEVATION_STOPS: [(f32, [u8; 3]); 16] = [
    (0.0, [0x72, 0xc6, 0xb5]),
    (10.0, [0x81, 0xce, 0xaa]),
    (25.0, [0x91, 0xd3, 0x96]),
    (50.0, [0xa5, 0xd6, 0x80]),
    (75.0, [0xba, 0xd9, 0x75]),
    (100.0, [0xd0, 0xdc, 0x78]),
    (150.0, [0xe0, 0xda, 0x7b]),
    (200.0, [0xeb, 0xd2, 0x7a]),
    (300.0, [0xed, 0xbe, 0x70]),
    (400.0, [0xe8, 0xa6, 0x66]),
    (600.0, [0xd9, 0x8b, 0x5a]),
    (800.0, [0xc4, 0x74, 0x52]),
    (1000.0, [0xad, 0x66, 0x50]),
    (1300.0, [0xb6, 0x98, 0x86]),
    (1600.0, [0xd1, 0xc5, 0xbb]),
    (2000.0, [0xf4, 0xf3, 0xef]),
];

/// 그림 한 변의 최대 칸 수(넘으면 건너뛰며 줄인다).
pub const MAX_IMAGE_SIDE: usize = 1024;

/// 표고의 색.
#[must_use]
pub fn elevation_color(z: f32) -> [u8; 3] {
    let first = ELEVATION_STOPS[0];
    if z <= first.0 {
        return first.1;
    }
    for pair in ELEVATION_STOPS.windows(2) {
        let ((z0, c0), (z1, c1)) = (pair[0], pair[1]);
        if z <= z1 {
            let t = (z - z0) / (z1 - z0);
            let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
            return [mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2])];
        }
    }
    ELEVATION_STOPS[ELEVATION_STOPS.len() - 1].1
}

/// 색 입힌 그림. 줄 순서는 북→남, 한 칸은 RGBA 4바이트(자료 없는 칸은 투명).
#[derive(Debug, Clone, PartialEq)]
pub struct ShadedDem {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    /// 덮는 범위 `[west, south, east, north]`(경위도).
    pub bbox: [f64; 4],
}

/// 표고 격자(경위도)를 색띠 + 음영 그림으로 만든다.
#[must_use]
pub fn shade_dem(dem: &DemRaster) -> ShadedDem {
    let step = dem.width.max(dem.height).div_ceil(MAX_IMAGE_SIDE).max(1);
    let (w, h) = (dem.width.div_ceil(step), dem.height.div_ceil(step));
    // 한 칸의 실제 크기(m): 위도에 따라 동서 간격이 줄어든다.
    let mid_lat = (dem.north - dem.dy * dem.height as f64 * 0.5).to_radians();
    let cell_x = (dem.dx * step as f64 * 111_320.0 * mid_lat.cos()) as f32;
    let cell_y = (dem.dy * step as f64 * 110_540.0) as f32;
    let at = |col: usize, row: usize| -> f32 {
        let (c, r) = ((col * step).min(dem.width - 1), (row * step).min(dem.height - 1));
        dem.values[r * dem.width + c]
    };
    // 빛: 북서쪽 위 45°. 단위 벡터(동, 북, 위).
    let light = [-0.5_f32, 0.5, std::f32::consts::FRAC_1_SQRT_2];
    let mut rgba = vec![0_u8; w * h * 4];
    for row in 0..h {
        for col in 0..w {
            let z = at(col, row);
            if !z.is_finite() {
                continue;
            }
            // 이웃(없거나 자료 없음이면 제 값)으로 기울기를 잰다.
            let near = |c: usize, r: usize| {
                let v = at(c, r);
                if v.is_finite() { v } else { z }
            };
            let dz_east = (near((col + 1).min(w - 1), row) - near(col.saturating_sub(1), row)) / (2.0 * cell_x);
            let dz_north = (near(col, row.saturating_sub(1)) - near(col, (row + 1).min(h - 1))) / (2.0 * cell_y);
            // 면의 법선(-dz/dx, -dz/dy, 1)과 빛의 각.
            let len = (dz_east * dz_east + dz_north * dz_north + 1.0).sqrt();
            let lit = ((-dz_east * light[0] - dz_north * light[1] + light[2]) / len).clamp(0.0, 1.0);
            // 평지(0.707)가 원래 색이 되게 맞추고, 그늘은 55 % 까지만 어둡게 한다.
            let gain = (0.55 + 0.45 * (lit / std::f32::consts::FRAC_1_SQRT_2)).min(1.25);
            let color = elevation_color(z);
            let i = (row * w + col) * 4;
            for k in 0..3 {
                rgba[i + k] = (f32::from(color[k]) * gain).round().clamp(0.0, 255.0) as u8;
            }
            rgba[i + 3] = 255;
        }
    }
    ShadedDem { width: w, height: h, rgba, bbox: dem.bbox() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(width: usize, height: usize, values: Vec<f32>) -> DemRaster {
        DemRaster { west: 127.0, north: 37.0, dx: 1.0 / 3600.0, dy: 1.0 / 3600.0, width, height, values }
    }

    #[test]
    fn the_colour_ramp_is_the_cpp_one_and_clamps_at_both_ends() {
        assert_eq!(elevation_color(0.0), [0x72, 0xc6, 0xb5]);
        assert_eq!(elevation_color(-30.0), [0x72, 0xc6, 0xb5]);
        assert_eq!(elevation_color(100.0), [0xd0, 0xdc, 0x78]);
        assert_eq!(elevation_color(2000.0), [0xf4, 0xf3, 0xef]);
        assert_eq!(elevation_color(9000.0), [0xf4, 0xf3, 0xef]);
        // 사이는 곧게 잇는다: 125 m 는 100 m 색과 150 m 색의 가운데.
        assert_eq!(elevation_color(125.0), [0xd8, 0xdb, 0x7a]);
    }

    #[test]
    fn flat_ground_keeps_its_colour_and_slopes_are_lit_from_the_north_west() {
        // 평지 3×3: 가운데 칸은 색띠 색 그대로.
        let flat = shade_dem(&raster(3, 3, vec![100.0; 9]));
        assert_eq!(&flat.rgba[16..20], &[0xd0, 0xdc, 0x78, 255]);
        // 서쪽이 낮고 동쪽이 높은 비탈: 빛(북서)을 등진 면이 아니라 마주 본 면 — 서쪽으로 내려가는 면이 밝다.
        let west_facing = shade_dem(&raster(3, 3, vec![100.0, 130.0, 160.0, 100.0, 130.0, 160.0, 100.0, 130.0, 160.0]));
        let east_facing = shade_dem(&raster(3, 3, vec![160.0, 130.0, 100.0, 160.0, 130.0, 100.0, 160.0, 130.0, 100.0]));
        assert!(west_facing.rgba[16] > east_facing.rgba[16], "{} {}", west_facing.rgba[16], east_facing.rgba[16]);
    }

    #[test]
    fn missing_cells_are_transparent_and_large_grids_are_thinned() {
        let mut values = vec![50.0; 4];
        values[3] = f32::NAN;
        let shaded = shade_dem(&raster(2, 2, values));
        assert_eq!(shaded.rgba[3], 255);
        assert_eq!(shaded.rgba[15], 0);
        let big = shade_dem(&raster(3000, 10, vec![10.0; 30_000]));
        assert_eq!((big.width, big.height), (1000, 4));
        assert_eq!(big.bbox, [127.0, 37.0 - 10.0 / 3600.0, 127.0 + 3000.0 / 3600.0, 37.0]);
    }
}
