//! C++ `GeoInfoBoreholeMaps.cpp` (`d7afcc8`) — 시추 전용 5종 GIS 도면 생성 엔진 이식.
//!
//! C++ 는 QGIS 프로젝트·캔버스에 레이어를 직접 등록하지만(`addIdwMap`, `:45-81`),
//! Rust 쪽은 레이어 트리가 없는 라이브러리 계층이므로 **파일 산출**만 담당하고,
//! 레이어 등록은 앱 계층(`strata-app`)이 [`BoreholeMapBundle`] 을 받아 처리한다.
//! 계산 규칙·파일명·래스터 범위·컬러 램프는 C++ 원문 그대로다.

use std::path::{Path, PathBuf};

use strata_net::geoinfo_borehole::BoreholeRecord;
use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};

use crate::borehole_color_ramp::{
    alluvial_isopach_ramp, bedrock_depth_ramp, bedrock_elev_ramp, fill_thickness_ramp,
    groundwater_ramp, BoreholeColorRamp,
};
use crate::borehole_geotiff::write_float32_geotiff;
use crate::borehole_gpkg::write_isopach_contours_gpkg;
use crate::borehole_idw::{
    idw_interpolate_grid_k_nearest, BOREHOLE_IDW_K_NEAREST, BOREHOLE_IDW_POWER,
};
use crate::borehole_isopach::extract_all_isopach_segments;
use crate::borehole_map_accumulate::{
    accumulate_boreholes, accumulate_groundwater, bedrock_elev_range, BoreholeAccumulation,
};
use crate::borehole_map_grid::{build_borehole_grid, GeoBounds};
use crate::borehole_map_options::BoreholeMapOptions;
use crate::terrain_edt_tpi::GridTransform;

pub use crate::borehole_map_accumulate::{
    bedrock_elev_range as bedrock_elev_min_max, bedrock_gl_with_fallback,
    BOREHOLE_BEDROCK_ELEV_MIN_RANGE_M, BOREHOLE_WATER_MIN_DEPTH_M,
};
pub use crate::borehole_map_files::{
    BOREHOLE_MIN_POINTS, FILE_ALLUVIAL_ISOPACH, FILE_BEDROCK_DEPTH, FILE_BEDROCK_ELEV,
    FILE_FILL_THICKNESS, FILE_GROUNDWATER, FILE_ISOPACH_GPKG, TITLE_ALLUVIAL_ISOPACH,
    TITLE_BEDROCK_DEPTH, TITLE_BEDROCK_ELEV, TITLE_BOREHOLE_MARKER, TITLE_FILL_THICKNESS,
    TITLE_GROUNDWATER, TITLE_ISOPACH_CONTOUR,
};

/// C++ `generateBoreholeGisMaps` 반환값 — 생성 건수·산출 파일·격자.
#[derive(Debug, Clone, PartialEq)]
pub struct BoreholeMapBundle {
    /// C++ `int created` (`:184`) — 마커 + 성공한 래스터/벡터 레이어 수.
    pub created_count: usize,
    /// 실제로 기록된 파일 경로(마커 맵은 파일이 없으므로 제외).
    pub written_files: Vec<PathBuf>,
    /// C++ `tf` (`:246`) — 180×180 격자 변환. 시추공 2개 미만이면 폭·높이 0.
    pub grid: GridTransform,
    /// 도면별 생성 결과(C++ `:189-197` 의 토글별 생성/제거 대응).
    pub layers: Vec<BoreholeMapLayerResult>,
    /// 화면에 칠할 IDW 격자들(각각 180×180). `layers[i].ramp` 와 순서가 같다.
    ///
    /// C++ 은 `addMapLayer` 로 레이어를 프로젝트에 넣어 QGIS 가 격자를 들고 있지만
    /// (`GeoInfoBoreholeMaps.cpp:75`), Rust 에는 레이어 계층이 없어 여기 담아 넘긴다.
    /// 이것이 없으면 파일만 만들어지고 지도에는 아무 색도 칠해지지 않는다.
    pub painted_grids: Vec<Vec<f64>>,
}

