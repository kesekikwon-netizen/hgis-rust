//! 절단선 단면 생성과 단면 위 시추공 투영.
//!
//! `pipeline` 에서 분리했다.

use serde::{Deserialize, Serialize};
use crate::terrain_edt_tpi::GridTransform;

use crate::borehole_strata::{parse_single_borehole, BoreholeRawInput, ParsedBoreholeSummary};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CrossSectionProfile {
    pub dist_m: Vec<f64>,
    pub z_surface_curr_m: Vec<f64>,
    pub z_paleo_top_m: Vec<f64>,
    pub z_bedrock_base_m: Vec<f64>,
}

/// C++ `KaPaleoCrossSectionWidget::generateProfile` 절단선 상으로 직교 투영된 시추공 모델
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectedBoreholeOnProfile {
    pub hole_id: String,
    pub station_m: f64,
    pub offset_m: f64,
    pub ground_elev_m: f64,
    pub total_depth_m: f64,
    pub strata_count: usize,
    pub summary: ParsedBoreholeSummary,
}

pub type CutlineProfileResult =
    Result<(Vec<f64>, Vec<f64>, Vec<ProjectedBoreholeOnProfile>), &'static str>;

/// C++ `KaPaleoCrossSectionWidget::generateProfile(const QgsGeometry &cutLine, double searchBufferMeters = 100.0)`
pub fn generate_cutline_profile(
    cut_line: &[[f64; 2]],
    tf: &GridTransform,
    z_curr: &[f64],
    raw_boreholes: &[BoreholeRawInput],
    search_buffer_m: f64,
    abort_requested: Option<&std::sync::atomic::AtomicBool>,
) -> CutlineProfileResult {
    use std::sync::atomic::Ordering;

    if cut_line.len() < 2 {
        return Err("절단선은 최소 2개 이상의 정점이 필요합니다");
    }
    let start = cut_line[0];
    let end = cut_line[cut_line.len() - 1];
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let total_len = (dx * dx + dy * dy).sqrt();
    if total_len < 1e-3 {
        return Err("절단선 길이가 0에 가깝습니다");
    }

    let steps = (total_len.round() as usize).max(1);
    let mut stations = Vec::with_capacity(steps + 1);
    let mut elevations = Vec::with_capacity(steps + 1);
    for s in 0..=steps {
        if abort_requested.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("단면도 프로파일 생성이 중단되었습니다");
        }
        let t = (s as f64) / (steps as f64);
        let wx = start[0] + dx * t;
        let wy = start[1] + dy * t;
        let z = tf
            .world_to_cell(wx, wy)
            .and_then(|(r, c)| z_curr.get(r * tf.width + c).copied())
            .unwrap_or(15.0);
        stations.push(total_len * t);
        elevations.push(z);
    }

    let ux = dx / total_len;
    let uy = dy / total_len;
    let mut projected = Vec::new();
    for bh in raw_boreholes {
        if abort_requested.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("단면도 프로파일 생성이 중단되었습니다");
        }
        let vx = bh.x - start[0];
        let vy = bh.y - start[1];
        let along = vx * ux + vy * uy;
        let perp = (vx * uy - vy * ux).abs();
        if (0.0..=total_len).contains(&along) && perp <= search_buffer_m.max(1.0) {
            let total_depth_m = bh
                .strata
                .iter()
                .map(|st| st.depth_to_m)
                .fold(0.0_f64, f64::max);
            projected.push(ProjectedBoreholeOnProfile {
                hole_id: bh.hole_id.clone(),
                station_m: along,
                offset_m: perp,
                ground_elev_m: bh.ground_elev_m,
                total_depth_m,
                strata_count: bh.strata.len(),
                summary: parse_single_borehole(bh),
            });
        }
    }
    projected.sort_by(|a, b| {
        a.station_m
            .partial_cmp(&b.station_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok((stations, elevations, projected))
}
