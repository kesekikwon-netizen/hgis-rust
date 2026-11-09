//! 트렌치 굴착 체적과 번호 매기기, DSM 안내 문구.
//!
//! `trench_profile3d` 에서 분리했다.

/// 3D 지형 원근 투영 기본 화각 (45.0°, C++ `HGIS_DEFAULT_FOV_DEGREES`)
pub const HGIS_DEFAULT_FOV_DEGREES: f32 = 45.0;

/// 트렌치 절개벽 최대 쿼드 수 (100,000, C++ `HGIS_MAX_TRENCH_MESH_QUADS`)
pub const HGIS_MAX_TRENCH_MESH_QUADS: usize = 100_000;

/// 트렌치 3D 지형 수직 절개 및 굴착 토량 산출 결과 (C++ `TrenchCutVolumeResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct TrenchCutVolumeResult {
    pub excavated_volume_m3: f64,
    pub trench_floor_area_m2: f64,
    pub max_excavation_depth_m: f64,
    pub wall_quad_count: usize,
    pub warning_message: Option<String>,
}

/// 평탄/일반 지형에서 트렌치 바닥 면적(`trench_area_m2`), 지표 표고 샘플(`surface_elevations_m`), 계획 바닥고(`target_floor_elev_m`)로부터 굴착 토량(`㎥`) 및 최대 굴착 심도 산출
/// (C++ `TrenchTerrain::carveTrenchIntoDem` 1:1 대응)
#[must_use]
pub fn carve_trench_volume_from_samples(
    surface_elevations_m: &[f64],
    trench_area_m2: f64,
    target_floor_elev_m: f64,
    perimeter_vertices: usize,
) -> TrenchCutVolumeResult {
    if surface_elevations_m.is_empty()
        || !trench_area_m2.is_finite()
        || trench_area_m2 <= 0.0
        || !target_floor_elev_m.is_finite()
    {
        return TrenchCutVolumeResult {
            excavated_volume_m3: 0.0,
            trench_floor_area_m2: 0.0,
            max_excavation_depth_m: 0.0,
            wall_quad_count: 0,
            warning_message: None,
        };
    }
    let cell_area = trench_area_m2 / (surface_elevations_m.len() as f64);
    let mut volume = 0.0_f64;
    let mut max_depth = 0.0_f64;
    for &z_surf in surface_elevations_m {
        if z_surf.is_finite() {
            let dz = (z_surf - target_floor_elev_m).max(0.0);
            volume += dz * cell_area;
            max_depth = max_depth.max(dz);
        }
    }
    let warning_message = if max_depth <= 0.0 {
        Some("굴착 바닥고가 지표면보다 높아 절개가 수행되지 않았습니다.".to_owned())
    } else {
        None
    };
    TrenchCutVolumeResult {
        excavated_volume_m3: volume,
        trench_floor_area_m2: trench_area_m2,
        max_excavation_depth_m: max_depth,
        wall_quad_count: perimeter_vertices.min(HGIS_MAX_TRENCH_MESH_QUADS),
        warning_message,
    }
}

/// 사면 상단 방위각(`upslope_azimuth_deg`)을 따라 높은 쪽부터 낮은 쪽으로, 같은 행에서는 좌에서 우로 시굴 트렌치 번호를 재부여
/// (C++ `TrenchTerrain::numberFromHighSide` 1:1 대응)
#[must_use]
pub fn number_trenches_from_high_side(
    trench_centers: &[[f64; 2]],
    upslope_azimuth_deg: f64,
    prefix: &str,
) -> Vec<(usize, String)> {
    let rad = upslope_azimuth_deg.to_radians();
    let ux = rad.sin();
    let uy = rad.cos();
    let vx = rad.cos();
    let vy = -rad.sin();

    let mut keys: Vec<(i64, f64, usize)> = trench_centers
        .iter()
        .enumerate()
        .map(|(idx, &[cx, cy])| {
            let along_cm = ((cx * ux + cy * uy) * 100.0).round() as i64;
            let across = cx * vx + cy * vy;
            (along_cm, across, idx)
        })
        .collect();

    keys.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.total_cmp(&b.1)));

    keys.into_iter()
        .enumerate()
        .map(|(ord, (_, _, orig_idx))| (orig_idx, format!("{prefix}{}", ord + 1)))
        .collect()
}

/// Copernicus GLO-30 30m 전지구 표고 배경 레이어 제목 (C++ `BasemapDsm::title`)
#[must_use]
pub const fn copernicus_dsm_title_ko() -> &'static str {
    "고도지형 (Copernicus 30 m)"
}

/// Copernicus GLO-30 30m 전지구 표고 배경 레이어 툴팁 (C++ `BasemapDsm::toolTip`)
#[must_use]
pub const fn copernicus_dsm_tooltip_ko() -> &'static str {
    "수치지형도 등고선이 없을 때 쓰는 전지구 30 m 표고 배경 (Copernicus GLO-30)"
}
