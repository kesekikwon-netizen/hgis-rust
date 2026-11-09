//! 적중 검사·마퀴 선택·선택 테두리 스타일.
//!
//! `canvas_painter` 에서 분리했다.

/// C++ `HGIS_MIN_FEATURE_PIXEL_SIZE` (1.5f) — 서브픽셀 컬링 최소 화면 투영 바운딩 박스 크기(px)
pub const HGIS_MIN_FEATURE_PIXEL_SIZE: f32 = 1.5;

/// C++ `HGIS_SELECTION_OUTLINE_COLOR` (`#00E5FF` Cyan Accent) 및 반투명 글로우 상수
pub const HGIS_SELECTION_OUTLINE_COLOR: [u8; 4] = [0, 229, 255, 255];

pub const HGIS_SELECTION_GLOW_COLOR: [u8; 4] = [0, 229, 255, 100];

/// C++ `HGIS_PICK_TOLERANCE_PX` (6.0f) — 2D 캔버스 클릭 히트 테스트 픽셀 반경
pub const HGIS_PICK_TOLERANCE_PX: f64 = 6.0;

/// C++ `HGIS_MARQUEE_ALPHA_VAL` (64) — 마키 드래그 선택 박스 채우기 알파값
pub const HGIS_MARQUEE_ALPHA_VAL: u8 = 64;

/// C++ `KaSpatialHitTester` 기하 차원(Dimension) 우선순위 (`Point(0) < LineString(1) < Polygon(2)`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HitGeometryDim {
    Point = 0,
    LineString = 1,
    Polygon = 2,
}

/// C++ `KaSpatialHitTester::pickFeatureAt` 입력 피처 후보
#[derive(Debug, Clone)]
pub struct PickableFeatureCandidate {
    pub fid: i64,
    pub dim: HitGeometryDim,
    pub visible: bool,
    pub locked: bool,
    pub bbox_min_xy: [f64; 2],
    pub bbox_max_xy: [f64; 2],
    pub distance_to_click_m: f64,
}

