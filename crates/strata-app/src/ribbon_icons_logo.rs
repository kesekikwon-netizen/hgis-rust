//! Strata 브랜드 로고 그리기.
//!
//! `ribbon_icons.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 함수는 그대로다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

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
