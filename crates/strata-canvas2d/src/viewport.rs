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
}
