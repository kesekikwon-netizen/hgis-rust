#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportTransform {
    pub center_xy: [f64; 2],
    pub meters_per_pixel: f64,
    pub screen_size_px: [f64; 2],
}

/// 지도 좌표 ↔ 화면 픽셀 **아핀 변환기**. C++ `QgsMapToPixel` 자리.
///
/// ## 문서 Phase 1 이 요구하는 것
///
/// 설계 문서는 회전각 θ를 포함한 순방향·역방향 행렬식을 제시한다:
///
/// ```text
///   [x_px]   [W/2]    1 [ cosθ  sinθ ] [X_map - Cx]
///   [y_px] = [H/2] + ---[ sinθ -cosθ ] [Y_map - Cy]
///                     r
/// ```
///
/// y 부호가 뒤집혀 있는 것은 화면 y 가 아래로 증가하고 지도 y 가 북쪽으로
/// 증가하기 때문이다. θ=0 이면 기존 [`ViewportTransform`] 과 같은 결과다.
///
/// ## 왜 ViewportTransform 을 안 지우고 따로 두는가
///
/// `ViewportTransform` 은 줌·패닝 상태(중심·해상도)를 들고 다니는 **가변 상태**이고
/// 이미 5곳에서 쓰인다. `MapToPixel` 은 그 상태에서 만들어 내는 **불변 변환기**라
/// 역할이 다르다. 기존 호출부를 깨지 않으려면 둘을 나눠 두는 편이 안전하다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapToPixel {
    /// 화면 중심이 나타내는 지도 좌표.
    pub center_xy: [f64; 2],
    /// 화면 1px 당 지도 단위(미터).
    pub meters_per_pixel: f64,
    /// 그리는 화면 크기(px).
    pub screen_size_px: [f64; 2],
    /// 지도 회전각(도). 0 이면 북쪽이 위.
    pub theta_deg: f64,
}

impl MapToPixel {
    /// 만든다. `meters_per_pixel` 이 0 이하이면 1 로 둔다.
    #[must_use]
    pub fn new(
        center_xy: [f64; 2],
        meters_per_pixel: f64,
        screen_size_px: [f64; 2],
        theta_deg: f64,
    ) -> Self {
        Self {
            center_xy,
            meters_per_pixel: if meters_per_pixel > 0.0 && meters_per_pixel.is_finite() {
                meters_per_pixel
            } else {
                1.0
            },
            screen_size_px,
            theta_deg: normalize_rotation_angle_deg(theta_deg),
        }
    }

    /// 기존 뷰포트에서 회전 0 인 변환기를 만든다.
    #[must_use]
    pub fn from_viewport(vp: &ViewportTransform) -> Self {
        Self::new(vp.center_xy, vp.meters_per_pixel, vp.screen_size_px, 0.0)
    }

    /// 지도 좌표 → 화면 픽셀 (문서 식 1).
    #[must_use]
    pub fn map_to_pixel(&self, w: [f64; 2]) -> [f64; 2] {
        let (s, c) = self.theta_deg.to_radians().sin_cos();
        let dx = w[0] - self.center_xy[0];
        let dy = w[1] - self.center_xy[1];
        let r = self.meters_per_pixel;
        let half_w = self.screen_size_px[0] * 0.5;
        let half_h = self.screen_size_px[1] * 0.5;
        [
            half_w + (c * dx + s * dy) / r,
            half_h + (s * dx - c * dy) / r,
        ]
    }

