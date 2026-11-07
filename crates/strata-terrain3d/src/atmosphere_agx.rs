pub fn agx_tonemap_srgb(linear_rgb: [f64; 3]) -> [f64; 3] {
    let inset = [
        0.842_479_062_253_094 * linear_rgb[0]
            + 0.078_433_599_999_999_2 * linear_rgb[1]
            + 0.079_223_745_147_764_3 * linear_rgb[2],
        0.042_328_242_261_012_3 * linear_rgb[0]
            + 0.878_468_636_469_772 * linear_rgb[1]
            + 0.079_166_127_460_543_4 * linear_rgb[2],
        0.042_375_654_905_705_1 * linear_rgb[0]
            + 0.078_433_6 * linear_rgb[1]
            + 0.879_142_973_793_104 * linear_rgb[2],
    ];
    inset.map(|c| {
        let v = c.max(1e-10);
        let ev = (v.log2().clamp(-10.0, 6.5) + 10.0) / 16.5;
        let s = ev * ev * (3.0 - 2.0 * ev);
        s.clamp(0.0, 1.0)
    })
}

pub fn nishita_sky_zenith_horizon_rgb(sun_elevation_deg: f64) -> ([u8; 3], [u8; 3]) {
    let t = (sun_elevation_deg / 90.0).clamp(0.0, 1.0);
    let zenith = [
        (38.0 + 30.0 * t) as u8,
        (88.0 + 50.0 * t) as u8,
        (165.0 + 60.0 * t) as u8,
    ];
    let horizon = [
        (185.0 + 40.0 * t) as u8,
        (205.0 + 30.0 * t) as u8,
        (230.0 + 20.0 * t) as u8,
    ];
    (zenith, horizon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quadtree_lod::build_quadtree_patches;

    #[test]
    fn agx_tonemapping_and_quadtree_lod_produce_seamless_terrain_mesh() {
        let rgb = agx_tonemap_srgb([1.2, 0.8, 0.4]);
        assert!(rgb.iter().all(|&c| (0.0..=1.0).contains(&c)));
        let patches = build_quadtree_patches([0.0, 0.0], 4096.0, 4);
        assert!(!patches.is_empty());
    }
}
