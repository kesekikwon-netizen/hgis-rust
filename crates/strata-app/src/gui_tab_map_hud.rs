//! 지도 HUD — 상단 안내 띠, 좌상단 투명도 레일, 측정 HUD.
//!
//! `gui_tab_map_canvas` 에서 300줄을 지키려고 나눴다. 캔버스 본문의 뒤쪽 구간을
//! 그대로 옮겼고, 쓰던 지역변수(rect·p·is_style 등)는 인자로 받는다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;

const BAND_MARGIN: f32 = 12.0;
const BAND_HEIGHT: f32 = 36.0;
const BAND_BUTTON_GAP: f32 = 8.0;
const BAND_GLYPH_PX: f32 = 20.0;
const PILL_PAD_X: f32 = 10.0;
const PILL_ITEM_GAP: f32 = 6.0;

/// HUD 가 쓰는 도구 상태 묶음 — `gui_tab_map_canvas` 가 채워 넘긴다.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MapToolFlags {
    pub(crate) is_trench: bool,
    pub(crate) is_measure: bool,
    pub(crate) is_draw_tool: bool,
    pub(crate) is_select_tool: bool,
    pub(crate) is_style: bool,
    pub(crate) effective_tool: MapToolKind,
    pub(crate) cursor_world: Option<[f64; 2]>,
}

// Guide band content matching KaDrawGuideBand::setTool and setMeasure
// (target/cpp-ref/src/app/KaDrawGuideBand.h:23-44, target/cpp-ref/src/app/KaDrawGuideBand.cpp:129-158).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuideBandSpec {
    pub(crate) icon_id: &'static str,
    pub(crate) title: String,
    pub(crate) hint: String,
    pub(crate) measure: Option<String>,
}

impl StrataAppState {
    // Active tool icon, title, hint, and optional paper-look sketch measure for KaDrawGuideBand
    // (target/cpp-ref/src/app/KaDrawSketchTools.cpp:173-237, target/cpp-ref/src/app/MainWindowChrome.cpp:71-82,
    // target/cpp-ref/src/app/KaDrawGuideBand.cpp:129-158, target/cpp-ref/src/app/KaSketchMeasure.cpp:7-26).
    pub(crate) fn map_guide_band_spec(&self, input: MapToolFlags) -> Option<GuideBandSpec> {
        let MapToolFlags {
            is_trench,
            is_measure,
            is_draw_tool,
            is_select_tool,
            effective_tool,
            cursor_world,
            ..
        } = input;
        if is_draw_tool {
            let key = self.edit_layer_key.as_deref().unwrap_or("");
            // C++ currentToolIconId and currentToolLabel (target/cpp-ref/src/app/KaDrawSketchTools.cpp:178-199,220-227).
            let icon_id = if key == "control_points" {
                "gps"
            } else if key.starts_with("artifact") {
                "artifact"
            } else if key == "feature_line" || key == "section_line" {
                "draw_line"
            } else {
                "draw_poly"
            };
            let title = self.tool_chip_label();
            if title.trim().is_empty() {
                return None;
            }
            // C++ hintForTool (target/cpp-ref/src/app/MainWindowChrome.cpp:71-76).
            let hint = if icon_id == "gps" || icon_id == "artifact" {
                "클릭한 자리에 점을 놓습니다".to_owned()
            } else {
                "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned()
            };
            // KaDrawGuideBand::setMeasure shows figures only in KaTheme::displayOptions().paperLook
            // (target/cpp-ref/src/app/KaDrawGuideBand.cpp:129-135).
            let measure = if matches!(effective_tool, MapToolKind::DrawPolygon | MapToolKind::DrawLine)
                && self.display_options.paper_look
                && !self.draw_sketch.points.is_empty()
            {
                let mut measured = self.draw_sketch.points.clone();
                if let Some(c) = cursor_world {
                    measured.push(c);
                }
                let text = crate::draw_sketch::sketch_measure_text(
                    &measured,
                    effective_tool == MapToolKind::DrawPolygon,
                );
                (!text.is_empty()).then_some(text)
            } else {
                None
            };
            Some(GuideBandSpec { icon_id, title, hint, measure })
        } else if is_measure {
            Some(GuideBandSpec {
                icon_id: "measure",
                title: "줄자".to_owned(),
                hint: "두 점을 클릭하면 거리가, 셋 이상이면 면적이 나옵니다".to_owned(),
                measure: None,
            })
        } else if is_trench {
            Some(GuideBandSpec {
                icon_id: "trench_grid",
                title: "시굴 격자".to_owned(),
                hint: "격자를 놓을 원점을 클릭합니다".to_owned(),
                measure: None,
            })
        } else if is_select_tool {
            Some(GuideBandSpec {
                icon_id: "select",
                title: "도형선택".to_owned(),
                hint: "도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다".to_owned(),
                measure: None,
            })
        } else {
            None
        }
    }

