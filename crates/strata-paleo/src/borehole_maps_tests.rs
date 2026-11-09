//! `borehole_maps` 오케스트레이터 통합 시험 — 파일 산출 계약 검증.
//!
//! 본문 모듈을 300줄 이하로 유지하기 위해 시험을 분리했다(`#[path]` 로 붙인다).

use strata_net::geoinfo_borehole::BoreholeRecord;
use strata_types::crs::WorkCrs;
use strata_types::error::StrataResult;

use crate::borehole_map_options::BoreholeMapOptions;
use crate::borehole_maps::{
    generate_borehole_gis_maps, FILE_ALLUVIAL_ISOPACH, FILE_BEDROCK_DEPTH, FILE_BEDROCK_ELEV,
    FILE_FILL_THICKNESS, FILE_GROUNDWATER, FILE_ISOPACH_GPKG, TITLE_GROUNDWATER,
};

fn hole(x: f64, y: f64, alt: f64, fill: f64, alluv: f64, weathered: f64, water: f64) -> BoreholeRecord {
    BoreholeRecord {
        hole_code: "BH".to_owned(),
        tm_x: x,
        tm_y: y,
        altitude_m: alt,
        fill_depth_m: fill,
        alluvial_from_m: fill,
        alluvial_to_m: fill + alluv,
        alluvial_thickness_m: alluv,
        weathered_depth_m: weathered,
        water_level_m: water,
        dominant_uscs: "SM".to_owned(),
        paleo_hint: String::new(),
        layers: Vec::new(),
    }
}

fn sample_holes() -> Vec<BoreholeRecord> {
    vec![
        hole(200_000.0, 400_000.0, 35.0, 1.0, 3.0, 8.0, 2.5),
        hole(200_500.0, 400_000.0, 37.0, 2.0, 4.0, 9.5, 3.0),
        hole(200_000.0, 400_500.0, 33.0, 0.5, 2.0, -1.0, 0.0),
    ]
}

#[test]
fn fewer_than_two_holes_creates_no_maps_except_marker() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let holes = vec![hole(200_000.0, 400_000.0, 35.0, 1.0, 3.0, 8.0, 2.5)];
    let bundle = generate_borehole_gis_maps(
        &holes,
        &[],
        WorkCrs::Epsg5187,
        dir.path(),
        &BoreholeMapOptions::default(),
    )?;
    assert_eq!(bundle.created_count, 1, "marker only");
    assert!(bundle.written_files.is_empty());
    assert_eq!(bundle.grid.width, 0);
    assert!(bundle.layers.is_empty());
    Ok(())
}

#[test]
fn full_generation_writes_all_enabled_files_with_cpp_names() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let kigam = vec![[200_250.0, 400_250.0, 4.2]];
    let opts = BoreholeMapOptions {
        bedrock_elev_map: true,
        ..BoreholeMapOptions::default()
    };
    let bundle =
        generate_borehole_gis_maps(&sample_holes(), &kigam, WorkCrs::Epsg5187, dir.path(), &opts)?;

    // 마커(1) + 충적 래스터(1) + 등층후선(1) + 성토(1) + 기반암깊이(1) + 기반암표고(1) + 수위(1)
    assert_eq!(bundle.created_count, 7);
    assert_eq!(bundle.written_files.len(), 6);

    for name in [
        FILE_ALLUVIAL_ISOPACH,
        FILE_FILL_THICKNESS,
        FILE_BEDROCK_DEPTH,
        FILE_BEDROCK_ELEV,
        FILE_GROUNDWATER,
        FILE_ISOPACH_GPKG,
    ] {
        let p = dir.path().join(name);
        assert!(p.exists(), "expected {name} to exist");
        assert!(p.metadata().map(|m| m.len()).unwrap_or(0) > 0);
    }
    assert_eq!((bundle.grid.width, bundle.grid.height), (180, 180));
    assert!(bundle.grid.cell_size >= 5.0);
    Ok(())
}

#[test]
fn groundwater_map_skipped_when_fewer_than_two_combined_points() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    // 시추 지하수위 모두 0 -> 결합점 0
    let holes = vec![
        hole(0.0, 0.0, 10.0, 1.0, 2.0, 5.0, 0.0),
        hole(300.0, 0.0, 12.0, 1.0, 2.0, 6.0, 0.0),
    ];
    let bundle = generate_borehole_gis_maps(
        &holes,
        &[],
        WorkCrs::Epsg5187,
        dir.path(),
        &BoreholeMapOptions::default(),
    )?;
    assert!(!dir.path().join(FILE_GROUNDWATER).exists());
    assert!(bundle.layers.iter().all(|l| l.title != TITLE_GROUNDWATER));
    Ok(())
}

#[test]
fn invalid_radius_is_rejected() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let opts = BoreholeMapOptions {
        radius_meters: 1234,
        ..BoreholeMapOptions::default()
    };
    let err = generate_borehole_gis_maps(&sample_holes(), &[], WorkCrs::Epsg5187, dir.path(), &opts);
    assert!(err.is_err());
    Ok(())
}

#[test]
fn disabled_toggles_write_nothing() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let opts = BoreholeMapOptions {
        marker_map: false,
        alluvial_isopach_map: false,
        fill_thickness_map: false,
        bedrock_depth_map: false,
        bedrock_elev_map: false,
        groundwater_map: false,
        ..BoreholeMapOptions::default()
    };
    let bundle =
        generate_borehole_gis_maps(&sample_holes(), &[], WorkCrs::Epsg5187, dir.path(), &opts)?;
    assert_eq!(bundle.created_count, 0);
    assert!(bundle.written_files.is_empty());
    assert!(bundle.layers.is_empty());
    Ok(())
}

#[test]
fn geotiff_output_is_readable_and_carries_work_crs() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let opts = BoreholeMapOptions {
        marker_map: false,
        alluvial_isopach_map: true,
        fill_thickness_map: false,
        bedrock_depth_map: false,
        bedrock_elev_map: false,
        groundwater_map: false,
        ..BoreholeMapOptions::default()
    };
    let bundle =
        generate_borehole_gis_maps(&sample_holes(), &[], WorkCrs::Epsg5186, dir.path(), &opts)?;
    let tif = dir.path().join(FILE_ALLUVIAL_ISOPACH);
    assert!(tif.exists());
    let info = crate::borehole_geotiff::read_geotiff_header_info(&tif)?;
    assert_eq!((info.width, info.height), (180, 180));
    assert_eq!(info.bits_per_sample, vec![32]);
    assert_eq!(info.sample_format, vec![3]);
    assert_eq!(info.geo_keys.len(), 12);
    assert_eq!(info.geo_keys[11], 5186, "EPSG:5186 written into GeoKey");
    assert_eq!(bundle.grid.width, 180);
    Ok(())
}
