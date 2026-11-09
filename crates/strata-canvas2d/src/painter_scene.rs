//! 지도 장면 모델(`MapSceneModel`)의 시험용 데모와 폴리곤 투영.
//!
//! `canvas_painter` 에서 분리했다.

use crate::viewport::ViewportTransform;

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
