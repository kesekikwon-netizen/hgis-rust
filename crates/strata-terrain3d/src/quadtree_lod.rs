#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadPatch {
    pub min_xy: [f64; 2],
    pub size_m: f64,
    pub lod_level: u32,
    pub skirt_depth_m: f64,
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
