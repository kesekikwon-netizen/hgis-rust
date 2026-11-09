//! 하늘·지평선 배경 채우기(원래 `render_terrain3d_rgba` 1단계).
//!
//! 함수 하나가 300줄을 넘어 이 단계를 그대로 옮겨 왔다.

use crate::atmosphere_agx::nishita_sky_zenith_horizon_rgb;

/// 하늘·지평선 배경을 채우고, 클램프된 카메라 피치(도)를 돌려준다.
///
/// 원래 `render_terrain3d_rgba` 안 1단계를 그대로 옮긴 것이다.
pub(crate) fn paint_sky_background(
    buf: &mut [u8],
    w: usize,
    h: usize,
    pitch_deg: f32,
) -> f32 {
    let (zenith, horizon) = nishita_sky_zenith_horizon_rgb(48.0);
    let pitch_clamped = if pitch_deg.is_finite() {
        pitch_deg.clamp(8.0, 82.0)
    } else {
        42.0
    };
    let horizon_frac = (0.34 - (pitch_clamped - 35.0) * 0.0032).clamp(0.16, 0.48);
    let sky_rows = ((h as f32 * horizon_frac) as usize).clamp(1, h.saturating_sub(1));
    for y in 0..h {
        let off_row = y * w * 4;
        let (r, g, b) = if y < sky_rows {
            let t = (y as f64 / (sky_rows.max(1) as f64)).powf(0.85);
            (
                (f64::from(zenith[0]) * (1.0 - t) + f64::from(horizon[0]) * t) as u8,
                (f64::from(zenith[1]) * (1.0 - t) + f64::from(horizon[1]) * t) as u8,
                (f64::from(zenith[2]) * (1.0 - t) + f64::from(horizon[2]) * t) as u8,
            )
        } else {
            let t = ((y - sky_rows) as f64 / ((h - sky_rows).max(1) as f64)).clamp(0.0, 1.0);
            (
                (f64::from(horizon[0]) * (1.0 - t * 0.32) + 198.0 * (t * 0.32)) as u8,
                (f64::from(horizon[1]) * (1.0 - t * 0.32) + 204.0 * (t * 0.32)) as u8,
                (f64::from(horizon[2]) * (1.0 - t * 0.32) + 210.0 * (t * 0.32)) as u8,
            )
        };
        for x in 0..w {
            let off = off_row + x * 4;
            buf[off..off + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
    pitch_clamped
}
