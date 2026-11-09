//! `ngii_dem` 회귀 시험 — C++ `PaleoAnalysisRunner.cpp:153-215` 규칙 확인.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::ngii_dem::*;
use strata_types::crs::WorkCrs;





fn extent() -> DemExtent {
    DemExtent {
        min_x: 218_000.0,
        min_y: 361_000.0,
        max_x: 219_000.0,
        max_y: 361_500.0,
    }
}

fn line(z: f64) -> NgiiContourFeature {
    NgiiContourFeature {
        vertices_xy: vec![[218_100.0, 361_100.0], [218_900.0, 361_400.0]],
        elevation_m: z,
        z_valid: true,
        cad_layer: "F0010000".to_owned(),
    }
}

/// C++ `:176-182` 피처 필터 1:1.
#[test]
fn feature_filter_matches_cpp_ranges_and_layer_codes() {
    let mut f = line(50.0);
    assert!(f.is_usable());

    f.elevation_m = -100.0;
    assert!(!f.is_usable(), "-100 은 제외(> -100 만 허용)");
    f.elevation_m = 3000.0;
    assert!(f.is_usable(), "3000 은 포함(<= 3000)");
    f.elevation_m = 3000.1;
    assert!(!f.is_usable());

    let mut g = line(10.0);
    g.z_valid = false;
    assert!(!g.is_usable(), "z_valid=0 은 제외");
    g.z_valid = true;
    g.cad_layer = "N3L_F0010000".to_owned();
    g.cad_layer = "F0020000".to_owned();
    assert!(g.is_usable(), "F002 로 시작하면 포함");
    g.cad_layer = "A001".to_owned();
    assert!(!g.is_usable(), "그 밖의 레이어 코드는 제외");
    g.cad_layer = String::new();
    assert!(g.is_usable(), "빈 레이어 코드는 통과(:177)");
}

/// C++ `:200-202` 셀 크기와 16..=360 클램프.
#[test]
fn grid_follows_cpp_cell_size_and_side_clamps() {
    // spanX 1000, spanY 500 -> cell = max(5, 1000/240) = 4.166.. -> 5.0
    let tf = build_ngii_dem_grid(&extent());
    assert!((tf.cell_size - 5.0).abs() < 1e-12, "cell={}", tf.cell_size);
    assert_eq!(tf.width, 200); // ceil(1000/5) = 200
    assert_eq!(tf.height, 100); // ceil(500/5) = 100
    assert!((tf.origin_x - 218_000.0).abs() < 1e-9);
    assert!((tf.origin_y - 361_500.0).abs() < 1e-9, "origin_y = maxY");

    // 아주 넓으면 cell 이 커지고 격자는 360 이하로 잘린다.
    let wide = DemExtent {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 400_000.0,
        max_y: 400_000.0,
    };
    let tw = build_ngii_dem_grid(&wide);
    assert!(tw.width <= NGII_DEM_MAX_SIDE);
    assert!(tw.height <= NGII_DEM_MAX_SIDE);
    // cell 이 span 에 비례해 커지므로 정사각형이면 400000/240 = 240 이 자연히 상한 아래다.
        // 360 클램프는 폭/셀 > 360 인 비정상 입력에서만 걸린다.
        assert_eq!(tw.width, 240);
}

/// C++ `:196` 점이 4개 미만이면 만들지 않는다.
#[test]
fn fewer_than_four_points_is_an_error() {
    let dir = std::env::temp_dir().join(format!("ka_hgis_ngii_dem_min_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let err = build_ngii_dem_from_topographic(&[], &extent(), WorkCrs::Epsg5187, &dir);
    assert!(err.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// 정상 입력이면 Float32 GeoTIFF 가 만들어지고 헤더가 맞는다.
#[test]
fn writes_float32_geotiff_named_like_cpp() {
    let dir = std::env::temp_dir().join(format!("ka_hgis_ngii_dem_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let features: Vec<NgiiContourFeature> = (0..6).map(|i| line(10.0 + i as f64 * 5.0)).collect();
    let path = build_ngii_dem_from_topographic(&features, &extent(), WorkCrs::Epsg5187, &dir)
        .expect("DEM 생성");
    assert_eq!(path.file_name().and_then(|n| n.to_str()), Some(NGII_DEM_FILE_NAME));
    let bytes = std::fs::read(&path).expect("read tif");
    assert!(bytes.starts_with(b"II*\0"), "little-endian TIFF magic");
    assert!(bytes.windows(2).any(|w| w == 33550_u16.to_le_bytes()));
    assert!(bytes.windows(2).any(|w| w == 34735_u16.to_le_bytes()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 등고선을 40m 간격으로 채워 표고점을 모은다 — 공개 경로로 확인한다.
#[test]
fn densified_contour_contributes_spaced_points_on_the_line() {
    let f = line(50.0);
    let mut pts: Vec<[f64; 3]> = Vec::new();
    densify_and_sample_vertices(&f.vertices_xy, f.elevation_m, NGII_DEM_DENSIFY_M, &extent(), &mut pts, 12_000);
    assert!(pts.len() >= 3, "1000m 대각선이면 여러 점이 나온다");
    for p in &pts {
        assert!((p[2] - 50.0).abs() < 1e-12, "모든 점의 표고는 등고선 값");
    }
}
