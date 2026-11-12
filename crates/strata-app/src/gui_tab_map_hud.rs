//! 지도 HUD — 상단 안내 띠, 좌상단 투명도 레일, 측정 HUD.
//!
//! `gui_tab_map_canvas` 에서 300줄을 지키려고 나눴다. 캔버스 본문의 뒤쪽 구간을
//! 그대로 옮겼고, 쓰던 지역변수(rect·p·is_style 등)는 인자로 받는다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;


/// HUD 가 쓰는 도구 상태 묶음 — `gui_tab_map_canvas` 가 채워 넘긴다.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MapToolFlags {
    pub(crate) is_trench: bool,
    pub(crate) is_measure: bool,
    pub(crate) is_draw_tool: bool,
    pub(crate) is_select_tool: bool,
    pub(crate) is_style: bool,
    pub(crate) effective_tool: MapToolKind,
}

impl StrataAppState {
    /// 안내 띠·투명도 레일·측정 HUD (원문 캔버스 본문 뒤쪽).
    pub(crate) fn paint_map_hud(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        rect: Rect,
        input: MapToolFlags,
    ) {
        let MapToolFlags {
            is_trench,
            is_measure,
            is_draw_tool,
            is_select_tool,
            is_style,
            effective_tool,
        } = input;
        // 1. KaDrawGuideBand (KaDrawGuideBand.cpp:28-70, MainWindowChrome.cpp:73-84)
        let guide_info: Option<(String, String)> = if is_draw_tool {
            if effective_tool == MapToolKind::PlacePoint {
                Some((
                    "그리기 · 유물 위치".to_owned(),
                    "클릭한 자리에 점을 놓습니다".to_owned(),
                ))
            } else if effective_tool == MapToolKind::DrawLine {
                Some((
                    "그리기 · 유구 선".to_owned(),
                    "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned(),
                ))
            } else {
                // KaDrawSketchTools.cpp:257-259 — 띠 제목은 도구 칸 글(currentToolLabel) 그대로.
                let title = self.tool_chip_label();
                // KaDrawGuideBand.cpp:48-62 — m_measure 실시간 길이·면적
                let mut measure_txt = String::new();
                let ring = &self.draw_sketch.points;
                if ring.len() >= 2 {
                    let mut len_m = 0.0_f64;
                    for w in ring.windows(2) {
                        let dx = w[1][0] - w[0][0];
                        let dy = w[1][1] - w[0][1];
                        len_m += (dx * dx + dy * dy).sqrt();
                    }
                    measure_txt = if len_m >= 1000.0 {
                        format!("길이 {:.1} m ({:.2} km)", len_m, len_m / 1000.0)
                    } else {
                        format!("길이 {len_m:.1} m")
                    };
                    if ring.len() >= 3 {
                        let mut area = 0.0_f64;
                        for k in 0..ring.len() {
                            let a = ring[k];
                            let b = ring[(k + 1) % ring.len()];
                            area += a[0] * b[1] - b[0] * a[1];
                        }
                        measure_txt.push_str(&format!(
                            " · 면적 {:.1} ㎡",
                            area.abs() * 0.5
                        ));
                    }
                }
                if measure_txt.is_empty() {
                    Some((title, "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned()))
                } else {
                    Some((title, measure_txt))
                }
            }
        } else if is_measure {
            Some((
                "줄자".to_owned(),
                "두 점을 클릭하면 거리가, 셋 이상이면 면적이 나옵니다".to_owned(),
            ))
        } else if is_trench {
            Some((
                "시굴 격자".to_owned(),
                "격자를 놓을 원점을 클릭합니다".to_owned(),
            ))
        } else if is_select_tool {
            Some((
                "도형선택".to_owned(),
                "도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다".to_owned(),
            ))
        } else {
            None
        };

        let opacity_top_y = if let Some((title_txt, hint_txt)) = guide_info {
            let full_guide_txt = format!("{title_txt} — {hint_txt}");
            let pill_w =
                ((full_guide_txt.chars().count() as f32) * 9.5 + 42.0).clamp(320.0, 520.0);
            let pill_r = Rect::from_min_size(
                rect.min + Vec2::new(12.0, 12.0),
                Vec2::new(pill_w, 34.0),
            );
            p.rect_filled(
                pill_r,
                6.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 248),
            );
            p.rect_stroke(
                pill_r,
                6.0,
                Stroke::new(1.2, Color32::from_rgb(181, 87, 58)),
                egui::StrokeKind::Middle,
            );
            // KaDrawGuideBand.cpp:25 — KaIcons::tinted("draw-polygon", 16, KaTheme::palette().accent)
            let ic = Pos2::new(pill_r.left() + 16.0, pill_r.center().y);
            let accent = Color32::from_rgb(181, 87, 58);
            let poly_pts = [
                Pos2::new(ic.x - 5.0, ic.y - 4.0),
                Pos2::new(ic.x + 4.0, ic.y - 5.0),
                Pos2::new(ic.x + 5.0, ic.y + 4.0),
                Pos2::new(ic.x - 4.0, ic.y + 5.0),
                Pos2::new(ic.x - 5.0, ic.y - 4.0),
            ];
            p.add(egui::Shape::line(
                poly_pts.to_vec(),
                Stroke::new(1.5, accent),
            ));
            for &v in &poly_pts[..4] {
                p.circle_filled(v, 2.0, accent);
            }
            p.text(
                Pos2::new(pill_r.left() + 30.0, pill_r.center().y),
                egui::Align2::LEFT_CENTER,
                &full_guide_txt,
                FontId::proportional(12.0),
                Color32::from_rgb(28, 25, 23),
            );
            self.paint_undo_redo_pill(ui, p, Pos2::new(pill_r.right() + 8.0, pill_r.top()));
            12.0 + 34.0 + 8.0
        } else {
            12.0
        };

        // 2. KaLayerOpacityRail (KaLayerOpacityRail.cpp:18-177) — 좌상단 투명도(0~100) · 밝기(-255~255) 레일
        let selected_layer = self.right_inspector.selected_layer_key.clone();
        if let Some(layer_lbl) = selected_layer {
            let card_r = Rect::from_min_size(
                rect.min + Vec2::new(12.0, opacity_top_y),
                Vec2::new(260.0, 86.0),
            );
            self.paint_opacity_card(ctx, ui, p, card_r, &layer_lbl);
            if opacity_top_y <= 13.0 {
                self.paint_undo_redo_pill(
                    ui,
                    p,
                    Pos2::new(card_r.right() + 8.0, card_r.top()),
                );
            }
        } else {
            let idle_r = Rect::from_min_size(
                rect.min + Vec2::new(12.0, opacity_top_y),
                Vec2::new(260.0, 34.0),
            );
            p.rect_filled(
                idle_r,
                6.0,
                Color32::from_rgba_unmultiplied(252, 251, 247, 248),
            );
            p.rect_stroke(
                idle_r,
                6.0,
                Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                egui::StrokeKind::Middle,
            );
            p.text(
                Pos2::new(idle_r.left() + 12.0, idle_r.center().y),
                egui::Align2::LEFT_CENTER,
                "투명도·밝기: 조절할 레이어를 고르세요",
                FontId::proportional(11.5),
                Color32::from_rgb(65, 64, 60),
            );
            if opacity_top_y <= 13.0 {
                self.paint_undo_redo_pill(
                    ui,
                    p,
                    Pos2::new(idle_r.right() + 8.0, idle_r.top()),
                );
            }
        }

        if is_style && self.right_inspector.show_draw_tooltip {
            let tip_r = Rect::from_min_size(
                Pos2::new(rect.left() + 26.0, rect.top() - 26.0),
                Vec2::new(418.0, 32.0),
            );
            let tp = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("ka_draw_tip"),
            ));
            tp.rect_filled(tip_r, 5.0, Color32::from_rgb(35, 34, 32));
            tp.text(
                tip_r.center(),
                egui::Align2::CENTER_CENTER,
                "그리기 — 조사구역·유구 면과 선을 그립니다. 다시 누르면 도구를 닫고 이동합니다",
                FontId::proportional(11.5),
                Color32::WHITE,
            );
        }
        if is_measure {
            self.paint_map_measure_hud(ui, p, rect);
        }
    }
}
