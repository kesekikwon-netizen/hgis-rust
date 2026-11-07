use crate::terrain_edt_tpi::{idw_interpolate_grid, GridTransform};

pub fn synthesize_precision_dem_from_topographic_points(
    tf: &GridTransform,
    contour_pts_3d: &[[f64; 3]],
) -> Vec<f64> {
    idw_interpolate_grid(tf, contour_pts_3d, 2.0, 0.0)
}
