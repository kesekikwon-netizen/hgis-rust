/// C++ `HGIS_MIN_AMBIENT_LIGHT` (0.15f) — 야간/일몰 후(`theta_sun < 0°`) 최소 주변광 강도 보장 상수
pub const HGIS_MIN_AMBIENT_LIGHT: f32 = 0.15;

/// C++ `KaAtmosphereAgX` 태양 고도각 기반 조도 계수 산출 — 야간(`sun_elevation_deg < 0.0`)에도 최소 `0.15f` 주변광을 보장
#[must_use]
pub fn compute_sun_illumination_factor(sun_elevation_deg: f32) -> f32 {
    if !sun_elevation_deg.is_finite() {
        return HGIS_MIN_AMBIENT_LIGHT;
    }
    let sin_el = sun_elevation_deg.to_radians().sin();
    sin_el.max(HGIS_MIN_AMBIENT_LIGHT)
}

pub fn agx_tonemap_srgb(linear_rgb: [f64; 3]) -> [f64; 3] {
    agx_tonemap_with_exposure_contrast(linear_rgb, 1.0, 1.0)
}

/// C++ `KaTerrain3dView::applyAgxToneMapping(float exposure, float contrast)`
#[must_use]
pub fn agx_tonemap_with_exposure_contrast(
    linear_rgb: [f64; 3],
    exposure: f64,
    contrast: f64,
) -> [f64; 3] {
    let exp_scale = exposure.max(0.01);
    let c_factor = contrast.clamp(0.2, 3.0);
    let exposed = linear_rgb.map(|c| (c * exp_scale).max(0.0));
    let inset = [
        0.842_479_062_253_094 * exposed[0]
            + 0.078_433_599_999_999_2 * exposed[1]
            + 0.079_223_745_147_764_3 * exposed[2],
        0.042_328_242_261_012_3 * exposed[0]
            + 0.878_468_636_469_772 * exposed[1]
            + 0.079_166_127_460_543_4 * exposed[2],
        0.042_375_654_905_705_1 * exposed[0]
            + 0.078_433_6 * exposed[1]
            + 0.879_142_973_793_104 * exposed[2],
    ];
    inset.map(|c| {
        let v = c.max(1e-10);
        let ev = (v.log2().clamp(-10.0, 6.5) + 10.0) / 16.5;
        let centered = (ev - 0.5) * c_factor + 0.5;
        let t = centered.clamp(0.0, 1.0);
        let s = t * t * (3.0 - 2.0 * t);
        s.clamp(0.0, 1.0)
    })
}

/// C++ `template <typename T> T sampleBilinear(const T* data, int w, int h, double fx, double fy)`
#[must_use]
pub fn sample_bilinear(data: &[f64], width: usize, height: usize, fx: f64, fy: f64) -> f64 {
    if data.is_empty() || width == 0 || height == 0 {
        return 0.0;
    }
    let x = fx.clamp(0.0, (width.saturating_sub(1)) as f64);
    let y = fy.clamp(0.0, (height.saturating_sub(1)) as f64);
    let x0 = (x.floor() as usize).min(width - 1);
    let y0 = (y.floor() as usize).min(height - 1);
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - (x0 as f64);
    let ty = y - (y0 as f64);

    let z00 = data[y0 * width + x0];
    let z10 = data[y0 * width + x1];
    let z01 = data[y1 * width + x0];
    let z11 = data[y1 * width + x1];

    let z0 = z00 * (1.0 - tx) + z10 * tx;
    let z1 = z01 * (1.0 - tx) + z11 * tx;
    z0 * (1.0 - ty) + z1 * ty
}

/// C++ `KaTerrain3dView::drapeVectorFeatures(const QList<QgsFeature> &features, float verticalOffset = 0.08f)`
#[must_use]
pub fn drape_vector_features_on_dem(
    features: &[Vec<[f64; 2]>],
    dem: &[f64],
    width: usize,
    height: usize,
    origin_xy: [f64; 2],
    cell_size: f64,
    vertical_offset_m: f64,
) -> Vec<Vec<[f64; 3]>> {
    let step = cell_size.abs().max(1e-6);
    features
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&[wx, wy]| {
                    let fx = (wx - origin_xy[0]) / step;
                    let fy = (wy - origin_xy[1]) / step;
                    let z = sample_bilinear(dem, width, height, fx, fy) + vertical_offset_m;
                    [wx, wy, z]
                })
                .collect()
        })
        .collect()
}