    /// 화면 픽셀 → 지도 좌표 (문서 식 2, 역변환).
    #[must_use]
    pub fn pixel_to_map(&self, p: [f64; 2]) -> [f64; 2] {
        let (s, c) = self.theta_deg.to_radians().sin_cos();
        let r = self.meters_per_pixel;
        let px = p[0] - self.screen_size_px[0] * 0.5;
        let py = p[1] - self.screen_size_px[1] * 0.5;
        [
            self.center_xy[0] + r * (c * px + s * py),
            self.center_xy[1] + r * (s * px - c * py),
        ]
    }
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

/// 표준 고고학 축척 프리셋 배열 (C++ `HGIS_STANDARD_SCALES` in `KaScaleCombo` / `StandardScales.cpp`)
pub const HGIS_STANDARD_SCALES: [f64; 9] = [
    100.0, 200.0, 500.0, 600.0, 1_000.0, 1_200.0, 5_000.0, 25_000.0, 50_000.0,
];
/// 최대 허용 축척 분모 (`1:1,000,000`, C++ `HGIS_MAX_ALLOWED_SCALE_DENOM`)
pub const HGIS_MAX_ALLOWED_SCALE_DENOM: f64 = 1_000_000.0;

/// 축척 문자열 파싱 오류 (C++ `ScaleParseError`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleParseError {
    InvalidScaleValue,
}

/// `"1:500"`, `"1 / 1,200"`, `"600"` 등 사용자 입력 축척 문자열을 정규화하여 분모(`500.0`, `1200.0`, `600.0`)로 파싱
/// `denom <= 0` 또는 숫자가 아닌 문자열은 `Err(ScaleParseError::InvalidScaleValue)` 반환
/// (C++ `KaScaleCombo::parseScaleString` 1:1 대응)
pub fn parse_scale_string(text: &str) -> Result<f64, ScaleParseError> {
    let cleaned: String = text
        .chars()
        .filter(|&c| !c.is_whitespace() && c != ',')
        .collect();
    if cleaned.is_empty() {
        return Err(ScaleParseError::InvalidScaleValue);
    }
    let denom_str = cleaned
        .strip_prefix("1:")
        .or_else(|| cleaned.strip_prefix("1/"))
        .unwrap_or(&cleaned);
    let val = denom_str
        .parse::<f64>()
        .map_err(|_| ScaleParseError::InvalidScaleValue)?;
    if !val.is_finite() || val <= 0.0 {
        return Err(ScaleParseError::InvalidScaleValue);
    }
    Ok(val.clamp(1.0, HGIS_MAX_ALLOWED_SCALE_DENOM))
}

