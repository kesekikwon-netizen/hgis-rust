#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportTransform {
    pub center_xy: [f64; 2],
    pub meters_per_pixel: f64,
    pub screen_size_px: [f64; 2],
}

impl ViewportTransform {
    pub const fn new(center_xy: [f64; 2], meters_per_pixel: f64, screen_size_px: [f64; 2]) -> Self {
        Self {
            center_xy,
            meters_per_pixel: if meters_per_pixel <= 0.0 {
                1.0
            } else {
                meters_per_pixel
            },
            screen_size_px,
        }
    }

    pub fn world_to_screen(&self, world_xy: [f64; 2]) -> [f64; 2] {
        let half_w = self.screen_size_px[0] * 0.5;
        let half_h = self.screen_size_px[1] * 0.5;
        let sx = half_w + (world_xy[0] - self.center_xy[0]) / self.meters_per_pixel;
        let sy = half_h - (world_xy[1] - self.center_xy[1]) / self.meters_per_pixel;
        [sx, sy]
    }

    pub fn screen_to_world(&self, screen_px: [f64; 2]) -> [f64; 2] {
        let half_w = self.screen_size_px[0] * 0.5;
        let half_h = self.screen_size_px[1] * 0.5;
        let wx = self.center_xy[0] + (screen_px[0] - half_w) * self.meters_per_pixel;
        let wy = self.center_xy[1] - (screen_px[1] - half_h) * self.meters_per_pixel;
        [wx, wy]
    }

    pub fn world_bbox(&self) -> [f64; 4] {
        let min = self.screen_to_world([0.0, self.screen_size_px[1]]);
        let max = self.screen_to_world([self.screen_size_px[0], 0.0]);
        [min[0], min[1], max[0], max[1]]
    }

    pub fn pan_by_pixels(&mut self, dx_px: f64, dy_px: f64) {
        self.center_xy[0] -= dx_px * self.meters_per_pixel;
        self.center_xy[1] += dy_px * self.meters_per_pixel;
    }

    pub fn zoom_at(&mut self, factor: f64, anchor_px: [f64; 2]) {
        let before = self.screen_to_world(anchor_px);
        self.meters_per_pixel = (self.meters_per_pixel * factor).clamp(0.01, 5000.0);
        let after = self.screen_to_world(anchor_px);
        self.center_xy[0] += before[0] - after[0];
        self.center_xy[1] += before[1] - after[1];
    }

    pub const HGIS_ZOOM_MIN_SCALE: f64 = 50.0;
    pub const HGIS_ZOOM_MAX_SCALE: f64 = 1_000_000.0;

    /// C++ `ViewportTransform::zoomAtScreenPos` — 줌 전후 마우스 커서 화면 좌표 아래의 월드 좌표 불변 조건 유지 및 축척 클램핑(`1:50` ~ `1:1,000,000`)
    pub fn zoom_at_screen_pos(&mut self, screen_cursor_pos: [f64; 2], zoom_factor: f64) {
        if zoom_factor <= 0.0 || !zoom_factor.is_finite() {
            return;
        }
        let world_before = self.screen_to_world(screen_cursor_pos);
        // 96 DPI 기준 1px = 0.0254 / 96.0 m
        let px_size_m = 0.0254 / 96.0;
        let min_mpp = Self::HGIS_ZOOM_MIN_SCALE * px_size_m;
        let max_mpp = Self::HGIS_ZOOM_MAX_SCALE * px_size_m;
        self.meters_per_pixel = (self.meters_per_pixel / zoom_factor).clamp(min_mpp, max_mpp);
        let world_after = self.screen_to_world(screen_cursor_pos);
        self.center_xy[0] += world_before[0] - world_after[0];
        self.center_xy[1] += world_before[1] - world_after[1];
    }
}

pub const HGIS_MAX_GRID_LINES: usize = 50;
pub const HGIS_GRID_TARGET_SPACING_PX: f64 = 100.0;
pub const HGIS_PRESET_GRID_INTERVALS: [f64; 5] = [10.0, 50.0, 100.0, 500.0, 1000.0];

/// C++ `KaMapRotationController::setRotationAngleDeg` — 임의 각도를 `[0.0°, 360.0°)` 구간으로 모듈로 정규화
#[must_use]
pub fn normalize_rotation_angle_deg(angle_deg: f64) -> f64 {
    if !angle_deg.is_finite() {
        return 0.0;
    }
    angle_deg.rem_euclid(360.0)
}