pub fn nishita_sky_zenith_horizon_rgb(sun_elevation_deg: f64) -> ([u8; 3], [u8; 3]) {
    let t = (sun_elevation_deg / 90.0).clamp(0.0, 1.0);
    let zenith = [
        (38.0 + 30.0 * t) as u8,
        (88.0 + 50.0 * t) as u8,
        (165.0 + 60.0 * t) as u8,
    ];
    let horizon = [
        (185.0 + 40.0 * t) as u8,
        (205.0 + 30.0 * t) as u8,
        (230.0 + 20.0 * t) as u8,
    ];
    (zenith, horizon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quadtree_lod::{build_quadtree_patches, build_quadtree_patches_with_vram_fallback};

    #[test]
    fn agx_tonemapping_and_quadtree_lod_produce_seamless_terrain_mesh() {
        let rgb = agx_tonemap_srgb([1.2, 0.8, 0.4]);
        assert!(rgb.iter().all(|&c| (0.0..=1.0).contains(&c)));
        let patches = build_quadtree_patches([0.0, 0.0], 4096.0, 4);
        assert!(!patches.is_empty());
        let (eff_lod, fb_patches) =
            build_quadtree_patches_with_vram_fallback([0.0, 0.0], 4096.0, 4, 8);
        assert_eq!(eff_lod, 3);
        assert!(fb_patches.len() < patches.len());
    }

    #[test]
    fn test_terrain_draping_15_deg_slope_adds_exact_008m_vertical_offset() {
        let slope_tan = 15.0_f64.to_radians().tan();
        let width = 11usize;
        let height = 11usize;
        let mut dem = vec![0.0_f64; width * height];
        for y in 0..height {
            for x in 0..width {
                dem[y * width + x] = (x as f64) * 10.0 * slope_tan;
            }
        }
        let rect_feature = vec![vec![
            [10.0, 10.0],
            [40.0, 10.0],
            [40.0, 30.0],
            [10.0, 30.0],
        ]];
        let draped = drape_vector_features_on_dem(
            &rect_feature,
            &dem,
            width,
            height,
            [0.0, 0.0],
            10.0,
            0.08,
        );
        assert_eq!(draped.len(), 1);
        assert_eq!(draped[0].len(), 4);
        for &[wx, _wy, wz] in &draped[0] {
            let expected_z = wx * slope_tan + 0.08;
            assert!((wz - expected_z).abs() < 1e-4);
        }
    }

    #[test]
    fn test_agx_tone_mapping_overexposed_rgb_rolls_off_without_clipping() {
        let mid = agx_tonemap_with_exposure_contrast([2.0, 2.0, 2.0], 1.0, 1.0);
        let high = agx_tonemap_with_exposure_contrast([8.0, 8.0, 8.0], 1.0, 1.0);
        for ch in 0..3 {
            assert!(mid[ch] > 0.5 && mid[ch] < 1.0);
            assert!(high[ch] > mid[ch] && high[ch] < 1.0);
        }

        // TestKaAtmosphereAgX::testHdrHighlightAgXCompression: vec3(10.0, 10.0, 10.0) in [0.0, 1.0]
        let hdr10 = agx_tonemap_srgb([10.0, 10.0, 10.0]);
        for &c in &hdr10 {
            assert!((0.0..=1.0).contains(&c));
            assert!(c > 0.85);
        }

        // Nighttime sun elevation (-15 deg) preserves minimum 0.15 ambient light
        let night_ambient = compute_sun_illumination_factor(-15.0);
        assert!((night_ambient - HGIS_MIN_AMBIENT_LIGHT).abs() < 1e-6);

        // TestKaQuadtreeLod::testSkirtDepthCalculation: 100.0m tile -> -5.0m skirt Z offset
        let skirt_z = crate::quadtree_lod::compute_skirt_z_offset_m(100.0);
        assert!((skirt_z - (-5.0)).abs() < 1e-6);

        // TestKaBoreholeCylinder3D::testZeroThicknessStrataSkip:
        // Strata: [0.0..3.5], [3.5..3.5 (0m)], [3.5..8.0] -> exactly 2 cylinder instances
        let strata = [
            crate::quadtree_lod::BoreholeStratumSpan {
                from_depth_m: 0.0,
                to_depth_m: 3.5,
                color_rgba: [194, 158, 120, 255],
            },
            crate::quadtree_lod::BoreholeStratumSpan {
                from_depth_m: 3.5,
                to_depth_m: 3.5,
                color_rgba: [0, 0, 0, 255],
            },
            crate::quadtree_lod::BoreholeStratumSpan {
                from_depth_m: 3.5,
                to_depth_m: 8.0,
                color_rgba: [222, 203, 166, 255],
            },
        ];
        let cylinders =
            crate::quadtree_lod::build_stratum_cylinders([219_800.0, 361_500.0, 45.0], &strata, 1.0);
        assert_eq!(cylinders.len(), 2);
        assert_eq!(cylinders[0].segments, 16);
        assert!((cylinders[0].height_m - 3.5).abs() < 1e-5);
        assert!((cylinders[1].height_m - 4.5).abs() < 1e-5);
    }
}
