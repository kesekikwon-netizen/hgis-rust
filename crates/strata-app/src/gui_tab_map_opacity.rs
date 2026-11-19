//! `KaLayerOpacityRail.cpp:18-177` — 좌상단 투명도(0~100)·밝기(-255~255) 레일.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};

use crate::app_state::StrataAppState;

#[path = "gui_tab_drawing_opacity.rs"]
mod drawing_rail;
#[path = "gui_tab_map_opacity_keys.rs"]
mod keys;
#[path = "gui_tab_map_opacity_placement.rs"]
mod placement;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SliderControl {
    Opacity,
    Brightness,
}

/// Map rail and drawing rail share the card. Their places and remembered layers stay apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RailKind {
    Map,
    Drawing,
}

fn rail_index(kind: RailKind) -> usize {
    match kind {
        RailKind::Map => 0,
        RailKind::Drawing => 1,
    }
}

fn slider_rect(card: Rect, row: usize) -> Rect {
    let y = card.top() + 40.0 + row as f32 * 24.0;
    Rect::from_min_max(Pos2::new(card.left() + 52.0, y - 10.0), Pos2::new(card.right() - 42.0, y + 10.0))
}

use super::gui_tab_map_format::BrightRow;
use super::gui_tab_map_geom::{brightness_row_text, is_raster_image_layer};
/// `KaLayerOpacityRail.cpp:18-177` — 좌상단 투명도(0~100) · 밝기(-255~255) 레일
impl StrataAppState {
    /// Legend row whose Korean title is the current layer name (`MainWindow.cpp:991` `currentLayer()`).
    fn legend_row_index(&self, title: &str) -> Option<usize> {
        self.left_layers.entries.iter().position(|e| e.label_ko == title)
    }

    /// What the 투명도 rail shows for the selected row (0 = clear, 1 = solid).
    pub(crate) fn rail_opacity(&self, title: &str) -> f32 {
        self.legend_row_index(title)
            .map_or(self.home_map.real_survey.layer_opacity, |i| self.left_layers.entries[i].opacity)
    }

    /// Brightness exists for a raster. On the drawing card a picture is one even when its title is not 위성
    /// (`BasemapOps.cpp:2311-2314`).
    fn rail_brightness_enabled(&self, kind: RailKind, title: &str) -> bool {
        is_raster_image_layer(title) || (kind == RailKind::Drawing && self.picture_index(title).is_some())
    }

    fn picture_index(&self, title: &str) -> Option<usize> {
        let key = self.left_layers.entries.iter().find(|e| e.label_ko == title)?.layer_key.as_str();
        self.pictures.layers.iter().position(|layer| layer.key == key)
    }

    /// The 투명도 rail was moved to `t` for the selected row. C++ applies it to that layer only
    /// (`MainWindow.cpp:989-1000` `LayerOps::applyLayerOpacity`), the same property the legend menu sets
    /// (`MainWindowContextMenus.cpp:381`). A title with no legend row is left alone (`canAdjustOpacity` is false).
    pub(crate) fn apply_rail_opacity(&mut self, title: &str, t: f32) {
        let Some(i) = self.legend_row_index(title) else {
            return;
        };
        self.left_layers.entries[i].opacity = t.clamp(0.0, 1.0);
    }

