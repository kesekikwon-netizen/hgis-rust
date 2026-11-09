//! 지형 RGB 디코드·음영기복(hillshade) 출력.
//!
//! `trench_profile3d` 에서 분리했다.

/// Mapbox / VWorld Terrain-RGB 24비트 타일 픽셀(`R, G, B`)을 실수 표고(`m`)로 디코딩
/// `elevation_m = -10000.0 + ((R * 65536 + G * 256 + B) * 0.1)`
/// (C++ `BasemapDsm::decodeTerrainRgbPixel` 1:1 대응)
#[must_use]
pub fn decode_terrain_rgb_pixel(r: u8, g: u8, b: u8) -> f32 {
    let packed = ((u32::from(r)) << 16) | ((u32::from(g)) << 8) | u32::from(b);
    -10_000.0_f32 + (packed as f32) * 0.1_f32
}

/// DEM 파일 경로에서 음영기복(`_hillshade.tif`) 출력 경로 도출 (C++ `DemHillshadeFile::outputPath`)
#[must_use]
pub fn hillshade_output_path(dem_path: &str) -> String {
    let trimmed = dem_path.trim();
    if let Some((stem, _ext)) = trimmed.rsplit_once('.') {
        if !stem.is_empty() {
            return format!("{stem}_hillshade.tif");
        }
    }
    format!("{trimmed}_hillshade.tif")
}

/// Horn (1981) 8방향 차분 커널을 이용해 DEM 격자에서 8비트 그레이스케일 음영기복(`0..=255`) 생성
/// 평탄 지형(`slope = 0`)에서 태양 고도 `45°`일 때 `floor(255.0 * cos(45°)) = 180` 반환
/// (C++ `HillshadeTileRenderer::renderHillshade` / `DemHillshadeFile::write` 1:1 대응)
#[must_use]
pub fn compute_horn_hillshade_byte_grid(
    grid: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
    sun_altitude_deg: f64,
    sun_azimuth_deg: f64,
) -> Vec<u8> {
    if width == 0 || height == 0 || grid.len() < width * height {
        return Vec::new();
    }
    let cs = if cell_size_m.is_finite() && cell_size_m > 1e-6 {
        cell_size_m
    } else {
        1.0
    };
    let zenith_rad = (90.0 - sun_altitude_deg.clamp(0.0, 90.0)).to_radians();
    let azimuth_math_rad = (360.0 - sun_azimuth_deg + 90.0)
        .rem_euclid(360.0)
        .to_radians();

    let sample = |r: isize, c: isize| -> f64 {
        let rr = r.clamp(0, (height as isize) - 1) as usize;
        let cc = c.clamp(0, (width as isize) - 1) as usize;
        let v = f64::from(grid[rr * width + cc]);
        if v.is_finite() {
            v
        } else {
            0.0
        }
    };

    let mut out = vec![0_u8; width * height];
    for r in 0..height {
        for c in 0..width {
            let ri = r as isize;
            let ci = c as isize;
            let z_nw = sample(ri - 1, ci - 1);
            let z_n = sample(ri - 1, ci);
            let z_ne = sample(ri - 1, ci + 1);
            let z_w = sample(ri, ci - 1);
            let z_e = sample(ri, ci + 1);
            let z_sw = sample(ri + 1, ci - 1);
            let z_s = sample(ri + 1, ci);
            let z_se = sample(ri + 1, ci + 1);

            let dz_dx = ((z_ne + 2.0 * z_e + z_se) - (z_nw + 2.0 * z_w + z_sw)) / (8.0 * cs);
            let dz_dy = ((z_sw + 2.0 * z_s + z_se) - (z_nw + 2.0 * z_n + z_ne)) / (8.0 * cs);
            let slope_rad = dz_dx.hypot(dz_dy).atan();
            let aspect_rad = if dz_dx.abs() > 1e-12 || dz_dy.abs() > 1e-12 {
                dz_dy.atan2(-dz_dx).rem_euclid(std::f64::consts::TAU)
            } else {
                0.0
            };

            let cos_i = zenith_rad.cos() * slope_rad.cos()
                + zenith_rad.sin() * slope_rad.sin() * (azimuth_math_rad - aspect_rad).cos();
            let val = (255.0 * cos_i.max(0.0)).floor().clamp(0.0, 255.0) as u8;
            out[r * width + c] = val;
        }
    }
    out
}
