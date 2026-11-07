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
}
