/// C++ `HGIS_LOD_MAX_DEPTH` (4) — 쿼드트리 최대 분할 깊이 (LOD 0~4)
pub const HGIS_LOD_MAX_DEPTH: u32 = 4;

/// C++ `HGIS_LOD_SCREEN_ERROR_PX` (2.0f) — LOD 분할 임계 화면 공간 오차(픽셀)
pub const HGIS_LOD_SCREEN_ERROR_PX: f32 = 2.0;

/// C++ `HGIS_BOREHOLE_RADIUS_DEFAULT` (1.0f) — 시추공 3D 기둥 기본 반경 (m)
pub const HGIS_BOREHOLE_RADIUS_DEFAULT: f32 = 1.0;

/// C++ `HGIS_CYLINDER_SEGMENTS` (16) — 시추공 원기둥 둘레 분할 각수 (16각 기둥)
pub const HGIS_CYLINDER_SEGMENTS: u32 = 16;

/// C++ `HGIS_SKIRT_DEPTH_RATIO` (0.05f) — 타일 한 변 크기 대비 T-Junction 방지 스커트 깊이 비율 (5%)
pub const HGIS_SKIRT_DEPTH_RATIO: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadPatch {
    pub min_xy: [f64; 2],
    pub size_m: f64,
    pub lod_level: u32,
    pub skirt_depth_m: f64,
}

/// C++ `KaQuadtreeLod::evaluateLodSubdivision` 화면 공간 오차(SSE) 계산 및 세분화 여부 판정
#[must_use]
pub fn evaluate_lod_subdivision(
    geometric_error_m: f32,
    camera_dist_m: f32,
    viewport_height_px: f32,
    fov_y_rad: f32,
    current_depth: u32,
    max_screen_error_px: f32,
) -> bool {
    if current_depth >= HGIS_LOD_MAX_DEPTH {
        return false;
    }
    let dist = camera_dist_m.max(1e-3);
    let tan_half_fov = (0.5 * fov_y_rad).tan().max(1e-4);
    let rho_px = (geometric_error_m / (dist * 2.0 * tan_half_fov)) * viewport_height_px.max(1.0);
    rho_px > max_screen_error_px.max(0.1)
}

/// C++ `KaQuadtreeLod::generateLodMeshWithSkirt` — 타일 외곽 테두리 정점의 스커트 바닥 Z 오프셋(`-0.05 * tile_size_m`) 산출
#[must_use]
pub fn compute_skirt_z_offset_m(tile_size_m: f64) -> f64 {
    -tile_size_m.abs() * HGIS_SKIRT_DEPTH_RATIO
}

/// C++ `KaBoreholeCylinder3D` 입력 지층 구간
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoreholeStratumSpan {
    pub from_depth_m: f32,
    pub to_depth_m: f32,
    pub color_rgba: [u8; 4],
}

/// C++ `CylinderInstanceData` — 16각 단위 원기둥 인스턴싱 데이터
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CylinderInstanceData {
    pub center_xyz: [f32; 3],
    pub radius_m: f32,
    pub height_m: f32,
    pub segments: u32,
    pub color_rgba: [u8; 4],
}

/// C++ `KaBoreholeCylinder3D::buildStratumCylinders` 1:1 대응:
/// 두께가 `0.0m` (`|to_depth - from_depth| < 1e-4`)인 불량/경계면 레코드는 Z-파이팅 방지를 위해 조용히 생략(Skip)한다.
#[must_use]
pub fn build_stratum_cylinders(
    collar_xyz: [f32; 3],
    strata: &[BoreholeStratumSpan],
    radius_m: f32,
) -> Vec<CylinderInstanceData> {
    let r = if radius_m.is_finite() && radius_m > 0.0 {
        radius_m
    } else {
        HGIS_BOREHOLE_RADIUS_DEFAULT
    };
    let mut out = Vec::with_capacity(strata.len());
    for s in strata {
        let thickness = s.to_depth_m - s.from_depth_m;
        if !thickness.is_finite() || thickness.abs() < 1e-4 || thickness <= 0.0 {
            continue;
        }
        let mid_depth = 0.5 * (s.from_depth_m + s.to_depth_m);
        out.push(CylinderInstanceData {
            center_xyz: [collar_xyz[0], collar_xyz[1], collar_xyz[2] - mid_depth],
            radius_m: r,
            height_m: thickness,
            segments: HGIS_CYLINDER_SEGMENTS,
            color_rgba: s.color_rgba,
        });
    }
    out
}

pub fn build_quadtree_patches(
    center_xy: [f64; 2],
    root_size_m: f64,
    max_depth: u32,
) -> Vec<QuadPatch> {
    let mut out = Vec::new();
    let half = root_size_m * 0.5;
    subdivide_node(
        [center_xy[0] - half, center_xy[1] - half],
        root_size_m,
        0,
        max_depth,
        center_xy,
        &mut out,
    );
    out
}

/// C++ `KaTerrain3dView::buildTerrainMesh` VRAM/버퍼 한도 초과 시 LOD 1단계 자동 하향 폴백
#[must_use]
pub fn build_quadtree_patches_with_vram_fallback(
    center_xy: [f64; 2],
    root_size_m: f64,
    requested_max_depth: u32,
    max_patch_budget: usize,
) -> (u32, Vec<QuadPatch>) {
    let patches = build_quadtree_patches(center_xy, root_size_m, requested_max_depth);
    if patches.len() > max_patch_budget.max(1) && requested_max_depth > 0 {
        let fallback_depth = requested_max_depth - 1;
        let fallback_patches = build_quadtree_patches(center_xy, root_size_m, fallback_depth);
        (fallback_depth, fallback_patches)
    } else {
        (requested_max_depth, patches)
    }
}

fn subdivide_node(
    min_xy: [f64; 2],
    size_m: f64,
    depth: u32,
    max_depth: u32,
    cam_xy: [f64; 2],
    out: &mut Vec<QuadPatch>,
) {
    let cx = min_xy[0] + size_m * 0.5;
    let cy = min_xy[1] + size_m * 0.5;
    let dist = ((cx - cam_xy[0]).powi(2) + (cy - cam_xy[1]).powi(2)).sqrt();
    if depth < max_depth && dist < size_m * 1.5 {
        let half = size_m * 0.5;
        for dy in [0.0, half] {
            for dx in [0.0, half] {
                subdivide_node(
                    [min_xy[0] + dx, min_xy[1] + dy],
                    half,
                    depth + 1,
                    max_depth,
                    cam_xy,
                    out,
                );
            }
        }
    } else {
        out.push(QuadPatch {
            min_xy,
            size_m,
            lod_level: depth,
            skirt_depth_m: size_m * 0.05,
        });
    }
}
