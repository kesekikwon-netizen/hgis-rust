//! 삼각형 Z-버퍼 래스터화.
//!
//! `viewport3d_renderer` 에서 분리했다.

#[allow(clippy::too_many_arguments)]
pub(crate) fn rasterize_triangle_zbuf(
    buf: &mut [u8],
    zbuf: &mut [f32],
    w: usize,
    h: usize,
    p0: [f32; 3],
    p1: [f32; 3],
    p2: [f32; 3],
    c0: [u8; 4],
    c1: [u8; 4],
    c2: [u8; 4],
) {
    if p0[2] <= 0.2 || p1[2] <= 0.2 || p2[2] <= 0.2 {
        return;
    }
    let area = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1]);
    if area.abs() < 1e-3 {
        return;
    }
    let min_x = p0[0].min(p1[0]).min(p2[0]).floor().max(0.0) as usize;
    let max_x = (p0[0].max(p1[0]).max(p2[0]).ceil().max(0.0) as usize).min(w.saturating_sub(1));
    let min_y = p0[1].min(p1[1]).min(p2[1]).floor().max(0.0) as usize;
    let max_y = (p0[1].max(p1[1]).max(p2[1]).ceil().max(0.0) as usize).min(h.saturating_sub(1));
    if min_x > max_x || min_y > max_y {
        return;
    }
    let inv_area = area.recip();
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        let row_idx = y * w;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let w1 = ((px - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (py - p0[1])) * inv_area;
            let w2 = ((p1[0] - p0[0]) * (py - p0[1]) - (px - p0[0]) * (p1[1] - p0[1])) * inv_area;
            let w0 = 1.0 - w1 - w2;
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let depth = w0 * p0[2] + w1 * p1[2] + w2 * p2[2];
            let z_slot = &mut zbuf[row_idx + x];
            if depth >= *z_slot {
                continue;
            }
            *z_slot = depth;
            let r = (w0 * f32::from(c0[0]) + w1 * f32::from(c1[0]) + w2 * f32::from(c2[0]))
                .round()
                .clamp(0.0, 255.0) as u8;
            let g = (w0 * f32::from(c0[1]) + w1 * f32::from(c1[1]) + w2 * f32::from(c2[1]))
                .round()
                .clamp(0.0, 255.0) as u8;
            let b = (w0 * f32::from(c0[2]) + w1 * f32::from(c1[2]) + w2 * f32::from(c2[2]))
                .round()
                .clamp(0.0, 255.0) as u8;
            let off = (row_idx + x) * 4;
            buf[off..off + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
}
