//! C++ `PaleoAnalysisRunner::buildNgiiDemFromTopographic` (`PaleoAnalysisRunner.cpp:153-215`) 1:1.
//!
//! 국토지리정보원 1:5,000 수치지형도 등고선(레이어 코드 `F001`/`F002`)과 표고점을 모아
//! IDW 로 정밀 DEM 격자를 만들고 `지형분석/국토지리원_DEM.tif` 로 쓴다.
//!
//! C++ 은 QGIS 레이어를 직접 훑지만 Rust 는 이미 메모리에 올린 등고선 점을 받는다.
//! **수치 규칙은 C++ 원문 그대로**다: 점 개수 상한 12,000, 표고 유효 범위
//! `-100 < z <= 3000`, 셀 크기 `max(5, span/240)`, 격자 `16..=360칸`, IDW k=10/p=2.0.

use std::path::{Path, PathBuf};

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};

use crate::borehole_geotiff::write_float32_geotiff;
use crate::terrain_edt_tpi::GridTransform;

/// C++ `PaleoAnalysisRunner.cpp:160` — 모으는 점 개수 상한.
pub const NGII_DEM_MAX_POINTS: usize = 12_000;

/// C++ `:196` — 이보다 적으면 DEM 을 만들지 않는다.
pub const NGII_DEM_MIN_POINTS: usize = 4;

/// C++ `:182` — 표고 유효 범위(하한 초과·상한 초과는 버린다).
pub const NGII_DEM_MIN_ELEV_M: f64 = -100.0;
pub const NGII_DEM_MAX_ELEV_M: f64 = 3_000.0;

/// C++ `:185` `densifyByDistance(40.0)`.
pub const NGII_DEM_DENSIFY_M: f64 = 40.0;

/// C++ `:200` 셀 크기 = `max(5.0, max(w,h)/240.0)`.
pub const NGII_DEM_MIN_CELL_M: f64 = 5.0;
pub const NGII_DEM_CELL_DIVISOR: f64 = 240.0;

/// C++ `:201-202` 격자 칸 수 상·하한.
pub const NGII_DEM_MIN_SIDE: usize = 16;
pub const NGII_DEM_MAX_SIDE: usize = 360;

/// C++ `:204` IDW 이웃 수·거듭제곱.
pub const NGII_DEM_IDW_K: usize = 10;
pub const NGII_DEM_IDW_POWER: f64 = 2.0;

/// C++ `:208` 출력 파일 이름.
pub const NGII_DEM_FILE_NAME: &str = "국토지리원_DEM.tif";

/// C++ `:205-207` 출력 폴더 이름(조사 폴더 아래).
pub const NGII_DEM_OUTPUT_DIR: &str = "지형분석";

/// 등고선 한 피처 — C++ 이 `QgsFeature` 에서 읽는 값만 담는다.
#[derive(Debug, Clone, PartialEq)]
pub struct NgiiContourFeature {
    /// 등고선 폴리라인 정점(작업 좌표계).
    pub vertices_xy: Vec<[f64; 2]>,
    /// 표고(m). C++ `:181` `elevation` 속성.
    pub elevation_m: f64,
    /// C++ `:176` `z_valid` 가 0 이면 버린다.
    pub z_valid: bool,
    /// C++ `:177-179` `Layer` 속성 — 비어 있지 않으면 `F001`/`F002` 로 시작해야 한다.
    pub cad_layer: String,
}

impl NgiiContourFeature {
    /// C++ `:176-182` 의 피처 단위 필터.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        if !self.z_valid {
            return false;
        }
        let code = self.cad_layer.trim();
        if !code.is_empty() && !code.starts_with("F001") && !code.starts_with("F002") {
            return false;
        }
        self.elevation_m.is_finite()
            && self.elevation_m > NGII_DEM_MIN_ELEV_M
            && self.elevation_m <= NGII_DEM_MAX_ELEV_M
    }
}

/// C++ `:158` `surveyExtentRadius(project, canvas, 5000.0)` 가 돌려주는 상자.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemExtent {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl DemExtent {
    #[must_use]
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    #[must_use]
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    #[must_use]
    pub fn contains(&self, p: [f64; 2]) -> bool {
        p[0] >= self.min_x && p[0] <= self.max_x && p[1] >= self.min_y && p[1] <= self.max_y
    }
}

