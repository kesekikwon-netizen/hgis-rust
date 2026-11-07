use crate::viewport::ViewportTransform;

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

#[derive(Debug, Clone, PartialEq)]
pub struct WorldPolygonItem {
    pub layer_key: String,
    pub world_pts: Vec<[f64; 2]>,
    pub fill_rgba: [u8; 4],
    pub stroke_rgba: [u8; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenPolygonPrimitive {
    pub layer_key: String,
    pub screen_pts: Vec<[f32; 2]>,
    pub fill_rgba: [u8; 4],
    pub stroke_rgba: [u8; 4],
    pub stroke_width_px: f32,
}

#[derive(Debug, Clone, Default)]
pub struct MapSceneModel {
    pub polygons: Vec<WorldPolygonItem>,
    pub buffer_500m_ring: Vec<[f64; 2]>,
    pub buffer_1000m_ring: Vec<[f64; 2]>,
    pub active_rubberband: Vec<[f64; 2]>,
    pub snap_marker_world: Option<[f64; 2]>,
    pub heritage_badges_world: Vec<(u32, [f64; 2], String)>,
}

impl MapSceneModel {
    #[must_use]
    pub fn new_demo_gyeongju_scene() -> Self {
        let cx = 219_850.0;
        let cy = 360_120.0;
        let survey_poly = vec![
            [cx - 65.0, cy - 45.0],
            [cx + 65.0, cy - 45.0],
            [cx + 65.0, cy + 45.0],
            [cx - 65.0, cy + 45.0],
        ];
        let mut polygons = vec![WorldPolygonItem {
            layer_key: "survey_area".to_owned(),
            world_pts: survey_poly,
            fill_rgba: [181, 87, 58, 52],
            stroke_rgba: [181, 87, 58, 240],
        }];
        for (idx, ox) in [-40.0, -12.0, 16.0, 44.0].into_iter().enumerate() {
            let oy = if idx % 2 == 0 { -18.0 } else { 8.0 };
            polygons.push(WorldPolygonItem {
                layer_key: "trial_trench".to_owned(),
                world_pts: vec![
                    [cx + ox - 1.0, cy + oy - 10.0],
                    [cx + ox + 1.0, cy + oy - 10.0],
                    [cx + ox + 1.0, cy + oy + 10.0],
                    [cx + ox - 1.0, cy + oy + 10.0],
                ],
                fill_rgba: [37, 99, 235, 95],
                stroke_rgba: [29, 78, 216, 255],
            });
        }

        let make_circle = |r: f64| -> Vec<[f64; 2]> {
            (0..=36)
                .map(|i| {
                    let th = (i as f64 / 36.0) * std::f64::consts::TAU;
                    [cx + r * th.cos(), cy + r * th.sin()]
                })
                .collect()
        };

        Self {
            polygons,
            buffer_500m_ring: make_circle(500.0),
            buffer_1000m_ring: make_circle(1000.0),
            active_rubberband: Vec::new(),
            snap_marker_world: Some([cx - 65.0, cy - 45.0]),
            heritage_badges_world: vec![
                (
                    1,
                    [cx + 180.0, cy + 95.0],
                    "경주 동궁과 월지 (통일신라)".to_owned(),
                ),
                (
                    2,
                    [cx - 210.0, cy - 110.0],
                    "경주 월성 (신라~통일신라)".to_owned(),
                ),
                (
                    3,
                    [cx + 340.0, cy - 160.0],
                    "경주 황룡사지 (신라)".to_owned(),
                ),
            ],
        }
    }

    #[must_use]
    pub fn project_polygons(&self, vp: &ViewportTransform) -> Vec<ScreenPolygonPrimitive> {
        let mut out = Vec::with_capacity(self.polygons.len());
        for item in &self.polygons {
            let screen_pts: Vec<[f32; 2]> = item
                .world_pts
                .iter()
                .map(|&p| {
                    let s = vp.world_to_screen(p);
                    [s[0] as f32, s[1] as f32]
                })
                .collect();
            out.push(ScreenPolygonPrimitive {
                layer_key: item.layer_key.clone(),
                screen_pts,
                fill_rgba: item.fill_rgba,
                stroke_rgba: item.stroke_rgba,
                stroke_width_px: if item.layer_key == "survey_area" {
                    2.4
                } else {
                    1.6
                },
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_gyeongju_scene_projects_survey_area_and_trenches_to_screen_space() {
        let scene = MapSceneModel::new_demo_gyeongju_scene();
        let vp = ViewportTransform::new([219_850.0, 360_120.0], 0.5, [1200.0, 800.0]);
        let prims = scene.project_polygons(&vp);
        assert!(prims.len() >= 2);
        assert!(prims.iter().any(|p| p.layer_key == "survey_area"));
        assert!(prims.iter().any(|p| p.layer_key == "trial_trench"));
    }

    #[test]
    fn test_subpixel_culling_under_1_5px_and_selection_outline_width_doubled() {
        // 1. TestKaCanvasPainter::testSubpixelCullingUnder1_5px: width = 0.8px, height = 0.7px -> false
        let tiny_poly = [[10.0_f32, 20.0], [10.8, 20.0], [10.8, 20.7], [10.0, 20.7]];
        assert!(!should_draw_polygon_feature(&tiny_poly));

        // Degenerate polygon (< 3 vertices) -> false
        assert!(!should_draw_polygon_feature(&[[10.0, 20.0], [30.0, 40.0]]));

        // Normal polygon (10px x 10px) -> true
        let normal_poly = [[10.0_f32, 20.0], [20.0, 20.0], [20.0, 30.0], [10.0, 30.0]];
        assert!(should_draw_polygon_feature(&normal_poly));

        // 2. TestKaFeatureStyleRenderer::testSelectionOutlineWidthDoubled: 0.35mm selected -> 0.70mm & #00E5FF
        let (w_sel, col_sel) = compute_selection_outline_style(0.35, [139, 69, 19, 255], true);
        assert!((w_sel - 0.70).abs() < 1e-6);
        assert_eq!(col_sel, HGIS_SELECTION_OUTLINE_COLOR);

        // 3. TestKaSpatialHitTester::testPointOverPolygonPriority & testMarqueeLeftRightMode
        let candidates = vec![
            PickableFeatureCandidate {
                fid: 200, // Polygon B
                dim: HitGeometryDim::Polygon,
                visible: true,
                locked: false,
                bbox_min_xy: [90.0, 90.0],
                bbox_max_xy: [120.0, 120.0],
                distance_to_click_m: 0.0,
            },
            PickableFeatureCandidate {
                fid: 101, // Artifact Point A at (100.0, 100.0)
                dim: HitGeometryDim::Point,
                visible: true,
                locked: false,
                bbox_min_xy: [100.0, 100.0],
                bbox_max_xy: [100.0, 100.0],
                distance_to_click_m: 0.2,
            },
        ];
        assert_eq!(pick_feature_at_with_priority(&candidates, 1.0), Some(101));

        // Marquee [80..110, 80..110] only partially overlaps Polygon B [90..120, 90..120]
        let only_poly = &candidates[0..1];
        let contain_res = select_by_marquee_directional(only_poly, [80.0, 80.0], [110.0, 110.0]);
        assert!(contain_res.is_empty());
        let crossing_res = select_by_marquee_directional(only_poly, [110.0, 110.0], [80.0, 80.0]);
        assert_eq!(crossing_res, vec![200]);
    }
}
