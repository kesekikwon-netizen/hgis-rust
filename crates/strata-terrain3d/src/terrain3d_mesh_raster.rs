//! 지형 메시 삼각형 래스터화(원래 `render_terrain3d_rgba` 5단계).
//!
//! 함수 하나가 300줄을 넘어 이 단계를 그대로 옮겨 왔다.

use crate::terrain3d_zbuf::rasterize_triangle_zbuf;

/// 지형 상부 삼각형 메시를 Z-버퍼로 래스터화한다.
///
/// 원래 `render_terrain3d_rgba` 안 5단계를 그대로 옮긴 것이다.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rasterize_terrain_mesh(
    buf: &mut [u8],
    zbuf: &mut [f32],
    w: usize,
    h: usize,
    proj_pts: &[[f32; 3]],
    vert_cols: &[[u8; 4]],
    cols: usize,
    rows: usize,
) {
    for r in 0..(rows - 1) {
        for c in 0..(cols - 1) {
            let i0 = r * cols + c;
            let i1 = i0 + 1;
            let i2 = i0 + cols;
            let i3 = i2 + 1;

            rasterize_triangle_zbuf(
                buf,
                zbuf,
                w,
                h,
                proj_pts[i0],
                proj_pts[i2],
                proj_pts[i1],
                vert_cols[i0],
                vert_cols[i2],
                vert_cols[i1],
            );
            rasterize_triangle_zbuf(
                buf,
                zbuf,
                w,
                h,
                proj_pts[i1],
                proj_pts[i2],
                proj_pts[i3],
                vert_cols[i1],
                vert_cols[i2],
                vert_cols[i3],
            );
        }
    }
}
