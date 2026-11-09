//! 검증 게이트 V16~V17 — 시추 5종 도면 생성과 화면 칠하기.
//!
//! `verify_harness.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 판정 조건·문구는 그대로다.

use strata_types::crs::WorkCrs;

use super::verify_harness_types::GateResult;

/// V16 (`d7afcc8`): 시추 5종 IDW GeoTIFF + 0.5m 등층후선 GPKG 를 실제로 만들어 검증한다.
///
/// C++ `GeoInfoBoreholeMaps.cpp:179-294` 의 산출물 규격을 파일 수준에서 확인한다:
/// 180×180 Float32 GeoTIFF, Tag 33550/33922/34735, GPKG 매직·SRID·등층후선 레이어.
#[must_use]
pub fn v16_borehole_maps_generation_gate() -> GateResult {
    use strata_net::geoinfo_borehole::BoreholeRecord;
    use strata_paleo::borehole_map_options::BoreholeMapOptions;
    use strata_paleo::borehole_maps::{
        generate_borehole_gis_maps, BOREHOLE_MIN_POINTS, FILE_ISOPACH_GPKG,
    };

    fn hole(code: &str, x: f64, y: f64, fill: f64, alluv: f64, weath: f64, water: f64) -> BoreholeRecord {
        BoreholeRecord {
            hole_code: code.to_owned(),
            tm_x: x,
            tm_y: y,
            altitude_m: 40.0,
            fill_depth_m: fill,
            alluvial_from_m: fill,
            alluvial_to_m: fill + alluv,
            alluvial_thickness_m: alluv,
            weathered_depth_m: weath,
            water_level_m: water,
            dominant_uscs: "CL".to_owned(),
            paleo_hint: "실트질점토".to_owned(),
            layers: Vec::new(),
        }
    }

    // C++ `:205-225` 처럼 여러 공을 흩어 놓아야 IDW 격자가 의미를 갖는다.
    let holes = [
        hole("BH-01", 218_500.0, 361_400.0, 1.2, 4.2, 5.4, 2.1),
        hole("BH-02", 218_900.0, 361_600.0, 0.8, 3.1, -1.0, 1.8),
        hole("BH-03", 219_300.0, 361_200.0, 2.0, 6.5, 8.5, 3.2),
        hole("BH-04", 218_200.0, 361_000.0, 0.5, 2.4, 2.9, 0.0),
        hole("BH-05", 219_700.0, 361_800.0, 1.6, 5.0, 6.6, 2.7),
    ];
    let kigam: [[f64; 3]; 2] = [[218_650.0, 361_250.0, 1.9], [219_050.0, 361_700.0, 2.6]];

    let dir = std::env::temp_dir().join(format!("ka_hgis_v16_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let opts = BoreholeMapOptions {
        radius_meters: 5000,
        marker_map: true,
        alluvial_isopach_map: true,
        fill_thickness_map: true,
        bedrock_depth_map: true,
        // 4-B 를 켜 5장 + 등층후선 1장이 모두 만들어지는지 본다.
        bedrock_elev_map: true,
        groundwater_map: true,
    };
    let bundle = generate_borehole_gis_maps(&holes, &kigam, WorkCrs::Epsg5187, &dir, &opts);
    let Ok(bundle) = bundle else {
        let _ = std::fs::remove_dir_all(&dir);
        return GateResult {
            gate_id: "V16",
            title_ko: "시추 5종 IDW GeoTIFF · 0.5m 등층후선 GPKG 생성",
            passed: false,
            detail: "generate_borehole_gis_maps returned Err".to_owned(),
        };
    };

    // C++ `:244-246` — 180×180 격자, cell >= 5.0.
    let grid_ok = bundle.grid.width == 180
        && bundle.grid.height == 180
        && bundle.grid.cell_size >= 5.0
        && bundle.grid.cell_size.is_finite();

    // C++ `:188` 마커 + `:251-291` 다섯 래스터 + 등층후선 = 7.
    let count_ok = bundle.created_count == 7;

    // 5장의 Float32 GeoTIFF 가 실제로 있고, 각각 TIFF 매직과 GeoKey 를 담는다.
    let tifs: Vec<&std::path::PathBuf> = bundle
        .written_files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "tif"))
        .collect();
    let mut tiff_ok = tifs.len() == 5;
    for p in &tifs {
        match std::fs::read(p) {
            Ok(bytes) => {
                let magic = bytes.starts_with(b"II*\0");
                let has_scale = bytes.windows(2).any(|w| w == 33550_u16.to_le_bytes());
                let has_tie = bytes.windows(2).any(|w| w == 33922_u16.to_le_bytes());
                let has_key = bytes.windows(2).any(|w| w == 34735_u16.to_le_bytes());
                // Float32 SampleFormat(339)=3 이 있어야 진짜 실수 래스터다.
                let has_float = bytes.windows(2).any(|w| w == 339_u16.to_le_bytes());
                if !(magic && has_scale && has_tie && has_key && has_float) {
                    tiff_ok = false;
                }
            }
            Err(_) => tiff_ok = false,
        }
    }

    // 등층후선 GPKG: SQLite 매직 + GPKG application_id + 레이어 이름.
    let gpkg_path = dir.join(FILE_ISOPACH_GPKG);
    let gpkg_ok = std::fs::read(&gpkg_path).is_ok_and(|bytes| {
        let sqlite_magic = bytes.starts_with(b"SQLite format 3\0");
        let has_gpkg_id = bytes
            .windows(4)
            .any(|w| w == strata_paleo::borehole_gpkg_geom::GPKG_APPLICATION_ID.to_be_bytes());
        sqlite_magic && has_gpkg_id
    });

    // 2공 미만이면 아무것도 만들지 않는다(`:226`).
    let short_ok = generate_borehole_gis_maps(
        &holes[..BOREHOLE_MIN_POINTS - 1],
        &[],
        WorkCrs::Epsg5187,
        &dir,
        &opts,
    )
    .is_ok_and(|b| b.created_count == usize::from(opts.marker_map) && b.written_files.is_empty());

    let _ = std::fs::remove_dir_all(&dir);

    GateResult {
        gate_id: "V16",
        title_ko: "시추 5종 IDW GeoTIFF · 0.5m 등층후선 GPKG 생성",
        passed: grid_ok && count_ok && tiff_ok && gpkg_ok && short_ok,
        detail: format!(
            "grid={}x{} cell={:.1}m created={} tifs={} gpkg={}",
            bundle.grid.width,
            bundle.grid.height,
            bundle.grid.cell_size,
            bundle.created_count,
            tifs.len(),
            if gpkg_ok { "ok" } else { "bad" }
        ),
    }
}

