//! 레이어 엔진이 낸 도형을 egui 로 칠한다.
//!
//! 엔진(`strata-canvas2d::layer_engine`)은 egui 를 모른다. 이 파일이 그 경계다:
//! `DrawShape` → `egui::Painter` 호출. 그래서 엔진은 화면 없이도 시험되고,
//! 조판·PDF 도 같은 도형을 재사용할 수 있다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use strata_canvas2d::layer_engine::{DrawShape, LayerDraw, ScreenTransform};

/// 레이어별 도형을 칠한다. 칠한 도형 수를 돌려준다.
pub fn paint_layer_draws(painter: &egui::Painter, draws: &[LayerDraw]) -> usize {
    let mut n = 0;
    for d in draws {
        for s in &d.shapes {
            paint_shape(painter, s);
            n += 1;
        }
    }
    n
}

fn paint_shape(painter: &egui::Painter, s: &DrawShape) {
    match s {
        DrawShape::Polygon {
            pts,
            fill,
            stroke,
            stroke_width_px,
        } => {
            let sp: Vec<Pos2> = pts.iter().map(|p| Pos2::new(p[0], p[1])).collect();
            if sp.len() >= 3 {
                if fill[3] > 0 {
                    // egui 의 `convex_polygon` 은 **볼록 전용**이다(삼각팬으로 채운다).
                    // 오목 도형(L자 시굴 트렌치·클릭으로 그린 조사구역·필지)은 파인 곳이
                    // 메워진다. 검증 경로(rasterize)는 스캔라인이라 정확하므로
                    // 그냥 두면 "시험은 통과하는데 화면은 틀린" 상태가 된다.
                    //
                    // 그래서 오목이면 삼각분할해서 `Shape::Mesh` 로 칠한다.
                    // 이미 워크스페이스에 있는 `geo` 의 earcut 을 쓴다(직접 구현 안 함).
                    let ring_f64: Vec<[f64; 2]> = pts
                        .iter()
                        .map(|p| [f64::from(p[0]), f64::from(p[1])])
                        .collect();
                    if strata_canvas2d::layer_engine::is_concave(&ring_f64) {
                        let tris = strata_canvas2d::layer_engine::triangulate_polygon(
                            &ring_f64,
                            &[],
                            |p| [p[0] as f32, p[1] as f32],
                        );
                        if tris.is_empty() {
                            // 삼각분할이 실패하면 예전처럼 볼록으로 시도한다(빈 화면보다 낫다).
                            painter.add(egui::Shape::convex_polygon(
                                sp.clone(),
                                to_color(*fill),
                                Stroke::NONE,
                            ));
                        } else {
                            painter.add(triangle_mesh(&tris, to_color(*fill)));
                        }
                    } else {
                        painter.add(egui::Shape::convex_polygon(
                            sp.clone(),
                            to_color(*fill),
                            Stroke::NONE,
                        ));
                    }
                }
                if *stroke_width_px > 0.0 && stroke[3] > 0 {
                    painter.add(egui::Shape::closed_line(
                        sp,
                        Stroke::new(*stroke_width_px, to_color(*stroke)),
                    ));
                }
            }
        }
        DrawShape::Line {
            pts,
            color,
            width_px,
        } => {
            if pts.len() >= 2 {
                let line: Vec<Pos2> = pts.iter().map(|p| Pos2::new(p[0], p[1])).collect();
                painter.add(egui::Shape::line(
                    line,
                    Stroke::new(*width_px, to_color(*color)),
                ));
            }
        }
        DrawShape::Point {
            at,
            radius_px,
            fill,
            stroke,
        } => {
            let c = Pos2::new(at[0], at[1]);
            painter.circle_filled(c, *radius_px, to_color(*fill));
            if stroke[3] > 0 {
                painter.circle_stroke(
                    c,
                    *radius_px,
                    Stroke::new(1.0, to_color(*stroke)),
                );
            }
        }
        DrawShape::Label {
            at,
            text,
            size_px,
            color,
        } => {
            painter.text(
                Pos2::new(at[0], at[1]),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(*size_px),
                to_color(*color),
            );
        }
    }
}

/// RGBA → egui 색.
fn to_color(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// 삼각분할 결과를 egui 메시로 묶는다.
///
/// egui `Shape::Mesh` 는 임의 삼각형을 받으므로 오목 다각형도 정확히 칠해진다.
/// `Shape::convex_polygon` 이 쓰는 삼각팬 방식과 달리 파인 곳이 메워지지 않는다.
fn triangle_mesh(tris: &[strata_canvas2d::layer_engine::ScreenTriangle], color: Color32) -> egui::Shape {
    let mut mesh = egui::Mesh::default();
    for t in tris {
        let base = mesh.vertices.len() as u32;
        for p in [t.a, t.b, t.c] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: Pos2::new(p[0], p[1]),
                uv: egui::epaint::WHITE_UV,
                color,
            });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    egui::Shape::mesh(mesh)
}

/// 화면 사각형과 뷰포트로 변환기를 만든다.
///
/// 지도 뷰포트(`ViewportTransform`)는 중심·축척을 들고 있고, 엔진은 좌상단
/// 원점을 원한다. 여기서 변환한다.
#[must_use]
pub fn screen_transform_for(
    vp: &strata_canvas2d::viewport::ViewportTransform,
    rect: Rect,
) -> ScreenTransform {
    let cx = f64::from(rect.center().x - rect.left());
    let cy = f64::from(rect.center().y - rect.top());
    ScreenTransform::new(
        [
            vp.center_xy[0] - cx * vp.meters_per_pixel,
            vp.center_xy[1] + cy * vp.meters_per_pixel,
        ],
        vp.meters_per_pixel,
        [rect.left(), rect.top()],
    )
}
