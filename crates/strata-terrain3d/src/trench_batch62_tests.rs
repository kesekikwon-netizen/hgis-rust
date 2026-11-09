//! 62차 배치 트렌치 지형 시험.
//!
//! trench_profile3d 에서 분리했다.

use crate::trench_profile3d::*;

#[cfg(test)]
mod batch62_trench_terrain_tests {
    use super::*;

    #[test]
    fn test_trench_volume_calculation_and_number_from_high_side() {
        // TestTrenchTerrain::testVolumeCalculation:
        // Flat ground Z = 50.0m, 2.0m x 20.0m trench (area = 40.0 m²), floor Z = 48.5m (depth = 1.5m) -> 60.0 m³
        let res = carve_trench_volume_from_samples(&[50.0, 50.0, 50.0, 50.0], 40.0, 48.5, 4);
        assert!((res.excavated_volume_m3 - 60.0).abs() <= 1e-2);
        assert!((res.max_excavation_depth_m - 1.5).abs() <= 1e-6);
        assert!(res.warning_message.is_none());

        // Negative cut guard (floor higher than ground)
        let neg = carve_trench_volume_from_samples(&[50.0], 40.0, 52.0, 4);
        assert!((neg.excavated_volume_m3 - 0.0).abs() < 1e-9);
        assert_eq!(
            neg.warning_message.as_deref(),
            Some("굴착 바닥고가 지표면보다 높아 절개가 수행되지 않았습니다.")
        );

        // TrenchTerrain::numberFromHighSide: north (0 deg) is high side -> higher Y gets T1, left-to-right gets T1 then T2
        let centers = [[20.0, 10.0], [10.0, 100.0], [20.0, 100.0]];
        let numbered = number_trenches_from_high_side(&centers, 0.0, "T");
        assert_eq!(numbered[0], (1, "T1".to_owned()));
        assert_eq!(numbered[1], (2, "T2".to_owned()));
        assert_eq!(numbered[2], (0, "T3".to_owned()));
    }

    #[test]
    fn test_terrain_rgb_decoding_and_flat_terrain_hillshade() {
        // TestBasemapDsm::testTerrainRgbDecoding: R=1, G=134 (0x86), B=160 (0xA0) -> 100,000 -> 0.0m
        let elev = decode_terrain_rgb_pixel(1, 134, 160);
        assert!((elev - 0.0).abs() <= 1e-4);
        assert_eq!(copernicus_dsm_title_ko(), "고도지형 (Copernicus 30 m)");
        assert_eq!(
            hillshade_output_path("/tmp/site_dem.tif"),
            "/tmp/site_dem_hillshade.tif"
        );

        // TestHillshadeTileRenderer::testFlatTerrainHillshade: flat 50.0m grid, sun altitude 45.0 deg -> 180
        let flat = vec![50.0_f32; 20 * 20];
        let hs = compute_horn_hillshade_byte_grid(&flat, 20, 20, 1.0, 45.0, 315.0);
        assert_eq!(hs.len(), 400);
        assert_eq!(hs[10 * 20 + 10], 180);
    }
}