    // Paint KaDrawGuideBand pill at (12, 12) with height 36 and return the top y for KaLayerOpacityRail
    // (target/cpp-ref/src/app/KaDrawGuideBand.h:23-29, target/cpp-ref/src/app/KaDrawGuideBand.cpp:28-53,142-158).
    pub(crate) fn paint_guide_band(
        &mut self,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        rect: Rect,
        spec: Option<&GuideBandSpec>,
    ) -> f32 {
        let Some(spec) = spec.filter(|s| !s.title.trim().is_empty()) else {
            return BAND_MARGIN;
        };
        let accent = Color32::from_rgb(181, 87, 58);
        let title_ink = Color32::from_rgb(28, 25, 23);
        let hint_ink = Color32::from_rgb(87, 83, 78);
        let has_glyph = !spec.icon_id.is_empty() && crate::ribbon_icon_svg::svg_parts(spec.icon_id).is_some();

        let title_galley = p.layout_no_wrap(spec.title.trim().to_owned(), FontId::proportional(12.0), title_ink);
        let hint_trimmed = spec.hint.trim();
        let hint_galley = (!hint_trimmed.is_empty())
            .then(|| p.layout_no_wrap(format!("— {hint_trimmed}"), FontId::proportional(12.0), hint_ink));
        let measure_galley = spec
            .measure
            .as_deref()
            .filter(|m| !m.is_empty())
            .map(|m| p.layout_no_wrap(m.to_owned(), FontId::monospace(12.0), title_ink));

        let mut content_w = title_galley.size().x;
        if has_glyph {
            content_w += BAND_GLYPH_PX + PILL_ITEM_GAP;
        }
        if let Some(hg) = &hint_galley {
            content_w += PILL_ITEM_GAP + hg.size().x;
        }
        if let Some(mg) = &measure_galley {
            content_w += PILL_ITEM_GAP + mg.size().x;
        }
        let pill_r = Rect::from_min_size(
            rect.min + Vec2::new(BAND_MARGIN, BAND_MARGIN),
            Vec2::new(PILL_PAD_X + content_w + PILL_PAD_X, BAND_HEIGHT),
        );
        p.rect_filled(pill_r, 6.0, Color32::from_rgba_unmultiplied(255, 255, 255, 248));
        p.rect_stroke(pill_r, 6.0, Stroke::new(1.2, accent), egui::StrokeKind::Middle);

        let mut x = pill_r.left() + PILL_PAD_X;
        if has_glyph {
            let glyph_r = Rect::from_min_size(
                Pos2::new(x, pill_r.center().y - BAND_GLYPH_PX * 0.5),
                Vec2::splat(BAND_GLYPH_PX),
            );
            let _ = crate::svg_path_paint::paint_svg_icon(p, glyph_r, spec.icon_id, accent);
            x += BAND_GLYPH_PX + PILL_ITEM_GAP;
        }
        let title_w = title_galley.size().x;
        p.galley(Pos2::new(x, pill_r.center().y - title_galley.size().y * 0.5), title_galley, title_ink);
        x += title_w;
        if let Some(hg) = hint_galley {
            x += PILL_ITEM_GAP;
            let hw = hg.size().x;
            p.galley(Pos2::new(x, pill_r.center().y - hg.size().y * 0.5), hg, hint_ink);
            x += hw;
        }
        if let Some(mg) = measure_galley {
            x += PILL_ITEM_GAP;
            p.galley(Pos2::new(x, pill_r.center().y - mg.size().y * 0.5), mg, title_ink);
        }

        self.paint_undo_redo_pill(ui, p, Pos2::new(pill_r.right() + BAND_BUTTON_GAP, pill_r.top()));
        BAND_MARGIN + BAND_HEIGHT + BAND_BUTTON_GAP
    }

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
            is_measure,
            is_style,
            ..
        } = input;
        // 1. KaDrawGuideBand (KaDrawGuideBand.cpp:28-70, MainWindowChrome.cpp:73-84)
        let guide_spec = self.map_guide_band_spec(input);
        let opacity_top_y = self.paint_guide_band(ui, p, rect, guide_spec.as_ref());

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

#[cfg(test)]
#[path = "gui_tab_map_hud_tests.rs"]
mod tests;