/// C++ `:185-191` `densifyByDistance(40.0)` 후 짝수 번째 정점만 상자 안에서 모은다.
pub fn densify_and_sample_vertices(
    vertices: &[[f64; 2]],
    elevation_m: f64,
    spacing_m: f64,
    extent: &DemExtent,
    out: &mut Vec<[f64; 3]>,
    max_points: usize,
) {
    let step_m = if spacing_m > 0.0 { spacing_m } else { NGII_DEM_DENSIFY_M };
    let mut parity = 0_usize;
    let mut push = |p: [f64; 2], z: f64, out: &mut Vec<[f64; 3]>| {
        if out.len() >= max_points {
            return;
        }
        if parity % 2 == 0 && extent.contains(p) {
            out.push([p[0], p[1], z]);
        }
        parity += 1;
    };
    if vertices.len() < 2 {
        for &v in vertices {
            push(v, elevation_m, out);
        }
        return;
    }
    for pair in vertices.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let seg_len = dx.hypot(dy);
        // C++ `densifyByDistance` 는 간격마다 정점을 심고 끝점을 유지한다.
        let n = if seg_len > step_m {
            (seg_len / step_m).ceil() as usize
        } else {
            1
        };
        for i in 0..n {
            let t = i as f64 / n as f64;
            push([a[0] + dx * t, a[1] + dy * t], elevation_m, out);
            if out.len() >= max_points {
                return;
            }
        }
    }
    push(vertices[vertices.len() - 1], elevation_m, out);
}

/// C++ `:200-203` 격자 변환을 만든다.
#[must_use]
pub fn build_ngii_dem_grid(extent: &DemExtent) -> GridTransform {
    let cell = NGII_DEM_MIN_CELL_M.max(extent.width().max(extent.height()) / NGII_DEM_CELL_DIVISOR);
    let w = if cell > 0.0 {
        (extent.width() / cell).ceil() as i64
    } else {
        NGII_DEM_MIN_SIDE as i64
    };
    let h = if cell > 0.0 {
        (extent.height() / cell).ceil() as i64
    } else {
        NGII_DEM_MIN_SIDE as i64
    };
    let w = w.clamp(NGII_DEM_MIN_SIDE as i64, NGII_DEM_MAX_SIDE as i64) as usize;
    let h = h.clamp(NGII_DEM_MIN_SIDE as i64, NGII_DEM_MAX_SIDE as i64) as usize;
    GridTransform {
        origin_x: extent.min_x,
        origin_y: extent.max_y,
        cell_size: cell,
        width: w,
        height: h,
    }
}

/// C++ `:153-215` 전체 흐름. 성공하면 쓴 `.tif` 경로를 돌려준다.
pub fn build_ngii_dem_from_topographic(
    features: &[NgiiContourFeature],
    extent: &DemExtent,
    work_crs: WorkCrs,
    output_dir: &Path,
) -> StrataResult<PathBuf> {
    let mut pts: Vec<[f64; 3]> = Vec::with_capacity(NGII_DEM_MAX_POINTS);
    for f in features {
        if !f.is_usable() {
            continue;
        }
        let mut dense: Vec<[f64; 2]> = Vec::new();
        // C++ 은 정점마다 표고가 같은 등고선을 40m 간격으로 채운다.
        densify_into(&f.vertices_xy, NGII_DEM_DENSIFY_M, &mut dense);
        let z = f.elevation_m;
        let mut parity = 0_usize;
        for p in dense {
            if pts.len() >= NGII_DEM_MAX_POINTS {
                break;
            }
            if parity % 2 == 0 && extent.contains(p) {
                pts.push([p[0], p[1], z]);
            }
            parity += 1;
        }
        if pts.len() >= NGII_DEM_MAX_POINTS {
            break;
        }
    }
    if pts.len() < NGII_DEM_MIN_POINTS {
        return Err(StrataError::InvalidInput(
            "수치지형도 등고선 표고점이 부족합니다.".to_owned(),
        ));
    }

    let tf = build_ngii_dem_grid(extent);
    let grid = crate::borehole_idw::idw_interpolate_grid_k_nearest(
        &tf,
        &pts,
        NGII_DEM_IDW_K,
        NGII_DEM_IDW_POWER,
    );

    std::fs::create_dir_all(output_dir)?;
    let out = output_dir.join(NGII_DEM_FILE_NAME);
    if out.exists() {
        // C++ `:211` `QFile::remove(demOut)`
        std::fs::remove_file(&out)?;
    }
    write_float32_geotiff(&out, &tf, work_crs.epsg_code(), &grid)?;
    Ok(out)
}

/// 선분을 `spacing_m` 간격으로 채운다(C++ `densifyByDistance` 대응).
fn densify_into(vertices: &[[f64; 2]], spacing_m: f64, out: &mut Vec<[f64; 2]>) {
    if vertices.len() < 2 {
        out.extend_from_slice(vertices);
        return;
    }
    for pair in vertices.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        let n = if len > spacing_m && spacing_m > 0.0 {
            (len / spacing_m).ceil() as usize
        } else {
            1
        };
        for i in 0..n {
            let t = i as f64 / n as f64;
            out.push([a[0] + dx * t, a[1] + dy * t]);
        }
    }
    if let Some(last) = vertices.last() {
        out.push(*last);
    }
}