/// C++ `KaSpatialHitTester::pickFeatureAt` 1:1 대응:
/// - 비가시(`!visible`) 또는 잠금(`locked`) 레이어 피처는 피킹에서 즉시 배제
/// - 월드 검색 반경 `r_world = HGIS_PICK_TOLERANCE_PX * meters_per_pixel` 이내 후보 중
///   `Point -> LineString -> Polygon` 차원 우선순위로 최우선 피처 ID를 반환한다.
#[must_use]
pub fn pick_feature_at_with_priority(
    candidates: &[PickableFeatureCandidate],
    meters_per_pixel: f64,
) -> Option<i64> {
    let r_world = HGIS_PICK_TOLERANCE_PX * meters_per_pixel.abs().max(1e-6);
    candidates
        .iter()
        .filter(|c| c.visible && !c.locked && c.distance_to_click_m <= r_world)
        .min_by(|a, b| {
            a.dim.cmp(&b.dim).then_with(|| {
                a.distance_to_click_m
                    .partial_cmp(&b.distance_to_click_m)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
        .map(|c| c.fid)
}

/// C++ `KaSpatialHitTester::selectByMarquee` — 왼쪽->오른쪽 드래그(`start_x < end_x`)는 내부 완전 포함(Contain),
/// 오른쪽->왼쪽 드래그(`start_x >= end_x`)는 경계 교차(Crossing/Intersect) 모드로 동작한다.
#[must_use]
pub fn select_by_marquee_directional(
    candidates: &[PickableFeatureCandidate],
    drag_start_world_xy: [f64; 2],
    drag_end_world_xy: [f64; 2],
) -> Vec<i64> {
    let is_left_to_right_contain = drag_start_world_xy[0] < drag_end_world_xy[0];
    let box_min_x = drag_start_world_xy[0].min(drag_end_world_xy[0]);
    let box_max_x = drag_start_world_xy[0].max(drag_end_world_xy[0]);
    let box_min_y = drag_start_world_xy[1].min(drag_end_world_xy[1]);
    let box_max_y = drag_start_world_xy[1].max(drag_end_world_xy[1]);

    candidates
        .iter()
        .filter(|c| c.visible && !c.locked)
        .filter(|c| {
            if is_left_to_right_contain {
                c.bbox_min_xy[0] >= box_min_x
                    && c.bbox_max_xy[0] <= box_max_x
                    && c.bbox_min_xy[1] >= box_min_y
                    && c.bbox_max_xy[1] <= box_max_y
            } else {
                c.bbox_max_xy[0] >= box_min_x
                    && c.bbox_min_xy[0] <= box_max_x
                    && c.bbox_max_xy[1] >= box_min_y
                    && c.bbox_min_xy[1] <= box_max_y
            }
        })
        .map(|c| c.fid)
        .collect()
}

/// C++ `KaCanvasPainter::shouldDrawFeature` 1:1 대응:
/// 화면 투영 바운딩 박스의 가로·세로가 모두 `1.5px` 미만(`width < 1.5 && height < 1.5`)이거나
/// 폴리곤 정점 수가 3개 미만인 퇴화 기하일 때 `false`를 반환하여 드로우 콜을 생략한다.
#[must_use]
pub fn should_draw_polygon_feature(screen_pts: &[[f32; 2]]) -> bool {
    if screen_pts.len() < 3 {
        return false;
    }
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for &[x, y] in screen_pts {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    !(width < HGIS_MIN_FEATURE_PIXEL_SIZE && height < HGIS_MIN_FEATURE_PIXEL_SIZE)
}

/// C++ `KaFeatureStyleRenderer` 선택 피처 외곽선 폭(2.0배) 및 색상(`#00E5FF`) 계산
#[must_use]
pub fn compute_selection_outline_style(
    base_width_mm: f32,
    base_color: [u8; 4],
    is_selected: bool,
) -> (f32, [u8; 4]) {
    if is_selected {
        (base_width_mm * 2.0, HGIS_SELECTION_OUTLINE_COLOR)
    } else {
        (base_width_mm, base_color)
    }
}

/// C++ `HGIS_SELECTION_TOLERANCE_PX` — 선택 허용 기본 반경 (`10.0 px`)
pub const HGIS_SELECTION_TOLERANCE_PX: f64 = 10.0;

/// C++ `HGIS_HIGHLIGHT_WIDTH_PX` — 하이라이트 외곽선 두께 (`2.5 px`)
pub const HGIS_HIGHLIGHT_WIDTH_PX: f64 = 2.5;

/// C++ `HitTestResult` (`KaCanvasHitTester::performHitTest`) — 캔버스 10px 히트 테스팅 결과
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasHitTestResult {
    pub has_hit: bool,
    pub hit_fid: Option<u64>,
    pub hit_distance_pixels: f64,
}

/// C++ `KaCanvasHitTester::performHitTest` — 폴리곤 내부 포함(`0.0 px`) 및 선/점 10px 허용 반경 히트 검사 (`metersPerPixel <= 0.0` 시 `0.001 m` 안전 클램핑)
#[must_use]
pub fn perform_canvas_hit_test_10px(
    polygon_bbox_xy: Option<([f64; 2], [f64; 2], u64)>,
    line_distance_m: Option<(f64, u64)>,
    click_world_xy: [f64; 2],
    meters_per_pixel: f64,
    search_radius_px: f64,
) -> CanvasHitTestResult {
    let safe_mpp = if meters_per_pixel.is_finite() && meters_per_pixel > 0.0 {
        meters_per_pixel
    } else {
        0.001
    };
    let tol_px = if search_radius_px > 0.0 {
        search_radius_px
    } else {
        HGIS_SELECTION_TOLERANCE_PX
    };
    if let Some((min_xy, max_xy, poly_fid)) = polygon_bbox_xy {
        if click_world_xy[0] >= min_xy[0]
            && click_world_xy[0] <= max_xy[0]
            && click_world_xy[1] >= min_xy[1]
            && click_world_xy[1] <= max_xy[1]
        {
            return CanvasHitTestResult {
                has_hit: true,
                hit_fid: Some(poly_fid),
                hit_distance_pixels: 0.0,
            };
        }
    }
    if let Some((dist_m, line_fid)) = line_distance_m {
        let dist_px = dist_m / safe_mpp;
        if dist_px <= tol_px {
            return CanvasHitTestResult {
                has_hit: true,
                hit_fid: Some(line_fid),
                hit_distance_pixels: dist_px,
            };
        }
    }
    CanvasHitTestResult {
        has_hit: false,
        hit_fid: None,
        hit_distance_pixels: f64::INFINITY,
    }
}

/// C++ `KaArchaeologicalMarkerRenderer::renderNorthArrowWithDeclination` — 지도 회전각(`map_rotation_deg`), 도북 편각(`grid_convergence_deg`), 자북 편차각(`magnetic_declination_deg`)으로부터 `(도북 화살표 각도, 자북 침 각도)` 산출
#[must_use]
pub fn compute_north_arrow_declination_angles_deg(
    map_rotation_deg: f64,
    grid_convergence_deg: f64,
    magnetic_declination_deg: f64,
) -> (f64, f64) {
    let theta_grid = map_rotation_deg + grid_convergence_deg;
    let theta_mag = theta_grid + magnetic_declination_deg;
    (theta_grid, theta_mag)
}
