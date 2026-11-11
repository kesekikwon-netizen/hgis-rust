//! 참조 레이어(받은 SHP)를 지도에 그린다.
//!
//! C++ 은 QGIS 가 올린 벡터 레이어를 단색 기호로 그리고, 이름 속성이 있으면 글자를 붙인다.
//! 여기서는 가는 회색 선(면은 외곽선만), 점은 작은 원으로 그리고, 가까이 보면
//! (1 픽셀이 1 m 이하) 이름을 흰 테두리 글자로 붙인다. 글자끼리 겹치면 건너뛴다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::real_survey::ring_centroid;

use super::gui_tab_map_geom::w2s;

/// 화면 1 mm 의 픽셀 수(96 DPI).
const PX_PER_MM: f32 = 96.0 / 25.4;

impl StrataAppState {
    pub(crate) fn paint_reference_layers(&self, p: &egui::Painter, rect: Rect) {
        let vp = &self.home_map.viewport;
        let rs = &self.home_map.real_survey;
        let font = FontId::proportional(8.0 * 96.0 / 72.0);
        let mut placed: Vec<Rect> = Vec::new();
        let seen = rect.expand(60.0);
        // 화면에 보이는 지도 범위(여유 포함). 범위 밖 도형은 좌표를 바꾸기 전에 거른다.
        let a = vp.screen_to_world([-60.0, -60.0]);
        let b = vp.screen_to_world([f64::from(rect.width()) + 60.0, f64::from(rect.height()) + 60.0]);
        let view_box = [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])];
        for layer in &self.home_map.reference_layers {
            if !rs.is_layer_visible(&layer.name) {
                continue;
            }
            // 레이어마다 제 선 색·굵기와 이름을 보이는 축척을 쓴다.
            let [r, g, b] = layer.style.line_rgb;
            let stroke = Stroke::new((layer.style.line_mm * PX_PER_MM).max(1.0), Color32::from_rgb(r, g, b));
            let show_labels = vp.meters_per_pixel <= layer.style.label_max_mpp;
            for shape in &layer.shapes {
                if !crate::reference_layers::boxes_overlap(shape.bbox, view_box) {
                    continue;
                }
                // 색이 있는 면(지질도 등)은 먼저 칠한다. `parts` 는 바깥 고리 + 구멍이다.
                if let Some([fr, fg, fb, fa]) = shape.fill {
                    super::gui_tab_map_domain::fill_polygon(
                        p,
                        rect,
                        vp,
                        &shape.parts,
                        Color32::from_rgba_unmultiplied(fr, fg, fb, fa),
                    );
                }
                let mut any_inside = false;
                for part in &shape.parts {
                    let pts: Vec<Pos2> = part.iter().map(|&q| w2s(rect, vp, q)).collect();
                    if !pts.iter().any(|q| seen.contains(*q)) {
                        continue;
                    }
                    any_inside = true;
                    if layer.shape_type == 1 || pts.len() == 1 {
                        p.circle(pts[0], 3.0, Color32::from_rgb(120, 116, 106), stroke);
                    } else {
                        p.add(egui::Shape::line(pts, stroke));
                    }
                }
                if !show_labels || !any_inside || shape.label.is_empty() {
                    continue;
                }
                let Some(first) = shape.parts.first() else {
                    continue;
                };
                let at = w2s(rect, vp, ring_centroid(first));
                if !rect.contains(at) {
                    continue;
                }
                let ink = Color32::from_rgb(31, 35, 40);
                let galley = p.layout_no_wrap(shape.label.clone(), font.clone(), ink);
                let top_left = Pos2::new(at.x - galley.size().x * 0.5, at.y - galley.size().y * 0.5);
                let bounds = Rect::from_min_size(top_left, galley.size()).expand(2.0);
                if placed.iter().any(|r| r.intersects(bounds)) {
                    continue;
                }
                placed.push(bounds);
                let halo = p.layout_no_wrap(shape.label.clone(), font.clone(), Color32::WHITE);
                for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
                    p.galley(top_left + Vec2::new(dx, dy), halo.clone(), Color32::WHITE);
                }
                p.galley(top_left, galley, ink);
            }
        }
    }
}
