use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::right_inspector::InspectorCardTab;

impl StrataAppState {
    /// C++ `MainWindow.cpp` 지도 탭 (Images 2, 10, 11, 15, 16) 1:1 재현
    pub(crate) fn render_map_tab_ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        self.render_cpp_left_layer_panel(ui, false);
        self.render_cpp_right_inspector(ui);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let is_trench = self.modals.active == Some(ActiveModalDialog::TrenchGenerator) || self.active_tool == MapToolKind::TrenchPlace;
            let is_measure = self.active_tool == MapToolKind::Measure;
            let is_style = self.right_inspector.active_card == InspectorCardTab::LayerStyleCard;
            if is_trench {
                egui::Frame::NONE.fill(Color32::from_rgb(225, 240, 218)).stroke(Stroke::new(1.0, Color32::from_rgb(145, 195, 138))).inner_margin(egui::Margin::symmetric(10, 4)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let _ = ui.small_button("II");
                        ui.colored_label(Color32::from_rgb(22, 138, 42), "✔");
                        ui.label(egui::RichText::new("시굴격자: 시굴격자 232개 · 총 8,686.58m² · 조사구역의 10.0% (시굴 기준 10%) — 격자...").size(12.0).strong());
                    });
                });
            }

            let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            if response.dragged() {
                let d = response.drag_delta();
                self.home_map.viewport.pan_by_pixels(f64::from(d.x), f64::from(d.y));
            }
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 0.0, Color32::from_rgb(251, 250, 246));
            self.paint_gyeongju_heritage_canvas(&p, rect, is_trench || is_measure || is_style);

            if is_style {
                let card_r = Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::new(236.0, 78.0));
                let layer_lbl = self.right_inspector.selected_layer_key.as_deref().unwrap_or("조사구역");
                Self::paint_opacity_card(&p, card_r, layer_lbl);
                Self::paint_undo_redo_pill(&p, Pos2::new(card_r.right() + 6.0, card_r.top() + 8.0));
                if self.right_inspector.show_draw_tooltip {
                    let tip_r = Rect::from_min_size(Pos2::new(rect.left() + 26.0, rect.top() - 26.0), Vec2::new(418.0, 32.0));
                    ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("ka_draw_tip"))).rect_filled(tip_r, 5.0, Color32::from_rgb(35, 34, 32));
                    ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("ka_draw_tip"))).text(
                        tip_r.center(),
                        egui::Align2::CENTER_CENTER,
                        "그리기 — 조사구역·유구 면과 선을 그립니다. 다시 누르면 도구를 닫고 이동합니다",
                        egui::FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                }
            } else {
                let top_txt = if is_measure {
                    "     줄자  — 두 점을 클릭하면 거리가, 셋 이상이면 면적이 나옵니다"
                } else if is_trench {
                    "⊞  시굴 격자  — 격자를 놓을 원점을 클릭합니다"
                } else {
                    "투명도·밝기: 조절할 레이어를 고르세요"
                };
                let top_w = if is_measure { 404.0 } else if is_trench { 290.0 } else { 224.0 };
                let top_r = Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::new(top_w, 30.0));
                p.rect_filled(top_r, 6.0, Color32::WHITE);
                p.rect_stroke(top_r, 6.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
                if is_measure {
                    let rc = Pos2::new(top_r.left() + 18.0, top_r.center().y);
                    p.line_segment([Pos2::new(rc.x - 5.0, rc.y + 4.5), Pos2::new(rc.x + 5.0, rc.y - 4.5)], Stroke::new(2.2, Color32::from_rgb(35, 35, 32)));
                    p.line_segment([Pos2::new(rc.x - 2.0, rc.y + 1.5), Pos2::new(rc.x - 0.5, rc.y + 3.0)], Stroke::new(1.2, Color32::from_rgb(35, 35, 32)));
                    p.line_segment([Pos2::new(rc.x + 1.0, rc.y - 1.5), Pos2::new(rc.x + 2.5, rc.y)], Stroke::new(1.2, Color32::from_rgb(35, 35, 32)));
                }
                p.text(Pos2::new(top_r.left() + 12.0, top_r.center().y), egui::Align2::LEFT_CENTER, top_txt, egui::FontId::proportional(11.5), Color32::from_rgb(45, 44, 40));
                Self::paint_undo_redo_pill(&p, Pos2::new(top_r.right() + 6.0, top_r.top()));

                if is_trench || is_measure {
                    let card_r = Rect::from_min_size(rect.min + Vec2::new(12.0, 52.0), Vec2::new(236.0, 78.0));
                    Self::paint_opacity_card(&p, card_r, "지적도 · 조사 주변 5km");
                }
            }

            if is_measure {
                let mr = Rect::from_min_size(rect.min + Vec2::new(260.0, 52.0), Vec2::new(280.0, 178.0));
                p.rect_filled(mr, 8.0, Color32::from_rgba_unmultiplied(248, 246, 240, 246));
                p.rect_stroke(mr, 8.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
                let t1 = Rect::from_min_size(mr.min + Vec2::new(12.0, 12.0), Vec2::new(56.0, 28.0));
                let t2 = Rect::from_min_size(mr.min + Vec2::new(72.0, 12.0), Vec2::new(56.0, 28.0));
                p.rect_filled(t1, 3.0, Color32::from_rgb(214, 208, 194));
                p.rect_filled(t2, 3.0, Color32::WHITE);
                p.text(t1.center(), egui::Align2::CENTER_CENTER, "거리", egui::FontId::proportional(12.5), Color32::BLACK);
                p.text(t2.center(), egui::Align2::CENTER_CENTER, "면적", egui::FontId::proportional(12.5), Color32::from_rgb(80, 79, 74));
                p.text(Pos2::new(mr.left() + 12.0, mr.top() + 58.0), egui::Align2::LEFT_CENTER, "0.000 m", egui::FontId::proportional(19.0), Color32::BLACK);
                p.text(Pos2::new(mr.left() + 12.0, mr.top() + 86.0), egui::Align2::LEFT_CENTER, "두 점 이상 찍고 우클릭 → 마침 · 작업좌표\nEPSG:5187 평면", egui::FontId::proportional(11.5), Color32::from_rgb(55, 54, 50));
                for (bi, b_txt) in ["마침", "복사", "한 점 취소", "다시"].into_iter().enumerate() {
                    let br = Rect::from_min_size(Pos2::new(mr.left() + 12.0 + (bi as f32) * 64.0, mr.top() + 110.0), Vec2::new(60.0, 28.0));
                    p.rect_filled(br, 5.0, Color32::WHITE);
                    p.rect_stroke(br, 5.0, Stroke::new(1.0, Color32::from_rgb(175, 172, 162)), egui::StrokeKind::Middle);
                    p.text(br.center(), egui::Align2::CENTER_CENTER, b_txt, egui::FontId::proportional(11.5), Color32::BLACK);
                }
                p.text(Pos2::new(mr.left() + 12.0, mr.top() + 154.0), egui::Align2::LEFT_CENTER, "클릭 점 · 우클릭에서 마침 · Backspace 한 점 취소 · Esc\n지우기", egui::FontId::proportional(10.5), Color32::from_rgb(110, 108, 102));
            }

            let zoom_r = Rect::from_min_size(Pos2::new(rect.right() - 40.0, rect.top() + 12.0), Vec2::new(28.0, 84.0));
            p.rect_filled(zoom_r, 5.0, Color32::WHITE);
            p.rect_stroke(zoom_r, 5.0, Stroke::new(1.0, Color32::from_rgb(190, 188, 178)), egui::StrokeKind::Middle);
            for (i, sym) in ["+", "−", "□"].into_iter().enumerate() {
                p.text(Pos2::new(zoom_r.center().x, zoom_r.top() + 14.0 + (i as f32) * 28.0), egui::Align2::CENTER_CENTER, sym, egui::FontId::proportional(14.0), Color32::from_rgb(45, 44, 40));
            }

            let bl = Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0);
            p.text(bl, egui::Align2::LEFT_CENTER, "▲ 도북", egui::FontId::proportional(11.0), Color32::from_rgb(45, 44, 40));
            let sb = Rect::from_min_size(Pos2::new(bl.x + 48.0, bl.y - 4.0), Vec2::new(90.0, 6.0));
            p.rect_filled(Rect::from_min_size(sb.min, Vec2::new(45.0, 6.0)), 0.0, Color32::from_rgb(35, 35, 32));
            p.rect_stroke(sb, 0.0, Stroke::new(1.0, Color32::from_rgb(35, 35, 32)), egui::StrokeKind::Middle);
            p.text(Pos2::new(sb.left(), sb.top() - 7.0), egui::Align2::LEFT_CENTER, "0          100         200 m", egui::FontId::proportional(10.0), Color32::from_rgb(45, 44, 40));

            let gc_r = Rect::from_min_size(Pos2::new(rect.right() - 86.0, rect.bottom() - 34.0), Vec2::new(74.0, 24.0));
            p.rect_filled(gc_r, 5.0, Color32::WHITE);
            p.rect_stroke(gc_r, 5.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
            p.text(gc_r.center(), egui::Align2::CENTER_CENTER, "□ 좌표격자", egui::FontId::proportional(11.0), Color32::from_rgb(55, 54, 50));
        });
    }

    fn paint_undo_redo_pill(p: &egui::Painter, min: Pos2) {
        let ur_r = Rect::from_min_size(min, Vec2::new(56.0, 30.0));
        p.rect_filled(ur_r, 5.0, Color32::WHITE);
        p.rect_stroke(ur_r, 5.0, Stroke::new(1.0, Color32::from_rgb(195, 192, 182)), egui::StrokeKind::Middle);
        p.line_segment([Pos2::new(ur_r.center().x, ur_r.top() + 1.0), Pos2::new(ur_r.center().x, ur_r.bottom() - 1.0)], Stroke::new(1.0, Color32::from_rgb(212, 209, 200)));
        p.text(Pos2::new(ur_r.left() + 14.0, ur_r.center().y), egui::Align2::CENTER_CENTER, "↺", egui::FontId::proportional(12.5), Color32::from_rgb(90, 88, 82));
        p.text(Pos2::new(ur_r.right() - 14.0, ur_r.center().y), egui::Align2::CENTER_CENTER, "↻", egui::FontId::proportional(12.5), Color32::from_rgb(175, 172, 162));
    }

    fn paint_opacity_card(p: &egui::Painter, card_r: Rect, title: &str) {
        p.rect_filled(card_r, 6.0, Color32::from_rgba_unmultiplied(252, 251, 247, 248));
        p.rect_stroke(card_r, 6.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
        p.text(Pos2::new(card_r.left() + 12.0, card_r.top() + 16.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(12.0), Color32::from_rgb(30, 30, 28));
        for (row_i, (lbl, pct_txt, knob_t, col)) in [("투명도", "100%", 1.0_f32, Color32::from_rgb(181, 87, 58)), ("밝기", "그림만", 0.5_f32, Color32::from_rgb(125, 124, 118))].into_iter().enumerate() {
            let y = card_r.top() + 38.0 + (row_i as f32) * 22.0;
            p.text(Pos2::new(card_r.left() + 12.0, y), egui::Align2::LEFT_CENTER, lbl, egui::FontId::proportional(11.5), Color32::from_rgb(95, 94, 88));
            let (sx0, sx1) = (card_r.left() + 56.0, card_r.right() - 48.0);
            let kx = sx0 + (sx1 - sx0) * knob_t;
            p.line_segment([Pos2::new(sx0, y), Pos2::new(kx, y)], Stroke::new(3.0, col));
            p.line_segment([Pos2::new(kx, y), Pos2::new(sx1, y)], Stroke::new(3.0, Color32::from_rgb(205, 202, 194)));
            p.circle_filled(Pos2::new(kx, y), 5.5, Color32::WHITE);
            p.circle_stroke(Pos2::new(kx, y), 5.5, Stroke::new(1.8, col));
            p.text(Pos2::new(card_r.right() - 10.0, y), egui::Align2::RIGHT_CENTER, pct_txt, egui::FontId::proportional(11.0), Color32::from_rgb(75, 74, 70));
        }
    }

    pub(crate) fn paint_gyeongju_heritage_canvas(&self, p: &egui::Painter, rect: Rect, cadastral_detail: bool) {
        let c = rect.center();
        let map_box = Rect::from_center_size(c, Vec2::new(rect.width() * 0.68, rect.height() * 0.72));
        p.rect_filled(map_box, 0.0, Color32::from_rgb(154, 198, 120));
        if cadastral_detail {
            let cad_s = Stroke::new(0.8, Color32::from_rgb(45, 44, 40));
            for gx in 0..22 {
                let x = rect.left() + 40.0 + (gx as f32) * ((rect.width() - 80.0) / 22.0);
                p.line_segment([Pos2::new(x, rect.top() + 10.0), Pos2::new(x + 14.0, rect.bottom() - 10.0)], cad_s);
            }
            for gy in 0..16 {
                let y = rect.top() + 10.0 + (gy as f32) * ((rect.height() - 20.0) / 16.0);
                p.line_segment([Pos2::new(rect.left() + 40.0, y), Pos2::new(rect.right() - 40.0, y - 8.0)], cad_s);
            }
        }
        let green_s = Stroke::new(1.3, Color32::from_rgb(34, 148, 52));
        let mag_s = Stroke::new(1.2, Color32::from_rgb(176, 28, 128));
        for (dx, dy, w, h, is_green) in [
            (-240.0, -140.0, 150.0, 120.0, true),
            (-110.0, -90.0, 180.0, 160.0, false),
            (95.0, -120.0, 140.0, 130.0, false),
            (220.0, 40.0, 190.0, 170.0, true),
            (-210.0, 110.0, 170.0, 140.0, false),
            (-40.0, 150.0, 160.0, 110.0, true),
            (30.0, 30.0, 110.0, 95.0, false),
        ] {
            let r = Rect::from_center_size(Pos2::new(c.x + dx, c.y + dy), Vec2::new(w, h));
            p.rect_stroke(r, 12.0, if is_green { green_s } else { mag_s }, egui::StrokeKind::Middle);
        }
        let sc = self.right_inspector.active_stroke_color();
        let fill_col = if self.right_inspector.no_fill {
            Color32::TRANSPARENT
        } else {
            Color32::from_rgba_unmultiplied(sc.r(), sc.g(), sc.b(), 85)
        };
        let stroke = if self.right_inspector.no_stroke {
            Stroke::NONE
        } else {
            Stroke::new(self.right_inspector.stroke_width_mm * 1.6, sc)
        };
        let survey_pts = vec![
            Pos2::new(c.x + 8.0, c.y - 165.0),
            Pos2::new(c.x + 32.0, c.y - 158.0),
            Pos2::new(c.x + 24.0, c.y + 155.0),
            Pos2::new(c.x + 2.0, c.y + 162.0),
        ];
        p.add(egui::Shape::convex_polygon(survey_pts, fill_col, stroke));
        for ti in 0..18 {
            let ty = c.y - 150.0 + (ti as f32) * 16.5;
            p.line_segment([Pos2::new(c.x + 7.0, ty), Pos2::new(c.x + 25.0, ty)], Stroke::new(2.0, Color32::from_rgb(156, 62, 32)));
            if cadastral_detail && ti % 4 == 1 {
                let tr_lbl = format!("Tr-{}\n(38.2 m²)", ti * 18 + 19);
                p.text(Pos2::new(c.x + 16.0, ty), egui::Align2::CENTER_CENTER, tr_lbl, egui::FontId::proportional(8.5), Color32::WHITE);
            }
        }
        let lbl_r = Rect::from_center_size(Pos2::new(c.x + 16.0, c.y + 4.0), Vec2::new(62.0, 16.0));
        p.rect_filled(lbl_r, 3.0, Color32::from_rgba_unmultiplied(255, 255, 255, 220));
        p.text(lbl_r.center(), egui::Align2::CENTER_CENTER, "86,865.77 m²", egui::FontId::proportional(10.0), Color32::BLACK);
        let (rx, ry) = (155.0, 240.0);
        let ring_pts: Vec<Pos2> = (0..=48).map(|i| { let a = (i as f32 / 48.0) * std::f32::consts::TAU; Pos2::new(c.x + 16.0 + rx * a.cos(), c.y + ry * a.sin()) }).collect();
        p.add(egui::Shape::line(ring_pts, Stroke::new(2.8, Color32::from_rgb(184, 58, 24))));
        for bp in [Pos2::new(c.x + 16.0, c.y - ry), Pos2::new(c.x + 16.0, c.y + ry), Pos2::new(c.x + 16.0 - rx, c.y), Pos2::new(c.x + 16.0 + rx, c.y)] {
            let b500 = Rect::from_center_size(bp, Vec2::new(36.0, 15.0));
            p.rect_filled(b500, 3.0, Color32::WHITE);
            p.text(b500.center(), egui::Align2::CENTER_CENTER, "500m", egui::FontId::proportional(9.5), Color32::from_rgb(184, 58, 24));
        }
    }
}