/// V17: 시추 도면이 **화면에 실제로 칠해지는지** 확인한다 (`d7afcc8` Phase 3 후속).
///
/// V16 은 파일이 만들어졌는지만 본다. 그래서 램프가 계산만 되고 지도에 안 칠해져도
/// V16 은 통과했다 — 실제로 그런 상태로 한동안 초록불이었다. 이 게이트는 그 구멍을 막는다:
/// 격자가 번들에 남아 있는지, 램프로 칠했을 때 불투명 픽셀이 나오는지,
/// C++ `setOpacity(0.72)` 알파가 지켜지는지 본다.
#[must_use]
pub fn v17_borehole_maps_are_painted_gate() -> GateResult {
    use strata_net::geoinfo_borehole::BoreholeRecord;
    use strata_paleo::borehole_color_ramp::{rasterize_ramp_rgba, BOREHOLE_RASTER_OPACITY};
    use strata_paleo::borehole_map_options::BoreholeMapOptions;
    use strata_paleo::borehole_maps::generate_borehole_gis_maps;

    fn hole(code: &str, x: f64, y: f64, fill: f64, alluv: f64, weath: f64) -> BoreholeRecord {
        BoreholeRecord {
            hole_code: code.to_owned(),
            tm_x: x,
            tm_y: y,
            altitude_m: 40.0,
            fill_depth_m: fill,
            alluvial_from_m: fill,
            alluvial_to_m: fill + alluv,
            alluvial_thickness_m: alluv,
            weathered_depth_m: weath,
            water_level_m: 2.4,
            dominant_uscs: "CL".to_owned(),
            paleo_hint: String::new(),
            layers: Vec::new(),
        }
    }

    let holes = [
        hole("BH-01", 218_500.0, 361_400.0, 1.2, 4.2, 5.4),
        hole("BH-02", 218_900.0, 361_600.0, 0.8, 3.1, -1.0),
        hole("BH-03", 219_300.0, 361_200.0, 2.0, 6.5, 8.5),
    ];
    let dir = std::env::temp_dir().join(format!("ka_hgis_v17_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let opts = BoreholeMapOptions {
        radius_meters: 5000,
        marker_map: true,
        alluvial_isopach_map: true,
        fill_thickness_map: true,
        bedrock_depth_map: true,
        bedrock_elev_map: true,
        groundwater_map: true,
    };

    let result = generate_borehole_gis_maps(&holes, &[], WorkCrs::Epsg5187, &dir, &opts);
    let _ = std::fs::remove_dir_all(&dir);

    let Ok(bundle) = result else {
        return GateResult {
            gate_id: "V17",
            title_ko: "시추 도면 화면 칠하기(램프 래스터화 · 알파 0.72)",
            passed: false,
            detail: "generate_borehole_gis_maps returned Err".to_owned(),
        };
    };

    // 1) 칠할 격자가 번들에 남아 있어야 한다(이게 없으면 화면이 빈다).
    let ramp_layers: Vec<&strata_paleo::borehole_maps::BoreholeMapLayerResult> =
        bundle.layers.iter().filter(|l| l.ramp.is_some()).collect();
    let grids_ok = bundle.painted_grids.len() == ramp_layers.len() && !bundle.painted_grids.is_empty();

    // 2) 각 격자가 램프로 칠해져 불투명 픽셀을 낸다.
    let expected_alpha = (BOREHOLE_RASTER_OPACITY * 255.0).round() as u8;
    let mut painted_ok = grids_ok;
    let mut visible_total = 0usize;
    for (grid, layer) in bundle.painted_grids.iter().zip(ramp_layers.iter()) {
        let Some(ramp) = layer.ramp.as_ref() else {
            painted_ok = false;
            continue;
        };
        let rgba = rasterize_ramp_rgba(grid, ramp);
        let visible = rgba.iter().filter(|p| p[3] > 0).count();
        if visible == 0 {
            painted_ok = false;
        }
        // 보이는 픽셀의 알파는 C++ `setOpacity(0.72)` 값과 같아야 한다.
        if rgba.iter().filter(|p| p[3] > 0).any(|p| p[3] != expected_alpha) {
            painted_ok = false;
        }
        visible_total += visible;
    }

    GateResult {
        gate_id: "V17",
        title_ko: "시추 도면 화면 칠하기(램프 래스터화 · 알파 0.72)",
        passed: grids_ok && painted_ok && expected_alpha == 184,
        detail: format!(
            "grids={} ramps={} alpha={} visible_px={}",
            bundle.painted_grids.len(),
            ramp_layers.len(),
            expected_alpha,
            visible_total
        ),
    }
}
