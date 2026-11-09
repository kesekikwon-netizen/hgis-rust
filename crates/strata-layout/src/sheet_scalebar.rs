//! `DrawingSheet` 의 축척자·좌표 격자 쪽.
//!
//! `impl DrawingSheet` 가 332줄이라 여기로 나눴다.

use crate::sheet_drawing::DrawingSheet;
use crate::sheet_geometry::SheetOrientation;

impl DrawingSheet {

    /// `LayoutService::applyMapCoordinateGrid` (`5362ef6`) — 지도에 미터 좌표격자가 켜져 있고 북쪽이 위(회전 0°)일 때 도면 격자를 지도 좌표격자와 같은 간격·같은 교점 오프셋으로 동기화
    pub fn apply_map_coordinate_grid(
        &mut self,
        map_grid_enabled: bool,
        map_rotation_deg: f64,
        step_m: f64,
        offset_xy_m: [f64; 2],
    ) -> bool {
        let north_up = (map_rotation_deg % 360.0).abs() < 1e-6;
        if map_grid_enabled && north_up && step_m > 0.0 {
            self.grid_enabled = true;
            self.grid_follow_map = true;
            self.grid_interval_m = step_m;
            self.grid_offset_xy_m = [
                offset_xy_m[0].rem_euclid(step_m),
                offset_xy_m[1].rem_euclid(step_m),
            ];
            true
        } else {
            self.grid_follow_map = false;
            false
        }
    }

    /// 지상 좌표 `world_xy` (m)를 현재 용지의 `map_frame` 내부 용지 밀리미터 좌표 `[x_mm, y_mm]`로 투영
    /// (`LayoutService::extentForPaperScale` 및 `layoutMapItemFromXy` 1:1 대응)
    #[must_use]
    pub fn world_to_sheet_mm(&self, center_world_xy: [f64; 2], world_xy: [f64; 2]) -> [f64; 2] {
        let scale = f64::from(self.scale_denominator.max(1));
        let mm_per_meter = 1000.0 / scale;
        let mc_x = self.map_frame.x_mm + self.map_frame.width_mm * 0.5;
        let mc_y = self.map_frame.y_mm + self.map_frame.height_mm * 0.5;
        [
            mc_x + (world_xy[0] - center_world_xy[0]) * mm_per_meter,
            mc_y - (world_xy[1] - center_world_xy[1]) * mm_per_meter,
        ]
    }

    /// C++ `PlateScaleBar::calculateDivisions(mapScale, targetWidthMm)` — 1-2-5 계열 정수 단위 축척막대 간격(m) 산출 1:1
    #[must_use]
    pub fn calculate_scale_bar_nice_interval_m(map_scale_denom: f64, target_width_mm: f64) -> f64 {
        Self::compute_nice_scale_bar_length(
            map_scale_denom,
            target_width_mm - 20.0,
            target_width_mm + 20.0,
        )
    }

    pub const HGIS_SCALEBAR_MIN_WIDTH_MM: f64 = 40.0;

    pub const HGIS_SCALEBAR_MAX_WIDTH_MM: f64 = 80.0;

    pub const HGIS_FALLBACK_SCALEBAR_LENGTH_M: f64 = 50.0;

    pub const HGIS_MAX_SCALEBAR_LENGTH_M: f64 = 100_000.0;

    pub const HGIS_NICE_MANTISSAS: [f64; 4] = [1.0, 2.0, 5.0, 10.0];

    /// C++ `KaScaleBarItem::computeNiceScaleBarLength` — 목표 폭 `[40.0mm, 80.0mm]` 및 `1·2·5·10` 가수 배열 기반 축척 막대 길이(m) 산출, 비정상 축척 시 `50.0m` 폴백
    #[must_use]
    pub fn compute_nice_scale_bar_length(
        map_scale_denom: f64,
        target_width_min_mm: f64,
        target_width_max_mm: f64,
    ) -> f64 {
        if map_scale_denom <= 1e-6 || !map_scale_denom.is_finite() {
            return Self::HGIS_FALLBACK_SCALEBAR_LENGTH_M;
        }
        let min_mm = target_width_min_mm.max(10.0);
        let max_mm = target_width_max_mm.max(min_mm);
        let mid_mm = 0.5 * (min_mm + max_mm);
        let raw_meters = map_scale_denom * (mid_mm / 1000.0);
        if raw_meters <= 1.0 {
            return 1.0;
        }
        let exp = raw_meters.log10().floor();
        let base = 10.0_f64.powf(exp);
        let mut best_len = Self::HGIS_FALLBACK_SCALEBAR_LENGTH_M;
        let mut best_dist = f64::INFINITY;
        for &m in &Self::HGIS_NICE_MANTISSAS {
            let cand_m = m * base;
            let bar_mm = (cand_m * 1000.0) / map_scale_denom;
            let in_range = (min_mm..=max_mm).contains(&bar_mm);
            let dist = if in_range {
                // 목표 범위 내에서는 5·2·1 정수 단위 중 mid_mm 이하에서 가장 큰 깔끔한 단위 우선
                if cand_m <= raw_meters {
                    raw_meters - cand_m
                } else {
                    (cand_m - raw_meters) + 1e3
                }
            } else {
                (bar_mm - mid_mm).abs() + 1e6
            };
            if dist < best_dist {
                best_dist = dist;
                best_len = cand_m;
            }
        }
        best_len.min(Self::HGIS_MAX_SCALEBAR_LENGTH_M)
    }

    /// C++ `KaLayoutComposer::setPaperOrientation` — 용지 가로/세로 전환 시 폭·높이 스왑 및 지도 중심 좌표 `(cx, cy)` 보존
    pub fn set_paper_orientation_preserving_center(
        &mut self,
        orientation: SheetOrientation,
        current_extent: [f64; 4],
    ) -> [f64; 4] {
        let cx = 0.5 * (current_extent[0] + current_extent[2]);
        let cy = 0.5 * (current_extent[1] + current_extent[3]);
        self.change_paper(self.paper_size, orientation);
        let scale = f64::from(self.scale_denominator.max(1));
        let half_w_m = (self.map_frame.width_mm / 1000.0) * scale * 0.5;
        let half_h_m = (self.map_frame.height_mm / 1000.0) * scale * 0.5;
        [cx - half_w_m, cy - half_h_m, cx + half_w_m, cy + half_h_m]
    }
}
