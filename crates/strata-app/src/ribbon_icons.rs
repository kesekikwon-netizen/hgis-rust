use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// Strata 브랜드 로고 (테라코타 둥근 사각형 + 고고학 꽃삽 + 등고선 파동)
pub fn paint_strata_logo(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 7.0, Color32::from_rgb(189, 98, 66));
    let c = rect.center();
    let r_max = rect.width() * 0.38;
    // 등고선 동심원 호
    for i in 1..=3 {
        let r = r_max * (i as f32 / 3.0);
        painter.circle_stroke(
            Pos2::new(c.x, c.y + 3.0),
            r,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 240, 230, 150)),
        );
    }
    // 꽃삽(Trowel) 마름모 날 + 손잡이
    let blade = vec![
        Pos2::new(c.x, c.y - 2.0),
        Pos2::new(c.x + 5.5, c.y + 3.5),
        Pos2::new(c.x, c.y + 9.5),
        Pos2::new(c.x - 5.5, c.y + 3.5),
    ];
    painter.add(egui::Shape::convex_polygon(
        blade,
        Color32::from_rgb(245, 243, 238),
        Stroke::new(1.2, Color32::from_rgb(60, 45, 38)),
    ));
    painter.line_segment(
        [Pos2::new(c.x, c.y - 2.0), Pos2::new(c.x, c.y - 9.5)],
        Stroke::new(2.4, Color32::from_rgb(60, 45, 38)),
    );
}