/// 한 도면의 생성 결과.
#[derive(Debug, Clone, PartialEq)]
pub struct BoreholeMapLayerResult {
    /// C++ `:54` 의 레이어 표시명.
    pub title: String,
    /// 생성 성공 여부.
    pub created: bool,
    /// 이 레이어가 쓴 파일(마커 맵은 `None`).
    pub file: Option<PathBuf>,
    /// 이 레이어가 쓴 컬러 램프(마커/벡터 레이어는 `None`).
    pub ramp: Option<BoreholeColorRamp>,
}

/// C++ `generateBoreholeGisMaps(...)` (`GeoInfoBoreholeMaps.cpp:179-294`) 의 계산부.
///
/// `holes` 는 이미 작업 좌표계로 변환된 시추공, `kigam_wells` 는 `[x, y, water_gl_m]`
/// (`:235` 의 `water_gl_m` 속성) 이다.
pub fn generate_borehole_gis_maps(
    holes: &[BoreholeRecord],
    kigam_wells: &[[f64; 3]],
    work_crs: WorkCrs,
    out_dir: &Path,
    opts: &BoreholeMapOptions,
) -> StrataResult<BoreholeMapBundle> {
    opts.validate()?;

    // C++ `:203-225` 시추공 순회 누적
    let accumulation = accumulate_boreholes(holes);
    // C++ `:226` — 유효 시추공 2개 미만이면 마커 말고는 아무 도면도 만들지 않는다.
    if accumulation.points.len() < BOREHOLE_MIN_POINTS {
        return Ok(BoreholeMapBundle {
            created_count: usize::from(opts.marker_map),
            written_files: Vec::new(),
            grid: empty_grid(),
            layers: Vec::new(),
            painted_grids: Vec::new(),
        });
    }

    // C++ `:242-246` 패딩·셀·180×180 격자
    let bounds = GeoBounds::from_points(&accumulation.points)
        .ok_or_else(|| StrataError::InvalidInput("시추공 좌표 경계를 계산할 수 없습니다".into()))?;
    let (tf, _pad) = build_borehole_grid(&bounds);

    if !out_dir.as_os_str().is_empty() {
        std::fs::create_dir_all(out_dir)?;
    }

    let mut bundle = BoreholeMapBundle {
        // C++ `:188` — 마커 맵은 레이어 표시만 켜고 `++created`
        created_count: usize::from(opts.marker_map),
        written_files: Vec::new(),
        grid: tf,
        layers: Vec::new(),
        painted_grids: Vec::new(),
    };
    if opts.marker_map {
        bundle.layers.push(BoreholeMapLayerResult {
            title: TITLE_BOREHOLE_MARKER.to_owned(),
            created: true,
            file: None,
            ramp: None,
        });
    }

    write_enabled_maps(&mut bundle, &accumulation, kigam_wells, work_crs, out_dir, opts)?;
    Ok(bundle)
}

