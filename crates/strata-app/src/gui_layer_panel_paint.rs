//! 레이어 한 줄에 쓰는 체크상자·기호 견본을 그린다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SwatchKind {
    CadastralOutline,
    RasterChecker,
    BufferBand,
    HeritageSub,
    /// 조사 면 레이어 — C++ `LayerStyleDefaults::forKey` 채움(투명도 포함)·외곽선·굵기(mm).
    DomainFill { fill: Color32, stroke: Color32, stroke_mm: f32 },
    /// 조사 선 레이어 — 굵기(mm). 단면선은 흰 테두리(casing 3 mm) 위에 색 선.
    DomainLine { stroke: Color32, width_mm: f32, casing: bool },
    /// 조사 점 레이어 — 채운 원·테두리·크기(mm).
    DomainMarker { fill: Color32, stroke: Color32, size_mm: f32 },
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
        SwatchKind::HeritageSub => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(
                r,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
                egui::StrokeKind::Middle,
            );
        }
        SwatchKind::DomainFill { .. }
        | SwatchKind::DomainLine { .. }
        | SwatchKind::DomainMarker { .. } => paint_framed_domain_swatch(p, center, kind),
    }
}

/// C++ 범례의 조사 레이어 견본(150% 캡처를 잰 모양): 20×14 상자에 1.33 px 먹색 테두리, 기호는
/// 행 배경 위에 그리고 상자 밖은 잘린다. 면은 외곽선이 상자 가장자리에 걸쳐 안쪽 절반만 보이고
/// (0.5 mm 시굴격자는 테두리에 가린다), 점은 지름 크기×3.78+2 px, 선은 굵기×3.78 px 띠다.
fn paint_framed_domain_swatch(p: &egui::Painter, center: Pos2, kind: SwatchKind) {
    use super::gui_layer_panel_consts::{UX_LEGEND_SWATCH_FRAME, UX_LEGEND_SWATCH_SIZE};
    const PX_PER_MM: f32 = 96.0 / 25.4;
    const FRAME_W: f32 = 4.0 / 3.0;
    let frame = Rect::from_center_size(center, Vec2::from(UX_LEGEND_SWATCH_SIZE));
    let clip = p.with_clip_rect(frame);
    match kind {
        SwatchKind::DomainFill { fill, stroke, stroke_mm } => {
            clip.rect_filled(frame, 0.0, fill);
            clip.rect_stroke(frame, 0.0, Stroke::new(stroke_mm * PX_PER_MM, stroke), egui::StrokeKind::Middle);
        }
        SwatchKind::DomainLine { stroke, width_mm, casing } => {
            let ends = [Pos2::new(frame.left(), center.y), Pos2::new(frame.right(), center.y)];
            if casing {
                clip.line_segment(ends, Stroke::new(3.0 * PX_PER_MM, Color32::WHITE));
            }
            clip.line_segment(ends, Stroke::new(width_mm * PX_PER_MM, stroke));
        }
        SwatchKind::DomainMarker { fill, stroke, size_mm } => {
            let r = size_mm * PX_PER_MM * 0.5;
            clip.circle_filled(center, r, fill);
            clip.circle_stroke(center, r, Stroke::new(2.0, stroke));
        }
        _ => {}
    }
    p.rect_stroke(frame, 0.0, Stroke::new(FRAME_W, UX_LEGEND_SWATCH_FRAME), egui::StrokeKind::Inside);
}