/// C++ `KaMapRotationController` — 회전 중심점 `(cx, cy)` 보존 2D 아핀 회전 변환
#[must_use]
pub fn rotate_point_around_center(pt: [f64; 2], center: [f64; 2], angle_deg: f64) -> [f64; 2] {
    let rad = normalize_rotation_angle_deg(angle_deg).to_radians();
    let (s, c) = rad.sin_cos();
    let dx = pt[0] - center[0];
    let dy = pt[1] - center[1];
    [center[0] + dx * c - dy * s, center[1] + dx * s + dy * c]
}

/// C++ `KaCoordinateGridOverlay::selectOptimalGridInterval` — `metersPerPixel * targetScreenSpacingPx`에 가장 가까운 표준 격자 간격(`10·50·100·500·1000m`) 자동 선택
#[must_use]
pub fn select_optimal_grid_interval(meters_per_pixel: f64, target_screen_spacing_px: f64) -> f64 {
    let mpp = if meters_per_pixel > 0.0 && meters_per_pixel.is_finite() {
        meters_per_pixel
    } else {
        1.0
    };
    let spacing_px = if target_screen_spacing_px > 0.0 && target_screen_spacing_px.is_finite() {
        target_screen_spacing_px
    } else {
        HGIS_GRID_TARGET_SPACING_PX
    };
    let ideal_m = mpp * spacing_px;
    let mut best = HGIS_PRESET_GRID_INTERVALS[0];
    let mut best_diff = f64::INFINITY;
    for &cand in &HGIS_PRESET_GRID_INTERVALS {
        let diff = (cand - ideal_m).abs();
        if diff < best_diff {
            best_diff = diff;
            best = cand;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_canvas_world_to_screen_roundtrip_preserves_submillimeter_precision() {
        let vp = ViewportTransform::new([200_000.0, 500_000.0], 1.0, [1920.0, 1080.0]);
        let screen = vp.world_to_screen([200_123.456, 500_654.321]);
        let world = vp.screen_to_world(screen);
        assert!((world[0] - 200_123.456).abs() < 1e-6);
        assert!((world[1] - 500_654.321).abs() < 1e-6);
    }

    #[test]
    fn test_viewport_zoom_cursor_invariance_at_400_300_2x_zoom_preserves_exact_world_coord() {
        let mut vp = ViewportTransform::new([400.0, 300.0], 1.0, [800.0, 600.0]);
        let cursor = [400.0, 300.0];
        let before = vp.screen_to_world(cursor);
        assert!((before[0] - 400.0).abs() < 1e-6);
        assert!((before[1] - 300.0).abs() < 1e-6);

        vp.zoom_at_screen_pos(cursor, 2.0);
        let after = vp.screen_to_world(cursor);
        assert!((after[0] - 400.0).abs() < 1e-6);
        assert!((after[1] - 300.0).abs() < 1e-6);

        // 비중앙 커서 (250, 180)에서도 2배 줌인 전후 월드 좌표 불변 검증
        let off_cursor = [250.0, 180.0];
        let off_before = vp.screen_to_world(off_cursor);
        vp.zoom_at_screen_pos(off_cursor, 2.0);
        let off_after = vp.screen_to_world(off_cursor);
        assert!((off_after[0] - off_before[0]).abs() < 1e-6);
        assert!((off_after[1] - off_before[1]).abs() < 1e-6);
    }

    #[test]
    fn test_angle_normalization_neg45_to_315_center_preservation_and_optimal_grid_interval_50m() {
        assert!((normalize_rotation_angle_deg(-45.0) - 315.0).abs() < 1e-9);
        let center = [200_000.0, 500_000.0];
        let rotated_center = rotate_point_around_center(center, center, -45.0);
        assert!((rotated_center[0] - center[0]).abs() < 1e-9);
        assert!((rotated_center[1] - center[1]).abs() < 1e-9);

        let pt = [200_100.0, 500_050.0];
        let fwd = rotate_point_around_center(pt, center, -45.0);
        let inv = rotate_point_around_center(fwd, center, 45.0);
        assert!((inv[0] - pt[0]).abs() < 1e-9);
        assert!((inv[1] - pt[1]).abs() < 1e-9);

        assert!((select_optimal_grid_interval(0.5, 100.0) - 50.0).abs() < 1e-9);
    }
}
