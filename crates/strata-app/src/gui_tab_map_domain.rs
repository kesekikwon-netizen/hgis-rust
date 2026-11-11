//! 조사 문서(`SurveyDoc`)의 유구 면·유구 선·단면선·기준점·유물을 지도에 그린다.
//!
//! C++ 은 `QgsVectorLayer` + `LayerStyleDefaults::forKey` 의 기호로 QGIS 가 그린다.
//! 색·굵기·점 크기는 `LayerStyleDefaults.cpp:42-76` 의 값을 그대로 옮겼다(단위 mm).
//! 조사구역·시굴격자는 아직 `gui_tab_map_heritage` 가 `real_survey` 에서 그린다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use geo::TriangulateEarcut;

use strata_canvas2d::viewport::ViewportTransform;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;

use super::gui_tab_map_geom::w2s;

/// 화면 1 mm 에 해당하는 논리 픽셀 수(96 DPI 기준). QGIS 의 mm 단위 기호 크기를 옮길 때 쓴다.
const PX_PER_MM: f32 = 96.0 / 25.4;

/// `LayerStyleDefaults::DomainStyle` 의 Rust 판.
struct DomainStyle {
    fill: Color32,
    stroke: Color32,
    stroke_mm: f32,
    /// 선 밑에 까는 흰 테두리(단면선) 굵기. 없으면 0.
    casing_mm: f32,
    marker_mm: f32,
}

/// C++ `LayerStyleDefaults::forKey` (`LayerStyleDefaults.cpp:42-76`).
fn style_for(key: DomainLayerKey) -> DomainStyle {
    let rgba = Color32::from_rgba_unmultiplied;
    match key {
        DomainLayerKey::FeaturePoly => DomainStyle {
            fill: rgba(22, 163, 74, 90),
            stroke: rgba(17, 94, 44, 255),
            stroke_mm: 1.8,
            casing_mm: 0.0,
            marker_mm: 0.0,
        },
        DomainLayerKey::FeatureLine => DomainStyle {
            fill: Color32::TRANSPARENT,
            stroke: rgba(202, 138, 4, 255),
            stroke_mm: 1.8,
            casing_mm: 0.0,
            marker_mm: 0.0,
        },
        DomainLayerKey::SectionLine => DomainStyle {
            fill: Color32::TRANSPARENT,
            stroke: rgba(190, 24, 93, 255),
            stroke_mm: 1.8,
            casing_mm: 3.0,
            marker_mm: 0.0,
        },
        DomainLayerKey::ControlPoints => DomainStyle {
            fill: rgba(234, 179, 8, 255),
            stroke: rgba(161, 98, 7, 255),
            stroke_mm: 0.6,
            casing_mm: 0.0,
            marker_mm: 4.0,
        },
        DomainLayerKey::ArtifactPoint => DomainStyle {
            fill: rgba(185, 28, 28, 255),
            stroke: rgba(127, 29, 29, 255),
            stroke_mm: 0.6,
            casing_mm: 0.0,
            marker_mm: 3.6,
        },
        DomainLayerKey::SurveyArea | DomainLayerKey::TrialTrench => DomainStyle {
            fill: rgba(37, 99, 235, 90),
            stroke: rgba(37, 99, 235, 255),
            stroke_mm: 1.0,
            casing_mm: 0.0,
            marker_mm: 0.0,
        },
    }
}

