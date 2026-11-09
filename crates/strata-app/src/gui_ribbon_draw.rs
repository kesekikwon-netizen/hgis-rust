//! 리본 칩·배경 알약 그리기 — C++ `KaBeginnerRibbon`·`KaRibbonChip` 대응.

use eframe::egui::{ self, Color32, Pos2, Rect, Stroke, Vec2 };

use crate::app_state::StrataAppState;
use crate::ribbon_icons::{cpp_ribbon_tooltip, paint_ribbon_icon};

pub(crate) fn draw_buffer_pill(
    ui: &mut egui::Ui,
    icon_key: &str,
    badge_bg: bool,
    text: &str,
    width: f32,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), egui::Sense::click());
    let p = ui.painter_at(r);
    p.rect_filled(r, 6.0, Color32::WHITE);
    p.rect_stroke(
        r,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
        egui::StrokeKind::Middle,
    );
    if !icon_key.is_empty() {
        let ir = Rect::from_center_size(
            Pos2::new(r.left() + 16.0, r.center().y),
            Vec2::new(20.0, 20.0),
        );
        if badge_bg {
            p.rect_filled(ir, 4.0, Color32::from_rgb(236, 233, 224));
        }
        paint_ribbon_icon(&p, ir.expand(2.0), icon_key, Color32::from_rgb(38, 37, 34));
        p.text(
            Pos2::new(r.left() + 31.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(12.5),
            Color32::from_rgb(30, 30, 28),
        );
    } else {
        p.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.5),
            Color32::from_rgb(30, 30, 28),
        );
    }
    resp.clicked()
}
/// C++ `BasemapDsm.h:21` `copernicusTitle()` = `지표모델(DSM) 30m · Copernicus`.
/// Rust 범례(`gui_layer_panel.rs`)는 이 이름을 `지표모델(DSM) 30m · Coperni...` 로 줄여
/// 쓰므로 DEM 토글은 두 키를 함께 맞춘다.
pub(crate) const DEM_LEGEND_KEY: &str = "지표모델(DSM) 30m · Coperni...";
impl StrataAppState {
    pub(crate) fn draw_ribbon_chip(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        muted: bool,
        highlight_save: bool,
        dark_active: bool,
        warm_active: bool,
    ) -> (bool, bool) {
        let target_rect = ui.max_rect();
        let (_, resp) = ui.allocate_exact_size(target_rect.size(), egui::Sense::click());
        let rect = target_rect;
        let p = ui.painter_at(rect);
        // KaBeginnerRibbon.cpp:229-250 — 창 너비에 맞춰 tile 크기를 32..56 범위에서 결정.
        let tile_side = (rect.width() - 6.0)
            .min(rect.height() - 28.0)
            .clamp(26.0, 56.0);
        let icon_box = Rect::from_min_size(
            Pos2::new(rect.center().x - tile_side * 0.5, rect.top() + 3.0),
            Vec2::new(tile_side, tile_side),
        );
        let corner = tile_side * 0.25;
        let is_hovered = resp.hovered()
            && ui
                .input(|i| i.pointer.hover_pos())
                .is_none_or(|pos| rect.contains(pos));
        let paper = self.display_options.paper_look;
        if highlight_save {
            let strong_bg = if paper {
                Color32::from_rgb(0xB5, 0x57, 0x3A)
            } else {
                Color32::from_rgb(0x20, 0x6C, 0xB0)
            };
            p.rect_filled(icon_box, corner, strong_bg);
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                4.2,
                Color32::WHITE,
            );
            p.circle_filled(
                Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5),
                2.8,
                Color32::from_rgb(0xF2, 0xA3, 0x3A),
            );
        } else if dark_active {
            let (on_bg, on_fg) = if paper {
                (
                    Color32::from_rgb(0x30, 0x30, 0x2E),
                    Color32::from_rgb(0xFA, 0xF9, 0xF5),
                )
            } else {
                (
                    Color32::from_rgb(0xE0, 0xEC, 0xF8),
                    Color32::from_rgb(0x10, 0x50, 0x88),
                )
            };
            p.rect_filled(icon_box, corner, on_bg);
            if !paper {
                p.rect_stroke(
                    icon_box,
                    corner,
                    Stroke::new(1.5, Color32::from_rgb(0x1A, 0x68, 0xB0)),
                    egui::StrokeKind::Inside,
                );
            }
            paint_ribbon_icon(&p, icon_box, label, on_fg);
        } else {
            // KaThemePaper.cpp:60-70 vs KaTheme.cpp:105-113
            let (bg, stroke_col) = if muted {
                if paper {
                    (
                        Color32::from_rgb(0xF5, 0xF4, 0xED),
                        Color32::from_rgb(0x9E, 0x9D, 0x98),
                    )
                } else {
                    (
                        Color32::from_rgb(0xEE, 0xF1, 0xF3),
                        Color32::from_rgb(0x8F, 0x98, 0xA3),
                    )
                }
            } else if warm_active {
                if paper {
                    (
                        Color32::from_rgb(0xE3, 0xDA, 0xCC),
                        Color32::from_rgb(0x14, 0x14, 0x13),
                    )
                } else {
                    (
                        Color32::from_rgb(0xE0, 0xEC, 0xF8),
                        Color32::from_rgb(0x10, 0x50, 0x88),
                    )
                }
            } else if is_hovered {
                if paper {
                    (
                        Color32::from_rgb(0xE8, 0xE6, 0xDC),
                        Color32::from_rgb(0x30, 0x30, 0x2E),
                    )
                } else {
                    (
                        Color32::from_rgb(0xD9, 0xE2, 0xE7),
                        Color32::from_rgb(0x2B, 0x48, 0x58),
                    )
                }
            } else if paper {
                (
                    Color32::from_rgb(0xF0, 0xEE, 0xE6),
                    Color32::from_rgb(0x30, 0x30, 0x2E),
                )
            } else {
                (
                    Color32::from_rgb(0xE4, 0xEA, 0xED),
                    Color32::from_rgb(0x2B, 0x48, 0x58),
                )
            };
            p.rect_filled(icon_box, corner, bg);
            if warm_active {
                let bw = (tile_side * 2.0 / 32.0).max(1.0);
                let border_col = if paper {
                    Color32::from_rgb(0xB0, 0xAE, 0xA5)
                } else {
                    Color32::from_rgb(0x1A, 0x68, 0xB0)
                };
                p.rect_stroke(
                    icon_box,
                    corner,
                    Stroke::new(bw, border_col),
                    egui::StrokeKind::Inside,
                );
            }
            paint_ribbon_icon(&p, icon_box, label, stroke_col);
        }
        let txt_col = if muted {
            Color32::from_rgb(0x9E, 0x9D, 0x98)
        } else {
            Color32::from_rgb(0x14, 0x14, 0x13)
        };
        p.text(
            Pos2::new(rect.center().x, rect.top() + tile_side + 15.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.5),
            txt_col,
        );
        let resp = resp.on_hover_text(cpp_ribbon_tooltip(label));
        let clicked = resp.clicked()
            && resp
                .interact_pointer_pos()
                .is_none_or(|pos| rect.contains(pos));
        let secondary = resp.secondary_clicked();
        (clicked, secondary)
    }
}
