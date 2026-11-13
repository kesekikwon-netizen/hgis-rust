//! 「도형 속성」 표고 구역의 단면 그림 — C++ `KaElevationFigure::paintEvent`(KaElevationFigure.cpp:50-91), 244×76.
//! 점선 상면·바닥, 땅과 판 구덩이, 깊이 괄호, 「상면 EL. …」·「바닥 EL. …」·깊이 글.

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Sense, Shape, Stroke, Vec2};

use crate::feature_record::{elevation_accessible_name, elevation_depth, elevation_level, metres};

pub(crate) const FIGURE_SIZE: Vec2 = Vec2::new(244.0, 76.0);
/// kGuide #B0AEA5.
const GUIDE: Color32 = Color32::from_rgb(0xB0, 0xAE, 0xA5);
const INK: Color32 = Color32::from_rgb(61, 61, 58);
const MUTED: Color32 = Color32::from_rgb(105, 103, 96);

fn dashed(painter: &egui::Painter, a: Pos2, b: Pos2) {
    painter.extend(Shape::dashed_line(&[a, b], Stroke::new(1.0, GUIDE), 3.0, 3.0));
}

/// 그 자리에 그림을 그리고 읽기 글(accessibleName)을 붙인다.
pub(crate) fn elevation_figure(ui: &mut egui::Ui, top: &str, bottom: &str, depth: &str) {
    let (rect, response) = ui.allocate_exact_size(FIGURE_SIZE, Sense::hover());
    let name = elevation_accessible_name(top, bottom, depth);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, name.clone()));
    let p = ui.painter_at(rect);
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y);
    dashed(&p, at(8.0, 18.5), at(196.0, 18.5)); // 상면
    dashed(&p, at(76.0, 58.5), at(196.0, 58.5)); // 바닥
    let ground = [at(8.0, 18.5), at(62.0, 18.5), at(76.0, 58.5), at(150.0, 58.5), at(164.0, 18.5), at(196.0, 18.5)];
    p.add(Shape::line(ground.to_vec(), Stroke::new(1.25, INK)));
    let bracket = Stroke::new(1.0, INK);
    p.line_segment([at(188.5, 18.5), at(188.5, 58.5)], bracket);
    p.line_segment([at(185.0, 18.5), at(192.0, 18.5)], bracket);
    p.line_segment([at(185.0, 58.5), at(192.0, 58.5)], bracket);
    let words = FontId::new(10.0, FontFamily::Proportional);
    let mono = FontId::new(10.0, FontFamily::Monospace);
    // 글의 바탕선(y)이 C++ drawText 의 기준이다 — egui 는 왼쪽 아래에 맞춘다.
    let caption = |x: f32, y: f32, word: &str, figure: String| {
        let w = p.text(at(x, y), egui::Align2::LEFT_BOTTOM, word, words.clone(), MUTED);
        p.text(Pos2::new(w.max.x, w.max.y), egui::Align2::LEFT_BOTTOM, figure, mono.clone(), MUTED);
    };
    caption(8.0, 12.0 + 2.0, "상면 ", elevation_level(top));
    caption(80.0, 71.0 + 2.0, "바닥 ", elevation_level(bottom));
    let depth_text = elevation_depth(top, bottom, depth).map_or_else(|| "—".to_owned(), |d| format!("{} m", metres(Some(d))));
    p.text(at(198.0, 42.0 + 2.0), egui::Align2::LEFT_BOTTOM, depth_text, mono, INK);
}
