//! 레이어 한 줄에 쓰는 체크상자·기호 견본을 그린다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
#[derive(Clone, Copy)]
pub(crate) enum SwatchKind {
    CadastralOutline,
    TrenchBeige,
    RasterChecker,
    SurveyOrange,
    DistanceBox,
    BufferBand,
    GroupFolder,
    HeritageSub,
}
pub(crate) fn paint_checkbox_state(p: &egui::Painter, center: Pos2, check_state: u8) {
    let r = Rect::from_center_size(center, Vec2::new(13.0, 13.0));
    if check_state == 2 {
        p.rect_filled(r, 2.5, Color32::from_rgb(28, 28, 26));
        let c = r.center();
        p.line_segment(
            [
                Pos2::new(c.x - 3.2, c.y + 0.2),
                Pos2::new(c.x - 0.8, c.y + 2.8),
            ],
            Stroke::new(1.7, Color32::WHITE),
        );
        p.line_segment(
            [
                Pos2::new(c.x - 0.8, c.y + 2.8),
                Pos2::new(c.x + 3.6, c.y - 2.6),
            ],
            Stroke::new(1.7, Color32::WHITE),
        );
    } else if check_state == 1 {
        p.rect_filled(r, 2.5, Color32::from_rgb(88, 86, 80));
        let c = r.center();
        p.line_segment(
            [Pos2::new(c.x - 3.5, c.y), Pos2::new(c.x + 3.5, c.y)],
            Stroke::new(2.0, Color32::WHITE),
        );
    } else {
        p.rect_filled(r, 2.5, Color32::WHITE);
        p.rect_stroke(
            r,
            2.5,
            Stroke::new(1.2, Color32::from_rgb(135, 133, 126)),
            egui::StrokeKind::Middle,
        );
    }
}
pub(crate) fn paint_symbol_swatch(p: &egui::Painter, center: Pos2, kind: SwatchKind) {
    let r = Rect::from_center_size(center, Vec2::new(15.0, 11.0));
    match kind {
        SwatchKind::CadastralOutline => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(200, 197, 188)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::TrenchBeige => {
            p.rect_filled(r, 1.0, Color32::from_rgb(237, 226, 216));
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.1, Color32::from_rgb(38, 37, 34)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::RasterChecker => {
            let s = 4.5;
            let tl = Pos2::new(center.x - s, center.y - s);
            p.rect_filled(
                Rect::from_min_size(tl, Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(56, 108, 176),
            );
            p.rect_filled(
                Rect::from_min_size(tl + Vec2::new(s, 0.0), Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(35, 35, 32),
            );
            p.rect_filled(
                Rect::from_min_size(tl + Vec2::new(0.0, s), Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(35, 35, 32),
            );
            p.rect_filled(
                Rect::from_min_size(tl + Vec2::new(s, s), Vec2::new(s, s)),
                0.0,
                Color32::from_rgb(135, 168, 210),
            );
        }
        SwatchKind::SurveyOrange => {
            p.rect_filled(r, 1.0, Color32::from_rgb(245, 208, 182));
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(2.0, Color32::from_rgb(214, 78, 22)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::DistanceBox => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.1, Color32::from_rgb(38, 37, 34)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::BufferBand => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            let top_band =
                Rect::from_min_size(r.min + Vec2::new(1.0, 2.0), Vec2::new(r.width() - 2.0, 3.0));
            p.rect_filled(top_band, 0.0, Color32::from_rgb(176, 62, 26));
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.1, Color32::from_rgb(38, 37, 34)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::GroupFolder => {
            let b1 = Rect::from_center_size(center + Vec2::new(1.5, 1.0), Vec2::new(10.0, 9.0));
            let b0 = Rect::from_center_size(center - Vec2::new(1.5, 1.0), Vec2::new(10.0, 9.0));
            p.rect_filled(b1, 1.0, Color32::WHITE);
            p.rect_stroke(
                b1,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(95, 94, 88)),
                egui::StrokeKind::Middle,
            );
            p.rect_filled(b0, 1.0, Color32::from_rgb(220, 230, 244));
            p.rect_stroke(
                b0,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(72, 98, 138)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::HeritageSub => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
                egui::StrokeKind::Middle,
            );
        }
    }
}
