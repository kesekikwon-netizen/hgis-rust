//! `PaleoTerrain3dTabState` 의 3D 미리보기·단면 표본 계산.
//!
//! `tabs_paleo_3d.rs` 가 300줄을 넘어 이 파일로 나눴다. 같은 `impl` 블록의 메서드이므로
//! 동작은 그대로이고, `PaleoTerrain3dTabState::rebuild_3d_preview` 경로도 그대로다.

use strata_terrain3d::viewport3d_renderer::render_terrain3d_rgba;
use crate::tabs_paleo_3d::PaleoTerrain3dTabState;

impl PaleoTerrain3dTabState {
    pub fn rebuild_3d_preview(&mut self) {
        self.camera_3d.z_exaggeration = self.z_exaggeration;
        if let Some(ref out) = self.last_paleo_output {
            self.terrain3d_rgba = render_terrain3d_rgba(
                &out.z_paleo,
                out.transform.width,
                out.transform.height,
                &self.camera_3d,
                320,
                200,
            );
        }
    }

    pub const HGIS_MIN_CUTLINE_LENGTH_M: f64 = 5.0;
    pub const HGIS_MAX_SECTION_SAMPLES: usize = 500;
    pub const HGIS_CORRIDOR_HALF_WIDTH_MIN: f64 = 50.0;
    pub const HGIS_CORRIDOR_HALF_WIDTH_MAX: f64 = 500.0;

    /// C++ `KaPaleoStudio::generateCrossSection` — 단면 절단선 최소 길이 5.0m 방어, 최대 샘플 개수 500개 제한, 시추공 투영 복도 반폭 `[50.0m, 500.0m]` 클램핑 1:1
    #[allow(clippy::type_complexity)]
    pub fn generate_cross_section_samples(
        cut_line: &[[f64; 2]],
        search_corridor_half_width: f64,
        max_sample_count: usize,
    ) -> Result<(Vec<[f64; 2]>, f64), (PaleoSectionError, String)> {
        if cut_line.len() < 2 {
            return Err((
                PaleoSectionError::CutLineTooShort,
                "단면 절단선 길이가 너무 짧습니다 (최소 5.0m 이상 작도 필요, 현재: 0.00 m)"
                    .to_owned(),
            ));
        }
        let mut total_len = 0.0_f64;
        for pair in cut_line.windows(2) {
            total_len += (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
        }
        if total_len < Self::HGIS_MIN_CUTLINE_LENGTH_M {
            return Err((
                PaleoSectionError::CutLineTooShort,
                format!(
                    "단면 절단선 길이가 너무 짧습니다 (최소 5.0m 이상 작도 필요, 현재: {total_len:.2} m)"
                ),
            ));
        }
        let corridor = search_corridor_half_width.clamp(
            Self::HGIS_CORRIDOR_HALF_WIDTH_MIN,
            Self::HGIS_CORRIDOR_HALF_WIDTH_MAX,
        );
        let cap = max_sample_count.clamp(2, Self::HGIS_MAX_SECTION_SAMPLES);
        let raw_samples = (total_len.floor() as usize + 1).min(cap);
        let start = cut_line[0];
        let end = cut_line[cut_line.len() - 1];
        let mut pts = Vec::with_capacity(raw_samples);
        for idx in 0..raw_samples {
            let t = idx as f64 / (raw_samples.saturating_sub(1).max(1) as f64);
            pts.push([
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ]);
        }
        Ok((pts, corridor))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaleoSectionError {
    CutLineTooShort,
}