/// C++ `:251-291` 의 토글별 분기를 순서대로 실행한다.
fn write_enabled_maps(
    bundle: &mut BoreholeMapBundle,
    acc: &BoreholeAccumulation,
    kigam_wells: &[[f64; 3]],
    work_crs: WorkCrs,
    out_dir: &Path,
    opts: &BoreholeMapOptions,
) -> StrataResult<()> {
    let tf = bundle.grid;

    // C++ `:251-263` ② 충적층 두께 + 등층후선
    if opts.alluvial_isopach_map {
        if let Some(grid) = write_idw_layer(
            bundle,
            &tf,
            &acc.points,
            &acc.alluvial,
            work_crs,
            out_dir,
            FILE_ALLUVIAL_ISOPACH,
            TITLE_ALLUVIAL_ISOPACH,
            alluvial_isopach_ramp(),
        )? {
            // C++ `:260-262` — 래스터가 성공해야 등층후선을 만든다 (`gAlluv` 전달).
            let segments = extract_all_isopach_segments(&tf, &grid);
            let gpkg = out_dir.join(FILE_ISOPACH_GPKG);
            write_isopach_contours_gpkg(&gpkg, work_crs.epsg_code() as i32, &segments)?;
            bundle.created_count += 1;
            bundle.written_files.push(gpkg.clone());
            bundle.layers.push(BoreholeMapLayerResult {
                title: TITLE_ISOPACH_CONTOUR.to_owned(),
                created: true,
                file: Some(gpkg),
                ramp: None,
            });
        }
    }

    // C++ `:264-270` ③ 성토·매립층 두께
    if opts.fill_thickness_map {
        write_idw_layer(
            bundle,
            &tf,
            &acc.points,
            &acc.fill,
            work_crs,
            out_dir,
            FILE_FILL_THICKNESS,
            TITLE_FILL_THICKNESS,
            fill_thickness_ramp(),
        )?;
    }

    // C++ `:271-276` ④-A 기반암 출현 깊이
    if opts.bedrock_depth_map {
        write_idw_layer(
            bundle,
            &tf,
            &acc.points,
            &acc.bedrock_gl,
            work_crs,
            out_dir,
            FILE_BEDROCK_DEPTH,
            TITLE_BEDROCK_DEPTH,
            bedrock_depth_ramp(),
        )?;
    }

    // C++ `:277-285` ④-B 기반암 해발고도 (min/max 클램프)
    if opts.bedrock_elev_map {
        let (mn, mx) = bedrock_elev_range(&acc.bedrock_el);
        write_idw_layer(
            bundle,
            &tf,
            &acc.points,
            &acc.bedrock_el,
            work_crs,
            out_dir,
            FILE_BEDROCK_ELEV,
            TITLE_BEDROCK_ELEV,
            bedrock_elev_ramp(mn, mx),
        )?;
    }

    // C++ `:286-291` ⑤ 지하수위 — 결합점 2개 이상일 때만
    if opts.groundwater_map {
        let water = accumulate_groundwater(acc, kigam_wells);
        if water.points.len() >= BOREHOLE_MIN_POINTS {
            write_idw_layer(
                bundle,
                &tf,
                &water.points,
                &water.values,
                work_crs,
                out_dir,
                FILE_GROUNDWATER,
                TITLE_GROUNDWATER,
                groundwater_ramp(),
            )?;
        }
    }
    Ok(())
}

/// 한 IDW 래스터를 계산·기록하고 번들에 반영한다. 성공하면 계산된 격자를 돌려준다.
#[allow(clippy::too_many_arguments)]
fn write_idw_layer(
    bundle: &mut BoreholeMapBundle,
    tf: &GridTransform,
    points: &[[f64; 2]],
    values: &[f64],
    work_crs: WorkCrs,
    out_dir: &Path,
    file_name: &str,
    title: &str,
    ramp: BoreholeColorRamp,
) -> StrataResult<Option<Vec<f64>>> {
    if points.len() != values.len() || points.is_empty() {
        bundle.layers.push(BoreholeMapLayerResult {
            title: title.to_owned(),
            created: false,
            file: None,
            ramp: None,
        });
        return Ok(None);
    }
    let pts_xyv: Vec<[f64; 3]> = points
        .iter()
        .zip(values.iter())
        .map(|(p, &v)| [p[0], p[1], v])
        .collect();
    // C++ `addIdwMap` (`:51`) — k=8, power=2.0
    let grid = idw_interpolate_grid_k_nearest(
        tf,
        &pts_xyv,
        BOREHOLE_IDW_K_NEAREST,
        BOREHOLE_IDW_POWER,
    );

    let path = out_dir.join(file_name);
    // C++ `QFile::remove(tifPath)` (`:57`) — 같은 이름 파일을 지우고 다시 쓴다.
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    write_float32_geotiff(&path, tf, work_crs.epsg_code(), &grid)?;
    // C++ 은 QGIS 가 레이어를 직접 들고 있어 격자가 프로젝트에 남는다(`:75` addMapLayer).
    // Rust 는 레이어 계층이 없으므로 화면에 칠할 수 있도록 격자를 번들에 함께 담는다.
    bundle.painted_grids.push(grid.clone());
    bundle.created_count += 1;
    bundle.written_files.push(path.clone());
    bundle.layers.push(BoreholeMapLayerResult {
        title: title.to_owned(),
        created: true,
        file: Some(path),
        ramp: Some(ramp),
    });
    Ok(Some(grid))
}

/// C++ `:226` 조기 반환 시 돌려줄 빈 격자(폭·높이 0).
#[must_use]
fn empty_grid() -> GridTransform {
    GridTransform {
        origin_x: 0.0,
        origin_y: 0.0,
        cell_size: 0.0,
        width: 0,
        height: 0,
    }
}
