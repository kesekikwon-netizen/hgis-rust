use crate::pipeline::CrossSectionProfile;

#[must_use]
pub fn render_cross_section_rgba(
    profile: &CrossSectionProfile,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let w = width.max(16) as usize;
    let h = height.max(16) as usize;
    // 기본 종이 배경 (#FAF6EE)
    let mut buf = vec![0_u8; w * h * 4];
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&[250, 246, 238, 255]);
    }
    let n = profile.dist_m.len();
    if n < 2 {
        return buf;
    }

    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for (&zs, (&zp, &zb)) in profile
        .z_surface_curr_m
        .iter()
        .zip(profile.z_paleo_top_m.iter().zip(profile.z_bedrock_base_m.iter()))
    {
        min_z = min_z.min(zb).min(zp).min(zs);
        max_z = max_z.max(zs).max(zp).max(zb);
    }
    let z_span = (max_z - min_z).max(2.0);
    let z_to_y = |z: f64| -> usize {
        let norm = ((max_z - z) / z_span).clamp(0.05, 0.95);
        (norm * (h as f64)) as usize
    };

    for col in 0..w {
        let t = (col as f64 / (w.saturating_sub(1).max(1) as f64)) * ((n - 1) as f64);
        let idx = (t.floor() as usize).min(n - 2);
        let frac = t - (idx as f64);
        let lerp = |a: f64, b: f64| a + (b - a) * frac;

        let zs = lerp(profile.z_surface_curr_m[idx], profile.z_surface_curr_m[idx + 1]);
        let zp = lerp(profile.z_paleo_top_m[idx], profile.z_paleo_top_m[idx + 1]);
        let zb = lerp(profile.z_bedrock_base_m[idx], profile.z_bedrock_base_m[idx + 1]);

        let ys = z_to_y(zs).min(h - 1);
        let yp = z_to_y(zp).min(h - 1);
        let yb = z_to_y(zb).min(h - 1);

        for row in ys..h {
            let color = if row < yp {
                [156, 163, 175, 255] // 매립·표토층 (회갈색)
            } else if row < yb {
                [217, 119, 6, 230] // 충적·고지형 퇴적층 (황토색)
            } else {
                [120, 113, 108, 255] // 풍화대·기반암 (암회색)
            };
            let off = (row * w + col) * 4;
            buf[off..off + 4].copy_from_slice(&color);
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_section_rgba_renderer_draws_three_stratigraphic_horizons() {
        let profile = CrossSectionProfile {
            dist_m: vec![0.0, 50.0, 100.0],
            z_surface_curr_m: vec![45.0, 44.5, 44.0],
            z_paleo_top_m: vec![43.0, 42.5, 42.0],
            z_bedrock_base_m: vec![38.0, 37.5, 37.0],
        };
        let rgba = render_cross_section_rgba(&profile, 240, 120);
        assert_eq!(rgba.len(), 240 * 120 * 4);
    }
}