/// 다각형(구멍·오목한 모양 포함)을 삼각형으로 나눠 칠한다.
/// `Shape::convex_polygon` 은 볼록한 모양만 맞게 칠하므로 쓰지 않는다.
pub(super) fn fill_polygon(p: &egui::Painter, rect: Rect, vp: &ViewportTransform, rings: &[Vec<[f64; 2]>], color: Color32) {
    let Some(outer) = rings.first() else {
        return;
    };
    if outer.len() < 4 || color == Color32::TRANSPARENT {
        return;
    }
    let to_line = |r: &Vec<[f64; 2]>| geo::LineString::from(r.iter().map(|q| (q[0], q[1])).collect::<Vec<_>>());
    let polygon = geo::Polygon::new(to_line(outer), rings[1..].iter().map(to_line).collect());
    let mut mesh = egui::Mesh::default();
    for tri in polygon.earcut_triangles() {
        let base = mesh.vertices.len() as u32;
        for c in [tri.v1(), tri.v2(), tri.v3()] {
            mesh.colored_vertex(w2s(rect, vp, [c.x, c.y]), color);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    p.add(egui::Shape::mesh(mesh));
}

fn screen_points(rect: Rect, vp: &ViewportTransform, pts: &[[f64; 2]]) -> Vec<Pos2> {
    pts.iter().map(|&q| w2s(rect, vp, q)).collect()
}

impl StrataAppState {
    /// 문서의 다섯 레이어를 C++ 범례 순서(아래→위: 면 → 선 → 단면선 → 점)로 그린다.
    pub(crate) fn paint_survey_doc_layers(&self, p: &egui::Painter, rect: Rect) {
        let vp = &self.home_map.viewport;
        let rs = &self.home_map.real_survey;
        for key in [
            DomainLayerKey::FeaturePoly,
            DomainLayerKey::FeatureLine,
            DomainLayerKey::SectionLine,
            DomainLayerKey::ArtifactPoint,
            DomainLayerKey::ControlPoints,
        ] {
            if !rs.is_layer_visible(key.name_ko()) {
                continue;
            }
            let st = style_for(key);
            let stroke = Stroke::new(st.stroke_mm * PX_PER_MM, st.stroke);
            for feature in &self.survey_doc.layer(key).features {
                match feature.geom.as_ref() {
                    Some(WkbGeometry::Polygon(rings)) => {
                        fill_polygon(p, rect, vp, rings, st.fill);
                        for ring in rings {
                            p.add(egui::Shape::line(screen_points(rect, vp, ring), stroke));
                        }
                    }
                    Some(WkbGeometry::LineString(pts)) => {
                        let line = screen_points(rect, vp, pts);
                        if st.casing_mm > 0.0 {
                            p.add(egui::Shape::line(
                                line.clone(),
                                Stroke::new(st.casing_mm * PX_PER_MM, Color32::WHITE),
                            ));
                        }
                        p.add(egui::Shape::line(line, stroke));
                    }
                    Some(WkbGeometry::Point(pt)) => {
                        let c = w2s(rect, vp, *pt);
                        let radius = st.marker_mm * PX_PER_MM * 0.5;
                        p.circle(c, radius, st.fill, stroke);
                    }
                    None => {}
                }
            }
        }
        self.paint_doc_selection(p, rect);
    }

    /// 고른 도형을 노란 선으로 한 번 더 그린다(QGIS 기본 선택 색).
    fn paint_doc_selection(&self, p: &egui::Painter, rect: Rect) {
        let Some((key, idx)) = self.selected_doc else {
            return;
        };
        let Some(geom) = self.survey_doc.layer(key).features.get(idx).and_then(|f| f.geom.as_ref()) else {
            return;
        };
        let vp = &self.home_map.viewport;
        let mark = Stroke::new(2.0, Color32::from_rgb(255, 255, 0));
        match geom {
            WkbGeometry::Polygon(rings) => {
                for ring in rings {
                    p.add(egui::Shape::line(screen_points(rect, vp, ring), mark));
                }
            }
            WkbGeometry::LineString(pts) => {
                p.add(egui::Shape::line(screen_points(rect, vp, pts), mark));
            }
            WkbGeometry::Point(pt) => {
                let r = style_for(key).marker_mm * PX_PER_MM * 0.5 + 3.0;
                p.circle_stroke(w2s(rect, vp, *pt), r, mark);
            }
        }
    }
}