/// 축척 분모를 `"1:500"` 형식으로 포맷팅 (C++ `KaScaleCombo::formatScaleDenominator`)
#[must_use]
pub fn format_scale_denominator(denominator: f64) -> String {
    let clamped = if denominator.is_finite() && denominator > 0.0 {
        denominator.clamp(1.0, HGIS_MAX_ALLOWED_SCALE_DENOM).round() as u64
    } else {
        500
    };
    format!("1:{clamped}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 문서 Phase 1 완료 기준: θ=0 이면 MapToPixel 이 기존 뷰포트와 **같은 결과**.
    #[test]
    fn map_to_pixel_matches_viewport_when_theta_is_zero() {
        let vp = ViewportTransform::new([200_000.0, 500_000.0], 0.35, [1920.0, 1080.0]);
        let m = MapToPixel::from_viewport(&vp);
        for w in [
            [200_000.0, 500_000.0],
            [200_123.456, 500_654.321],
            [199_800.0, 499_700.0],
        ] {
            let a = vp.world_to_screen(w);
            let b = m.map_to_pixel(w);
            assert!((a[0] - b[0]).abs() < 1e-9, "x 불일치 {w:?}");
            assert!((a[1] - b[1]).abs() < 1e-9, "y 불일치 {w:?}");
        }
    }

    /// 순방향→역방향 왕복이 원래 좌표로 돌아온다(θ 있는 경우 포함).
    #[test]
    fn map_to_pixel_roundtrip_survives_rotation() {
        for theta in [0.0, 15.0, 45.0, 90.0, 180.0, 315.0] {
            let m = MapToPixel::new([218_500.0, 361_400.0], 0.25, [1600.0, 900.0], theta);
            for w in [
                [218_500.0, 361_400.0],
                [218_742.0, 361_513.0],
                [218_100.0, 361_900.0],
            ] {
                let back = m.pixel_to_map(m.map_to_pixel(w));
                assert!(
                    (back[0] - w[0]).abs() < 1e-6 && (back[1] - w[1]).abs() < 1e-6,
                    "θ={theta} 에서 왕복 실패: {w:?} -> {back:?}"
                );
            }
        }
    }

    /// 화면 중심은 언제나 지도 중심을 가리킨다(회전과 무관).
    #[test]
    fn screen_center_maps_to_map_center_at_any_rotation() {
        for theta in [0.0, 30.0, 120.0, 270.0] {
            let m = MapToPixel::new([1000.0, 2000.0], 0.5, [800.0, 600.0], theta);
            let c = m.map_to_pixel([1000.0, 2000.0]);
            assert!((c[0] - 400.0).abs() < 1e-9, "θ={theta}");
            assert!((c[1] - 300.0).abs() < 1e-9, "θ={theta}");
        }
    }

    /// 회전은 **거리를 보존**한다 — 두 점 사이 화면 거리는 θ 와 무관해야 한다.
    #[test]
    fn rotation_preserves_screen_distance() {
        let a = [1000.0, 2000.0];
        let b = [1060.0, 2080.0]; // 60,80 -> 100m
        let mut dists = Vec::new();
        for theta in [0.0, 45.0, 137.0, 300.0] {
            let m = MapToPixel::new([1000.0, 2000.0], 0.5, [800.0, 600.0], theta);
            let pa = m.map_to_pixel(a);
            let pb = m.map_to_pixel(b);
            dists.push(((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2)).sqrt());
        }
        // 100m / 0.5 m/px = 200px
        for d in &dists {
            assert!((d - 200.0).abs() < 1e-6, "화면 거리 {d} 가 200px 이어야 한다");
        }
    }

    /// θ=90° 면 북쪽(+y)이 화면 **오른쪽**으로 간다.
    #[test]
    fn theta_90_puts_north_to_the_right() {
        let m = MapToPixel::new([0.0, 0.0], 1.0, [200.0, 200.0], 90.0);
        let ahead = m.map_to_pixel([0.0, 10.0]); // 북쪽 10m
        let center = m.map_to_pixel([0.0, 0.0]);
        assert!(ahead[0] > center[0], "북쪽이 오른쪽이어야 한다");
        assert!((ahead[1] - center[1]).abs() < 1e-9, "위아래로는 안 움직인다");
    }

    /// 각도는 0~360 으로 정규화된다.
    #[test]
    fn theta_is_normalized() {
        let a = MapToPixel::new([0.0, 0.0], 1.0, [100.0, 100.0], -45.0);
        let b = MapToPixel::new([0.0, 0.0], 1.0, [100.0, 100.0], 315.0);
        assert_eq!(a.theta_deg, b.theta_deg);
        assert!((a.theta_deg - 315.0).abs() < 1e-9);
    }

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
    fn test_angle_normalization_neg45_to_315_center_preservation_and_optimal_grid_interval_50m() {        assert!((normalize_rotation_angle_deg(-45.0) - 315.0).abs() < 1e-9);
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

        // TestKaScaleCombo::testScaleParsingVariousFormats
        assert!((parse_scale_string("1:500").expect("1:500") - 500.0).abs() < 1e-6);
        assert!((parse_scale_string("1 / 1,200").expect("1 / 1,200") - 1200.0).abs() < 1e-6);
        assert!((parse_scale_string("600").expect("600") - 600.0).abs() < 1e-6);
        assert_eq!(
            parse_scale_string("invalid_text"),
            Err(ScaleParseError::InvalidScaleValue)
        );
        assert_eq!(
            parse_scale_string("-500"),
            Err(ScaleParseError::InvalidScaleValue)
        );
        assert_eq!(format_scale_denominator(500.0), "1:500");
    }
}
