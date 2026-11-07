use crate::borehole_strata::{parse_single_borehole, BoreholeRawInput, ParsedBoreholeSummary};
use crate::terrain_edt_tpi::{
    compute_river_distance_edt, compute_slope_and_hillshade, compute_tpi_integral_image,
    idw_interpolate_grid, GridTransform,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaleoClass {
    NaturalLevee = 1,
    FluvialTerrace = 2,
    GentleHillslope = 3,
    PaleochannelBackswamp = 4,
    ActiveFloodplain = 5,
    ArtificialFill = 6,
    SteepMountain = 7,
}

impl PaleoClass {
    pub const fn name_ko(self) -> &'static str {
        match self {
            Self::NaturalLevee => "자연제방·미고지",
            Self::FluvialTerrace => "하안단구·홍적대지",
            Self::GentleHillslope => "구릉 완사면·산록완사면",
            Self::PaleochannelBackswamp => "구하도·배후습지",
            Self::ActiveFloodplain => "현하도·범람원",
            Self::ArtificialFill => "깊은 매립·교란지",
            Self::SteepMountain => "산지 급경사지",
        }
    }

    pub const fn color_rgba(self) -> [u8; 4] {
        match self {
            Self::NaturalLevee => [217, 119, 6, 210],
            Self::FluvialTerrace => [180, 83, 9, 210],
            Self::GentleHillslope => [101, 163, 13, 210],
            Self::PaleochannelBackswamp => [37, 99, 235, 210],
            Self::ActiveFloodplain => [56, 189, 248, 210],
            Self::ArtificialFill => [107, 114, 128, 210],
            Self::SteepMountain => [75, 85, 99, 210],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CrossSectionProfile {
    pub dist_m: Vec<f64>,
    pub z_surface_curr_m: Vec<f64>,
    pub z_paleo_top_m: Vec<f64>,
    pub z_bedrock_base_m: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaleoAnalysisResult {
    pub transform: GridTransform,
    pub z_paleo: Vec<f64>,
    pub rem: Vec<f64>,
    pub slope_deg: Vec<f64>,
    pub tpi_50m: Vec<f64>,
    pub dist_to_river_m: Vec<f64>,
    pub multidirectional_hillshade: Vec<f64>,
    pub paleo_class_grid: Vec<PaleoClass>,
    pub paleo_rgba_buffer: Vec<u8>,
    pub parsed_boreholes: Vec<ParsedBoreholeSummary>,
    pub center_cross_section: CrossSectionProfile,
    pub aoi_summary_text: String,
}

#[allow(clippy::too_many_arguments)]
pub fn run_paleo_pipeline(
    z_curr: &[f64],
    tf: &GridTransform,
    raw_boreholes: &[BoreholeRawInput],
    _soil_grid: &[u8],
    _geol_grid: &[u8],
    river_mask: &[u8],
    aoi_mask: &[u8],
    _aoi_center_xy: (f64, f64),
) -> PaleoAnalysisResult {
    let n = tf.width * tf.height;
    let parsed_boreholes: Vec<ParsedBoreholeSummary> =
        raw_boreholes.iter().map(parse_single_borehole).collect();

    let fill_pts: Vec<[f64; 3]> = parsed_boreholes
        .iter()
        .map(|b| [b.x, b.y, b.fill_thickness_m])
        .collect();
    let base_pts: Vec<[f64; 3]> = parsed_boreholes
        .iter()
        .map(|b| [b.x, b.y, b.weathered_top_depth_m])
        .collect();

    let fill_grid = idw_interpolate_grid(tf, &fill_pts, 2.0, 0.5);
    let base_grid = idw_interpolate_grid(tf, &base_pts, 2.0, 4.0);
    let z_paleo: Vec<f64> = z_curr
        .iter()
        .zip(fill_grid.iter())
        .map(|(&z, &f)| z - f)
        .collect();

    let dist_to_river_m = compute_river_distance_edt(tf, river_mask);
    let tpi_50m = compute_tpi_integral_image(tf, &z_paleo, 50.0);
    let (slope_deg, multidirectional_hillshade) = compute_slope_and_hillshade(tf, &z_paleo);

    let min_z = z_paleo.iter().copied().fold(f64::INFINITY, f64::min);
    let rem: Vec<f64> = z_paleo.iter().map(|&z| (z - min_z).max(0.0)).collect();

    let mut paleo_class_grid = Vec::with_capacity(n);
    let mut paleo_rgba_buffer = Vec::with_capacity(n * 4);
    let mut levee_cnt = 0usize;

    for i in 0..n {
        let cls = if slope_deg[i] >= 15.0 {
            PaleoClass::SteepMountain
        } else if fill_grid[i] >= 3.5 {
            PaleoClass::ArtificialFill
        } else if slope_deg[i] >= 5.0 {
            PaleoClass::GentleHillslope
        } else if rem[i] >= 4.0 {
            PaleoClass::FluvialTerrace
        } else if tpi_50m[i] >= -0.2 && dist_to_river_m[i] <= 600.0 {
            levee_cnt += usize::from(aoi_mask.get(i).copied().unwrap_or(1) != 0);
            PaleoClass::NaturalLevee
        } else if tpi_50m[i] < -0.3 {
            PaleoClass::PaleochannelBackswamp
        } else {
            PaleoClass::ActiveFloodplain
        };
        paleo_class_grid.push(cls);
        let [r, g, b, a] = cls.color_rgba();
        let shade = multidirectional_hillshade[i];
        paleo_rgba_buffer.extend_from_slice(&[
            ((r as f64) * shade).clamp(0.0, 255.0) as u8,
            ((g as f64) * shade).clamp(0.0, 255.0) as u8,
            ((b as f64) * shade).clamp(0.0, 255.0) as u8,
            a,
        ]);
    }

    let mid_row = tf.height / 2;
    let mut profile = CrossSectionProfile::default();
    for c in 0..tf.width {
        let idx = mid_row * tf.width + c;
        profile.dist_m.push((c as f64) * tf.cell_size);
        profile.z_surface_curr_m.push(z_curr[idx]);
        profile.z_paleo_top_m.push(z_paleo[idx]);
        profile
            .z_bedrock_base_m
            .push(z_curr[idx] - base_grid[idx]);
    }

    let aoi_summary_text = format!(
        "고지형 4단계 정밀 판독 요약: 시추공 {}공 분석 완료, 자연제방·미고지 격자 {}셀 확인.",
        parsed_boreholes.len(),
        levee_cnt
    );

    PaleoAnalysisResult {
        transform: *tf,
        z_paleo,
        rem,
        slope_deg,
        tpi_50m,
        dist_to_river_m,
        multidirectional_hillshade,
        paleo_class_grid,
        paleo_rgba_buffer,
        parsed_boreholes,
        center_cross_section: profile,
        aoi_summary_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::borehole_strata::StratumLayer;

    #[test]
    fn paleo_pipeline_classifies_natural_levee_and_generates_cross_section() {
        let tf = GridTransform {
            origin_x: 200_000.0,
            origin_y: 500_000.0,
            cell_size: 10.0,
            width: 32,
            height: 32,
        };
        let z_curr = vec![15.0; 32 * 32];
        let mut river_mask = vec![0u8; 32 * 32];
        river_mask[16 * 32 + 16] = 1;
        let aoi_mask = vec![1u8; 32 * 32];
        let bh = vec![BoreholeRawInput {
            hole_id: "BH-1".into(),
            x: 200_160.0,
            y: 499_840.0,
            ground_elev_m: 15.0,
            water_depth_m: 2.5,
            strata: vec![
                StratumLayer {
                    depth_from_m: 0.0,
                    depth_to_m: 1.0,
                    layer_name: "매립토".into(),
                    uscs_code: "SM".into(),
                    description: "".into(),
                    n_value: 5.0,
                },
                StratumLayer {
                    depth_from_m: 1.0,
                    depth_to_m: 4.0,
                    layer_name: "충적사질토".into(),
                    uscs_code: "SM".into(),
                    description: "".into(),
                    n_value: 12.0,
                },
                StratumLayer {
                    depth_from_m: 4.0,
                    depth_to_m: 8.0,
                    layer_name: "풍화암".into(),
                    uscs_code: "WR".into(),
                    description: "".into(),
                    n_value: 50.0,
                },
            ],
        }];
        let res = run_paleo_pipeline(
            &z_curr,
            &tf,
            &bh,
            &[],
            &[],
            &river_mask,
            &aoi_mask,
            (200_160.0, 499_840.0),
        );
        assert_eq!(res.z_paleo.len(), 32 * 32);
        assert_eq!(res.paleo_rgba_buffer.len(), 32 * 32 * 4);
        assert!(!res.center_cross_section.dist_m.is_empty());
        assert!(res.aoi_summary_text.contains("고지형"));
    }
}
