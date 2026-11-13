//! 지도 오버레이 — 우상단 줌 단추, 좌하단 도북 화살표·축척막대, 우하단 좌표격자.
//!
//! `gui_tab_map_canvas` 에서 300줄을 지키려고 나눴다. 캔버스 본문 뒤쪽 구간을
//! 그대로 옮겼고, 쓰던 지역변수(p·rect·is_style)는 인자로 받는다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_map_geom::{scale_bar_metres, scale_bar_tick_text};

impl StrataAppState {
    /// 줌 단추·도북 화살표·축척막대·좌표격자 컨트롤 (원문 캔버스 본문 뒤쪽).
    pub(crate) fn paint_map_overlays(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        rect: Rect,
    ) {
        // KaMapControls.cpp:70-109 — 우상단 + / − / 맞춤 버튼
        let zoom_r = Rect::from_min_size(
            Pos2::new(rect.right() - 44.0, rect.top() + 12.0),
            Vec2::new(32.0, 96.0),
        );
        p.rect_filled(zoom_r, 5.0, Color32::WHITE);
        p.rect_stroke(
            zoom_r,
            5.0,
            Stroke::new(1.0, Color32::from_rgb(190, 188, 178)),
            egui::StrokeKind::Middle,
        );
        for (i, (sym, tip)) in [
            ("+", "확대 (마우스 휠을 앞으로)"),
            ("−", "축소 (마우스 휠을 뒤로)"),
            ("□", "조사구역이 화면에 꽉 차게 봅니다"),
        ]
        .into_iter()
        .enumerate()
        {
            let zr = Rect::from_min_size(
                Pos2::new(zoom_r.left(), zoom_r.top() + (i as f32) * 32.0),
                Vec2::new(32.0, 32.0),
            );
            if i > 0 {
                p.line_segment(
                    [
                        Pos2::new(zr.left() + 4.0, zr.top()),
                        Pos2::new(zr.right() - 4.0, zr.top()),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(220, 218, 210)),
                );
            }
            p.text(
                zr.center(),
                egui::Align2::CENTER_CENTER,
                sym,
                FontId::proportional(15.0),
                Color32::from_rgb(45, 44, 40),
            );
            if ui
                .interact(zr, ui.id().with(("map_zoom_btn", i)), egui::Sense::click())
                .on_hover_text(tip)
                .clicked()
            {
                let center_px = [
                    f64::from(rect.width()) * 0.5,
                    f64::from(rect.height()) * 0.5,
                ];
                match i {
                    0 => self.home_map.viewport.zoom_at(0.8, center_px),
                    1 => self.home_map.viewport.zoom_at(1.25, center_px),
                    _ => self.fit_to_survey_area(),
                }
            }
        }

        // KaMapScaleBar (KaMapControls.cpp:138-260) — 좌하단 도북 화살표 · 1·2·5 decade 축척 막대
        let bl = Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0);
        p.text(
            bl,
            egui::Align2::LEFT_CENTER,
            "▲ 도북",
            FontId::proportional(11.0),
            Color32::from_rgb(45, 44, 40),
        );
        if let Some(round_m) = scale_bar_metres(self.home_map.viewport.meters_per_pixel) {
            let per_pixel = self.home_map.viewport.meters_per_pixel.max(1e-6);
            let pixels = (round_m / per_pixel).clamp(40.0, 220.0) as f32;
            let sb = Rect::from_min_size(
                Pos2::new(bl.x + 48.0, bl.y - 4.0),
                Vec2::new(pixels, 6.0),
            );
            // 인쇄 축척 막대처럼 진한 칸·흰 칸을 번갈아 네 칸
            let ink = Color32::from_rgb(35, 35, 32);
            let quarter = sb.width() / 4.0;
            for i in 0..4 {
                p.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(sb.left() + quarter * i as f32, sb.top()),
                        Vec2::new(quarter, sb.height()),
                    ),
                    0.0,
                    if i % 2 == 0 { ink } else { Color32::WHITE },
                );
            }
            p.rect_stroke(sb, 0.0, Stroke::new(1.0, ink), egui::StrokeKind::Middle);
            let (start_txt, mid_txt, end_txt) = scale_bar_tick_text(round_m);
            let tick_font = FontId::monospace(10.0);
            p.text(
                Pos2::new(sb.left(), sb.top() - 7.0),
                egui::Align2::LEFT_CENTER,
                start_txt,
                tick_font.clone(),
                Color32::from_rgb(45, 44, 40),
            );
            p.text(
                Pos2::new(sb.center().x, sb.top() - 7.0),
                egui::Align2::CENTER_CENTER,
                mid_txt,
                tick_font.clone(),
                Color32::from_rgb(45, 44, 40),
            );
            p.text(
                Pos2::new(sb.right(), sb.top() - 7.0),
                egui::Align2::RIGHT_CENTER,
                end_txt,
                tick_font,
                Color32::from_rgb(45, 44, 40),
            );
        }

        // 3. KaShellGridControls (KaShellGridControls.cpp:41-193) — 우하단 좌표격자 토글 · 격자 종류 · 간격 선택 · 라벨 표시 · 선 색상
        self.paint_shell_grid_controls(ctx, ui, p, rect);
    }
}