    fn paint_opacity_card(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, p: &egui::Painter, card_r: Rect, title: &str, kind: RailKind) {
        p.rect_filled(card_r, 6.0, Color32::from_rgba_unmultiplied(252, 251, 247, 248));
        p.rect_stroke(card_r, 6.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
        p.text(
            Pos2::new(card_r.left() + 12.0, card_r.top() + 16.0),
            egui::Align2::LEFT_CENTER,
            title,
            FontId::proportional(12.0),
            Color32::from_rgb(28, 25, 23),
        );

        let opacity_val = self.rail_opacity(title).clamp(0.0, 1.0);
        let bright_val = match kind {
            RailKind::Map => self.home_map.real_survey.layer_brightness,
            RailKind::Drawing => self.drawing_brightness_unit(title),
        }
        .clamp(0.0, 1.0);
        let is_raster = self.rail_brightness_enabled(kind, title);
        let opacity_pct = (opacity_val * 100.0).round() as i32;
        let bright_int = ((bright_val - 0.5) * 2.0 * 255.0).round() as i32;
        let opacity_txt = format!("{opacity_pct}%");
        // KaLayerOpacityRail.cpp:156-173 — 벡터(지적도·유적 경계)에는 밝기가 없다.
        // A drawing picture is a raster even when its title does not say 위성 (`BasemapOps.cpp:2311-2314`).
        let (bright_txt, bright_tip) = brightness_row_text(bright_int, is_raster);

        for (row_i, (lbl, pct_txt, knob_t, col, tip)) in [
            (
                "투명도",
                opacity_txt,
                opacity_val,
                Color32::from_rgb(181, 87, 58),
                "선택한 레이어의 투명도. 오른쪽으로 밀면 진해집니다. 지도에서만 흐려지고 자료는 그대로입니다.",
            ),
            ("밝기", bright_txt, bright_val, Color32::from_rgb(92, 90, 84), bright_tip),
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
                p.line_segment([Pos2::new(kx, y), Pos2::new(sx1, y)], Stroke::new(3.5, Color32::from_rgb(208, 205, 196)));
                p.circle_filled(Pos2::new(kx, y), 6.0, Color32::WHITE);
                p.circle_stroke(Pos2::new(kx, y), 6.0, Stroke::new(2.0, col));
            } else {
                p.line_segment([Pos2::new(sx0, y), Pos2::new(sx1, y)], Stroke::new(3.5, Color32::from_rgb(208, 205, 196)));
            }
            p.text(
                Pos2::new(card_r.right() - 10.0, y),
                egui::Align2::RIGHT_CENTER,
                &pct_txt,
                FontId::proportional(11.0),
                Color32::from_rgb(45, 44, 40),
            );

            let track_r = slider_rect(card_r, row_i);
            let control = match row_kind {
                BrightRow::Opacity => Some(SliderControl::Opacity),
                BrightRow::Raster => Some(SliderControl::Brightness),
                BrightRow::Vector => None,
            };
            let slider_id = match kind {
                RailKind::Map => ui.id().with(("ka_opacity_slider", row_i)),
                RailKind::Drawing => ui.id().with(("ka_drawing_opacity_slider", row_i)),
            };
            let resp = if matches!(row_kind, BrightRow::Vector) {
                // 벡터 밝기는 끌 수 없다 (KaLayerOpacityRail.cpp:158-166). 안내말만 남긴다.
                ui.interact(track_r, slider_id, egui::Sense::hover())
            } else {
                keys::interact(ui, track_r, slider_id, if row_i == 0 { SliderControl::Opacity } else { SliderControl::Brightness }, kind)
            }
            .on_hover_text(tip);
            if resp.has_focus() {
                p.rect_stroke(track_r.expand(2.0), 4.0, Stroke::new(1.0, Color32::from_rgb(181, 87, 58)), egui::StrokeKind::Inside);
            }
            let pointer_t = (control.is_some() && (resp.clicked_by(egui::PointerButton::Primary) || resp.dragged_by(egui::PointerButton::Primary)))
                .then(|| resp.interact_pointer_pos())
                .flatten()
                .map(|pos| ((pos.x - sx0) / (sx1 - sx0).max(1.0)).clamp(0.0, 1.0));
            let keyboard_t = control.and_then(|control| keys::keyboard_value(ui, &resp, control, pointer_t.unwrap_or(knob_t)));
            if let Some(t) = keyboard_t.or(pointer_t) {
                ctx.request_repaint();
                match kind {
                    RailKind::Map => {
                        if row_i == 0 {
                            self.apply_rail_opacity(title, t);
                        } else {
                            self.home_map.real_survey.layer_brightness = t;
                            self.home_map.set_basemap_brightness((t - 0.5) * 2.0);
                        }
                        let shown = self.rail_opacity(title);
                        ctx.data_mut(|d| {
                            d.insert_temp(egui::Id::new(("ka_layer_op_br", title)), (shown, self.home_map.real_survey.layer_brightness));
                        });
                    }
                    RailKind::Drawing => {
                        if row_i == 0 {
                            self.apply_drawing_rail_opacity(title, t);
                        } else {
                            self.apply_drawing_rail_brightness(title, t);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_map_opacity_layer_tests.rs"]
mod layer_tests;

#[cfg(test)]
#[path = "opacity_rail_disk_tests.rs"]
mod disk_tests;
