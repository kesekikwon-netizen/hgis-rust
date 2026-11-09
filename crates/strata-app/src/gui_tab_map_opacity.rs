//! `KaLayerOpacityRail.cpp:18-177` — 좌상단 투명도(0~100)·밝기(-255~255) 레일.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};

use crate::app_state::StrataAppState;

use super::gui_tab_map_format::BrightRow;
use super::gui_tab_map_geom::{brightness_row_text, is_raster_image_layer};
/// `KaLayerOpacityRail.cpp:18-177` — 좌상단 투명도(0~100) · 밝기(-255~255) 레일
impl StrataAppState {
    pub(crate) fn paint_opacity_card(
    &mut self,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    p: &egui::Painter,
    card_r: Rect,
    title: &str,
) {
    p.rect_filled(
        card_r,
        6.0,
        Color32::from_rgba_unmultiplied(252, 251, 247, 248),
    );
    p.rect_stroke(
        card_r,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
        egui::StrokeKind::Middle,
    );
    p.text(
        Pos2::new(card_r.left() + 12.0, card_r.top() + 16.0),
        egui::Align2::LEFT_CENTER,
        title,
        FontId::proportional(12.0),
        Color32::from_rgb(28, 25, 23),
    );

    let opacity_val = self.home_map.real_survey.layer_opacity.clamp(0.0, 1.0);
    let bright_val = self.home_map.real_survey.layer_brightness.clamp(0.0, 1.0);
    let opacity_pct = (opacity_val * 100.0).round() as i32;
    let bright_int = ((bright_val - 0.5) * 2.0 * 255.0).round() as i32;
    let opacity_txt = format!("{opacity_pct}%");
    // KaLayerOpacityRail.cpp:156-173 — 벡터(지적도·유적 경계)에는 밝기가 없다
    let is_raster = is_raster_image_layer(title);
    let (bright_txt, bright_tip) = brightness_row_text(bright_int, is_raster);

    for (row_i, (lbl, pct_txt, knob_t, col, tip)) in [
        (
            "투명도",
            opacity_txt,
            opacity_val,
            Color32::from_rgb(181, 87, 58),
            "선택한 레이어의 투명도. 오른쪽으로 밀면 진해집니다. 지도에서만 흐려지고 자료는 그대로입니다.",
        ),
        (
            "밝기",
            bright_txt,
            bright_val,
            Color32::from_rgb(92, 90, 84),
            bright_tip,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let is_bright_row = row_i == 1;
        let row_kind = if is_bright_row {
            if is_raster {
                BrightRow::Raster
            } else {
                BrightRow::Vector
            }
        } else {
            BrightRow::Opacity
        };
        let y = card_r.top() + 40.0 + (row_i as f32) * 24.0;
        p.text(
            Pos2::new(card_r.left() + 12.0, y),
            egui::Align2::LEFT_CENTER,
            lbl,
            FontId::proportional(11.5),
            Color32::from_rgb(65, 64, 60),
        );
        let (sx0, sx1) = (card_r.left() + 58.0, card_r.right() - 48.0);
        let kx = sx0 + (sx1 - sx0) * knob_t;
        // 벡터 레이어의 밝기 줄은 값 자리에 「그림만」만 두고 손잡이를 없앤다
        if !matches!(row_kind, BrightRow::Vector) {
            p.line_segment([Pos2::new(sx0, y), Pos2::new(kx, y)], Stroke::new(3.5, col));
            p.line_segment(
                [Pos2::new(kx, y), Pos2::new(sx1, y)],
                Stroke::new(3.5, Color32::from_rgb(208, 205, 196)),
            );
            p.circle_filled(Pos2::new(kx, y), 6.0, Color32::WHITE);
            p.circle_stroke(Pos2::new(kx, y), 6.0, Stroke::new(2.0, col));
        } else {
            p.line_segment(
                [Pos2::new(sx0, y), Pos2::new(sx1, y)],
                Stroke::new(3.5, Color32::from_rgb(208, 205, 196)),
            );
        }
        p.text(
            Pos2::new(card_r.right() - 10.0, y),
            egui::Align2::RIGHT_CENTER,
            &pct_txt,
            FontId::proportional(11.0),
            Color32::from_rgb(45, 44, 40),
        );

        let track_r =
            Rect::from_min_max(Pos2::new(sx0 - 6.0, y - 10.0), Pos2::new(sx1 + 6.0, y + 10.0));
        let resp = if matches!(row_kind, BrightRow::Vector) {
            // 벡터 밝기는 끌 수 없다 (KaLayerOpacityRail.cpp:158-166). 안내말만 남긴다.
            ui.interact(
                track_r,
                ui.id().with(("ka_opacity_slider", row_i)),
                egui::Sense::hover(),
            )
        } else {
            ui.interact(
                track_r,
                ui.id().with(("ka_opacity_slider", row_i)),
                egui::Sense::click_and_drag(),
            )
        }
        .on_hover_text(tip);
        if !matches!(row_kind, BrightRow::Vector) && (resp.clicked() || resp.dragged()) {
            if let Some(pos) = resp.interact_pointer_pos() {
                let t = ((pos.x - sx0) / (sx1 - sx0).max(1.0)).clamp(0.0, 1.0);
                if row_i == 0 {
                    self.home_map.real_survey.layer_opacity = t;
                    self.home_map.set_basemap_opacity(t);
                } else {
                    self.home_map.real_survey.layer_brightness = t;
                    self.home_map.set_basemap_brightness((t - 0.5) * 2.0);
                }
                ctx.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new(("ka_layer_op_br", title)),
                        (
                            self.home_map.real_survey.layer_opacity,
                            self.home_map.real_survey.layer_brightness,
                        ),
                    );
                });
            }
        }
    }
}
}
