//! 키보드 이동 상수·사각형 생성·중심 이동.
//!


/// 시굴 트렌치 기본 단폭 (2.0 m, C++ `HGIS_TRENCH_DEFAULT_WIDTH_M`)
pub const HGIS_TRENCH_DEFAULT_WIDTH_M: f64 = 2.0;
/// 시굴 트렌치 기본 장폭 (20.0 m, C++ `HGIS_TRENCH_DEFAULT_LENGTH_M`)
pub const HGIS_TRENCH_DEFAULT_LENGTH_M: f64 = 20.0;
/// 화살표 키 기본 1회 이동 보폭 (0.1 m / 10 cm, C++ `HGIS_TRENCH_KEY_STEP_BASE_M`)
pub const HGIS_TRENCH_KEY_STEP_BASE_M: f64 = 0.1;
/// Shift + 화살표 키 대형 이동 보폭 (1.0 m, C++ `HGIS_TRENCH_KEY_STEP_LARGE_M`)
pub const HGIS_TRENCH_KEY_STEP_LARGE_M: f64 = 1.0;
/// 키 연속 입력 디바운스 간격 (30 ms, C++ `HGIS_KEY_DEBOUNCE_INTERVAL_MS`)
pub const HGIS_KEY_DEBOUNCE_INTERVAL_MS: u64 = 30;

/// 화살표 키 방향 열거형 (C++ `KaTrenchMoveToolKeys`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrenchArrowKey {
    Left,
    Right,
    Up,
    Down,
}

/// 중심점(`center`), 단폭(`width_m`, 최소 1.0m 클램핑), 장폭(`length_m`, 최소 5.0m 클램핑), 회전각(`rotation_deg`)으로부터
/// 2D 아핀 회전된 트렌치 4정점 닫힌 링, 바운딩 박스(`[min_x, min_y, max_x, max_y]`), 면적(㎡) 산출
/// (C++ `KaTrenchPlaceTool::canvasReleaseEvent` 1:1 대응)
#[must_use]
pub fn create_trench_rectangle_polygon(
    center: [f64; 2],
    width_m: f64,
    length_m: f64,
    rotation_deg: f64,
) -> (Vec<[f64; 2]>, [f64; 4], f64) {
    let eff_w = if width_m.is_finite() {
        width_m.max(1.0)
    } else {
        HGIS_TRENCH_DEFAULT_WIDTH_M
    };
    let eff_l = if length_m.is_finite() {
        length_m.max(5.0)
    } else {
        HGIS_TRENCH_DEFAULT_LENGTH_M
    };
    let rad = if rotation_deg.is_finite() {
        rotation_deg.to_radians()
    } else {
        0.0
    };
    let (cos_t, sin_t) = (rad.cos(), rad.sin());
    let half_l = eff_l * 0.5;
    let half_w = eff_w * 0.5;
    let local_corners = [
        [-half_l, -half_w],
        [half_l, -half_w],
        [half_l, half_w],
        [-half_l, half_w],
    ];
    let mut ring = Vec::with_capacity(5);
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for [dx, dy] in local_corners {
        let wx = center[0] + dx * cos_t - dy * sin_t;
        let wy = center[1] + dx * sin_t + dy * cos_t;
        min_x = min_x.min(wx);
        min_y = min_y.min(wy);
        max_x = max_x.max(wx);
        max_y = max_y.max(wy);
        ring.push([wx, wy]);
    }
    if let Some(&first) = ring.first() {
        ring.push(first);
    }
    (ring, [min_x, min_y, max_x, max_y], eff_w * eff_l)
}

/// 화살표 키 및 Shift 수식키에 따른 트렌치 중심 좌표 평행이동 및 비편집 모드 가드
/// (C++ `KaTrenchMoveToolKeys::translateSelectedTrenches` 1:1 대응)
pub fn translate_trench_center_with_key(
    center: [f64; 2],
    key: TrenchArrowKey,
    shift_pressed: bool,
    layer_is_editable: bool,
) -> Result<[f64; 2], &'static str> {
    if !layer_is_editable {
        return Err(
            "트렌치 레이어가 편집 모드가 아닙니다. 편집 시작(연필 아이콘)을 먼저 눌러주세요.",
        );
    }
    let step = if shift_pressed {
        HGIS_TRENCH_KEY_STEP_LARGE_M
    } else {
        HGIS_TRENCH_KEY_STEP_BASE_M
    };
    let (dx, dy) = match key {
        TrenchArrowKey::Left => (-step, 0.0),
        TrenchArrowKey::Right => (step, 0.0),
        TrenchArrowKey::Up => (0.0, step),
        TrenchArrowKey::Down => (0.0, -step),
    };
    Ok([center[0] + dx, center[1] + dy])
}