/// C++ `KaIcons.cpp` 23개 리본 벡터 라인 아이콘 1:1 드로잉
pub fn paint_ribbon_icon(painter: &egui::Painter, rect: Rect, label: &str, stroke_col: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.6, stroke_col);
    let s_thin = Stroke::new(1.2, stroke_col);
    match label {
        "새 조사" => {
            let r = Rect::from_center_size(c, Vec2::new(14.0, 16.0));
            painter.rect_stroke(r, 2.0, s, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x + 4.0, c.y)], s);
            painter.line_segment([Pos2::new(c.x, c.y - 4.0), Pos2::new(c.x, c.y + 4.0)], s);
        }
        "열기" => {
            let r = Rect::from_center_size(Pos2::new(c.x, c.y + 1.5), Vec2::new(16.0, 12.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            painter.line_segment(
                [Pos2::new(r.left() + 1.0, r.top() - 2.5), Pos2::new(r.left() + 7.0, r.top() - 2.5)],
                s,
            );
        }
        "저장" | "다른 이름" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 15.0));
            painter.rect_stroke(r, 2.0, s, egui::StrokeKind::Middle);
            let top_box = Rect::from_min_max(Pos2::new(c.x - 4.5, r.top()), Pos2::new(c.x + 3.5, c.y - 1.5));
            painter.rect_stroke(top_box, 1.0, s_thin, egui::StrokeKind::Middle);
            if label == "다른 이름" {
                painter.line_segment(
                    [Pos2::new(c.x + 2.0, c.y + 5.0), Pos2::new(c.x + 7.5, c.y - 0.5)],
                    s,
                );
            }
        }
        "선택" => {
            let pts = vec![
                Pos2::new(c.x - 5.0, c.y - 7.0),
                Pos2::new(c.x + 6.5, c.y - 1.0),
                Pos2::new(c.x + 1.0, c.y + 1.5),
                Pos2::new(c.x - 1.5, c.y + 7.0),
            ];
            painter.add(egui::Shape::closed_line(pts, s));
        }
        "측거" => {
            let r = Rect::from_center_size(c, Vec2::new(17.0, 7.0));
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
            for dx in [-5.0, -1.5, 2.0, 5.5] {
                painter.line_segment(
                    [Pos2::new(c.x + dx, r.top()), Pos2::new(c.x + dx, r.top() + 3.2)],
                    s_thin,
                );
            }
        }
        "그리기" => {
            let pts = [
                Pos2::new(c.x - 5.5, c.y - 4.5),
                Pos2::new(c.x + 5.0, c.y - 6.0),
                Pos2::new(c.x + 6.0, c.y + 4.5),
                Pos2::new(c.x - 4.0, c.y + 6.0),
            ];
            painter.add(egui::Shape::closed_line(pts.to_vec(), s_thin));
            for p in pts {
                painter.circle_filled(p, 2.2, stroke_col);
            }
        }
        "시굴격자" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 15.0));
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
            for d in [-2.5, 2.5] {
                painter.line_segment([Pos2::new(c.x + d, r.top()), Pos2::new(c.x + d, r.bottom())], s_thin);
                painter.line_segment([Pos2::new(r.left(), c.y + d), Pos2::new(r.right(), c.y + d)], s_thin);
            }
        }
        "버퍼" => {
            painter.circle_stroke(c, 7.5, s);
            painter.circle_filled(c, 2.0, stroke_col);
        }
        "지적" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 15.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x - 1.0, r.top()), Pos2::new(c.x - 1.0, r.bottom())], s_thin);
            painter.line_segment([Pos2::new(r.left(), c.y + 1.0), Pos2::new(c.x - 1.0, c.y + 1.0)], s_thin);
        }
        "수치지형" => {
            painter.line_segment([Pos2::new(c.x, c.y - 7.0), Pos2::new(c.x, c.y + 3.0)], s);
            painter.line_segment([Pos2::new(c.x - 4.0, c.y - 1.0), Pos2::new(c.x, c.y + 3.0)], s);
            painter.line_segment([Pos2::new(c.x + 4.0, c.y - 1.0), Pos2::new(c.x, c.y + 3.0)], s);
            painter.line_segment([Pos2::new(c.x - 7.0, c.y + 6.5), Pos2::new(c.x + 7.0, c.y + 6.5)], s);
        }
        "유산" => {
            let roof = vec![
                Pos2::new(c.x - 7.5, c.y - 0.5),
                Pos2::new(c.x, c.y - 7.5),
                Pos2::new(c.x + 7.5, c.y - 0.5),
            ];
            painter.add(egui::Shape::line(roof, s));
            let body = Rect::from_min_max(Pos2::new(c.x - 5.5, c.y - 0.5), Pos2::new(c.x + 5.5, c.y + 7.0));
            painter.rect_stroke(body, 1.0, s, egui::StrokeKind::Middle);
        }
        "지형" | "DEM" => {
            let pts = vec![
                Pos2::new(c.x - 8.0, c.y + 6.0),
                Pos2::new(c.x - 2.5, c.y - 5.5),
                Pos2::new(c.x + 1.5, c.y + 1.5),
                Pos2::new(c.x + 5.0, c.y - 3.5),
                Pos2::new(c.x + 8.5, c.y + 6.0),
            ];
            painter.add(egui::Shape::closed_line(pts, s));
        }
        "등고선" => {
            painter.circle_stroke(c, 7.5, s);
            painter.circle_stroke(c, 4.2, s_thin);
            painter.circle_filled(c, 1.5, stroke_col);
        }
        "토양" => {
            for dy in [-3.5, 0.5, 4.5] {
                let pts = vec![
                    Pos2::new(c.x, c.y + dy - 3.5),
                    Pos2::new(c.x + 7.5, c.y + dy),
                    Pos2::new(c.x, c.y + dy + 3.5),
                    Pos2::new(c.x - 7.5, c.y + dy),
                ];
                painter.add(egui::Shape::closed_line(pts, s_thin));
            }
        }
        "고지형" => {
            painter.circle_stroke(c, 7.2, s);
            painter.line_segment([c, Pos2::new(c.x, c.y - 4.2)], s);
            painter.line_segment([c, Pos2::new(c.x + 3.5, c.y + 1.5)], s);
        }
        "지질" => {
            for (dy, w) in [(-4.0, 5.5), (0.5, 8.0), (5.0, 6.5)] {
                painter.line_segment([Pos2::new(c.x - w, c.y + dy), Pos2::new(c.x + w, c.y + dy)], s);
            }
            painter.line_segment([Pos2::new(c.x, c.y - 6.5), Pos2::new(c.x, c.y + 1.0)], s);
        }
        "수계" => {
            for dy in [-4.5, 0.0, 4.5] {
                let pts = vec![
                    Pos2::new(c.x - 7.0, c.y + dy),
                    Pos2::new(c.x - 2.5, c.y + dy - 2.0),
                    Pos2::new(c.x + 2.5, c.y + dy + 2.0),
                    Pos2::new(c.x + 7.0, c.y + dy),
                ];
                painter.add(egui::Shape::line(pts, s));
            }
        }
        "옛 지도" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 13.0));
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Middle);
            painter.line_segment([Pos2::new(c.x - 2.5, r.top()), Pos2::new(c.x - 2.5, r.bottom())], s_thin);
            painter.line_segment([Pos2::new(c.x + 2.5, r.top()), Pos2::new(c.x + 2.5, r.bottom())], s_thin);
        }
        "정합" => {
            painter.circle_stroke(c, 5.5, s);
            painter.line_segment([Pos2::new(c.x - 8.5, c.y), Pos2::new(c.x + 8.5, c.y)], s_thin);
            painter.line_segment([Pos2::new(c.x, c.y - 8.5), Pos2::new(c.x, c.y + 8.5)], s_thin);
        }
        "도면" => {
            let r = Rect::from_center_size(c, Vec2::new(13.0, 16.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            for dy in [-3.0, 0.5, 4.0] {
                painter.line_segment([Pos2::new(c.x - 3.5, c.y + dy), Pos2::new(c.x + 3.5, c.y + dy)], s_thin);
            }
        }
        "인쇄" => {
            let body = Rect::from_center_size(Pos2::new(c.x, c.y + 1.0), Vec2::new(16.0, 9.0));
            painter.rect_stroke(body, 1.5, s, egui::StrokeKind::Middle);
            let top = Rect::from_min_max(Pos2::new(c.x - 4.5, c.y - 6.5), Pos2::new(c.x + 4.5, c.y - 3.5));
            painter.rect_stroke(top, 0.5, s_thin, egui::StrokeKind::Middle);
        }
        "GeoTIFF" => {
            let r = Rect::from_center_size(c, Vec2::new(15.0, 13.0));
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Middle);
            painter.circle_filled(Pos2::new(c.x - 3.5, c.y - 2.5), 1.6, stroke_col);
            painter.line_segment([Pos2::new(r.left() + 2.0, r.bottom() - 2.0), Pos2::new(c.x + 1.5, c.y)], s_thin);
            painter.line_segment([Pos2::new(c.x + 1.5, c.y), Pos2::new(r.right() - 2.0, r.bottom() - 2.0)], s_thin);
        }
        "검수·제출" => {
            let pts = vec![
                Pos2::new(c.x, c.y - 7.5),
                Pos2::new(c.x + 7.0, c.y - 3.5),
                Pos2::new(c.x + 7.0, c.y + 4.0),
                Pos2::new(c.x, c.y + 7.5),
                Pos2::new(c.x - 7.0, c.y + 4.0),
                Pos2::new(c.x - 7.0, c.y - 3.5),
            ];
            painter.add(egui::Shape::closed_line(pts, s));
            painter.line_segment([Pos2::new(c.x - 7.0, c.y - 3.5), c], s_thin);
            painter.line_segment([Pos2::new(c.x + 7.0, c.y - 3.5), c], s_thin);
            painter.line_segment([c, Pos2::new(c.x, c.y + 7.5)], s_thin);
        }
        _ => {
            for dx in [-4.5, 0.0, 4.5] {
                painter.circle_filled(Pos2::new(c.x + dx, c.y), 1.6, stroke_col);
            }
        }
    }
}
