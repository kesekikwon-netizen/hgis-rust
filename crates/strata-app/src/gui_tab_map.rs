use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::real_survey::ring_area_m2;
use crate::right_inspector::InspectorCardTab;

fn w2s(rect: Rect, vp: &ViewportTransform, xy: [f64; 2]) -> Pos2 {
    let [sx, sy] = vp.world_to_screen(xy);
    Pos2::new(rect.left() + sx as f32, rect.top() + sy as f32)
}

fn s2w(rect: Rect, vp: &ViewportTransform, pos: Pos2) -> [f64; 2] {
    vp.screen_to_world([
        f64::from(pos.x - rect.left()),
        f64::from(pos.y - rect.top()),
    ])
}

impl StrataAppState {
    pub(crate) fn render_map_tab_ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        self.render_cpp_left_layer_panel(ui, false);
        self.render_cpp_right_inspector(ui);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let is_trench = self.modals.active == Some(ActiveModalDialog::TrenchGenerator) || self.active_tool == MapToolKind::TrenchPlace;
            let is_measure = self.active_tool == MapToolKind::Measure;
            let is_style = self.right_inspector.active_card == InspectorCardTab::LayerStyleCard;
            if is_trench {
                let trench_cnt = self.home_map.real_survey.trenches.len();
                let trench_area: f64 = self.home_map.real_survey.trenches.iter().map(|t| t.area_m2).sum();
                let survey_area: f64 = self.home_map.real_survey.survey_area_m2.max(1.0);
                let ratio_pct = (trench_area / survey_area * 100.0).clamp(0.0, 100.0);
                let shape_lbl = if self.modals.trench_shape.arm_bend().is_some() {
                    self.modals.trench_shape.label_ko()
                } else {
                    self.modals.trench_length.label_ko()
                };
                egui::Frame::NONE.fill(Color32::from_rgb(225, 240, 218)).stroke(Stroke::new(1.0, Color32::from_rgb(145, 195, 138))).inner_margin(egui::Margin::symmetric(10, 4)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let _ = ui.small_button("II");
                        ui.colored_label(Color32::from_rgb(22, 138, 42), "✔");
                        ui.label(egui::RichText::new(format!("시굴격자: {shape_lbl} 시굴격자 {trench_cnt}개 · 총 {trench_area:.2}m² · 조사구역의 {ratio_pct:.1}% (시굴 기준 10%)")).size(12.0).strong());
                    });
                });
            }

            let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            self.home_map.viewport.screen_size_px = [f64::from(rect.width()), f64::from(rect.height())];
            if let Some(hp) = response.hover_pos() {
                self.home_map.cursor_world_xy = s2w(rect, &self.home_map.viewport, hp);
                if matches!(self.active_tool, MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint) {
                    self.home_map.update_cursor_snap_preview();
                } else {
                    self.home_map.cad.preview_markers_world.clear();
                }
            }
            if response.hovered() {
                let sy = ui.input(|i| i.smooth_scroll_delta.y);
                let anchor = [f64::from(rect.width()) * 0.5, f64::from(rect.height()) * 0.5];
                if sy.abs() > 0.1 {
                    self.home_map.viewport.zoom_at(if sy > 0.0 { 0.85 } else { 1.18 }, anchor);
                }
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.home_map.viewport.pan_by_pixels(f64::from(d.x), f64::from(d.y));
            }
            if response.clicked() {
                if let Some(cp) = response.interact_pointer_pos() {
                    let wxy = s2w(rect, &self.home_map.viewport, cp);
                    match self.active_tool {
                        MapToolKind::DrawPolygon
                        | MapToolKind::DrawLine
                        | MapToolKind::PlacePoint
                        | MapToolKind::Measure => {
                            let tool = self.active_tool;
                            self.home_map.on_canvas_left_click(wxy, tool);
                        }
                        MapToolKind::TrenchPlace => {
                            let len_m = self.modals.trench_length.meters();
                            let ring_5 = strata_geo::trench_grid::create_trench_rectangle_polygon(
                                wxy,
                                2.0,
                                len_m,
                                self.modals.trench_azimuth_deg,
                            );
                            let next_no = self.home_map.real_survey.trenches.len() + 1;
                            let name = format!("Tr-{next_no}");
                            let area_m2 = 2.0 * len_m;
                            self.home_map
                                .real_survey
                                .trenches
                                .push(crate::real_survey::RealTrenchPoly {
                                    name: name.clone(),
                                    ring: ring_5.0,
                                    area_m2,
                                    center: wxy,
                                });
                            self.is_dirty = true;
                            self.home_map.draw_guide_message = Some(format!(
                                "시굴격자 {name} ({area_m2:.1} m²)을(를) 배치했습니다 — 화살표 키(Shift: 1 m)로 미세 이동할 수 있습니다."
                            ));
                        }
                        MapToolKind::Select => {
                            let mpp = self.home_map.viewport.meters_per_pixel;
                            if let Some((idx, tr)) = self
                                .home_map
                                .real_survey
                                .trenches
                                .iter()
                                .enumerate()
                                .find(|(_, t)| {
                                    let min_x = t.ring.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
                                    let max_x = t.ring.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
                                    let min_y = t.ring.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
                                    let max_y = t.ring.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
                                    strata_canvas2d::canvas_painter::perform_canvas_hit_test_10px(
                                        Some(([min_x, min_y], [max_x, max_y], 1)),
                                        None,
                                        wxy,
                                        mpp,
                                        10.0,
                                    )
                                    .has_hit
                                })
                            {
                                let tr_name = tr.name.clone();
                                let tr_area = tr.area_m2;
                                self.right_inspector.selected_layer_key = Some("시굴격자".to_owned());
                                self.home_map.draw_guide_message = Some(format!(
                                    "도형 선택: 시굴격자 #{idx} ({tr_name}, {tr_area:.1} m²) — 화살표 키(0.1 m) 또는 Shift+화살표(1.0 m)로 이동"
                                ));
                            } else {
                                self.home_map.draw_guide_message = Some(format!(
                                    "클릭 좌표 (X {:.2}, Y {:.2}) 주변 10px 이내에 선택된 도형이 없습니다.",
                                    wxy[0], wxy[1]
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if response.secondary_clicked() {
                let tool = self.active_tool;
                let ring = self.home_map.scene.active_rubberband.clone();
                if self.home_map.cad.on_right_click(tool, &mut self.home_map.scene) && ring.len() >= 3 {
                    let area_m2 = ring_area_m2(&ring);
                    self.home_map.real_survey.save_drawn_polygon(ring);
                    self.is_dirty = true;
                    self.home_map.draw_guide_message = Some(format!("조사구역·유구 다각형을 저장했습니다 ({area_m2:.2} m²)."));
                }
            }

            let p = ui.painter_at(rect);
            p.rect_filled(rect, 0.0, Color32::from_rgb(251, 250, 246));
            self.paint_gyeongju_heritage_canvas(&p, rect, is_trench || is_measure || is_style);

            if is_style {
                let card_r = Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::new(236.0, 78.0));
                let layer_lbl = self.right_inspector.selected_layer_key.as_deref().unwrap_or("조사구역").to_owned();
                self.paint_opacity_card(ui, &p, card_r, &layer_lbl);
                self.paint_undo_redo_pill(ui, &p, Pos2::new(card_r.right() + 6.0, card_r.top() + 8.0));
                if self.right_inspector.show_draw_tooltip {
                    let tip_r = Rect::from_min_size(Pos2::new(rect.left() + 26.0, rect.top() - 26.0), Vec2::new(418.0, 32.0));
                    let tp = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("ka_draw_tip")));
                    tp.rect_filled(tip_r, 5.0, Color32::from_rgb(35, 34, 32));
                    tp.text(tip_r.center(), egui::Align2::CENTER_CENTER, "그리기 — 조사구역·유구 면과 선을 그립니다. 다시 누르면 도구를 닫고 이동합니다", egui::FontId::proportional(11.5), Color32::WHITE);
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
                }
                p.text(Pos2::new(top_r.left() + 12.0, top_r.center().y), egui::Align2::LEFT_CENTER, top_txt, egui::FontId::proportional(11.5), Color32::from_rgb(45, 44, 40));
                self.paint_undo_redo_pill(ui, &p, Pos2::new(top_r.right() + 6.0, top_r.top()));
                if is_trench || is_measure {
                    let card_r = Rect::from_min_size(rect.min + Vec2::new(12.0, 52.0), Vec2::new(236.0, 78.0));
                    self.paint_opacity_card(ui, &p, card_r, "지적도 · 조사 주변 5km");
                }
            }

            if is_measure {
                let mr = Rect::from_min_size(rect.min + Vec2::new(260.0, 52.0), Vec2::new(280.0, 178.0));
                p.rect_filled(mr, 8.0, Color32::from_rgba_unmultiplied(248, 246, 240, 246));
                p.rect_stroke(mr, 8.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
                let t1 = Rect::from_min_size(mr.min + Vec2::new(12.0, 12.0), Vec2::new(56.0, 28.0));
                let t2 = Rect::from_min_size(mr.min + Vec2::new(72.0, 12.0), Vec2::new(56.0, 28.0));
                let area_mode = self.home_map.real_survey.measure_area_tab;
                p.rect_filled(t1, 3.0, if !area_mode { Color32::from_rgb(214, 208, 194) } else { Color32::WHITE });
                p.rect_filled(t2, 3.0, if area_mode { Color32::from_rgb(214, 208, 194) } else { Color32::WHITE });
                p.text(t1.center(), egui::Align2::CENTER_CENTER, "거리", egui::FontId::proportional(12.5), Color32::BLACK);
                p.text(t2.center(), egui::Align2::CENTER_CENTER, "면적", egui::FontId::proportional(12.5), Color32::from_rgb(80, 79, 74));
                if ui.interact(t1, ui.id().with("m_tab_dist"), egui::Sense::click()).clicked() { self.home_map.real_survey.measure_area_tab = false; }
                if ui.interact(t2, ui.id().with("m_tab_area"), egui::Sense::click()).clicked() { self.home_map.real_survey.measure_area_tab = true; }

                let readout = if let Some(m) = &self.home_map.cad.last_measure {
                    if area_mode { format!("{:.2} m²", m.polygon_area_m2) } else { format!("{:.3} m", m.total_distance_m) }
                } else {
                    "0.000 m".to_owned()
                };
                p.text(Pos2::new(mr.left() + 12.0, mr.top() + 58.0), egui::Align2::LEFT_CENTER, &readout, egui::FontId::proportional(19.0), Color32::BLACK);
                p.text(Pos2::new(mr.left() + 12.0, mr.top() + 86.0), egui::Align2::LEFT_CENTER, "두 점 이상 찍고 우클릭 → 마침 · 작업좌표\nEPSG:5187 평면", egui::FontId::proportional(11.5), Color32::from_rgb(55, 54, 50));
                for (bi, b_txt) in ["마침", "복사", "한 점 취소", "다시"].into_iter().enumerate() {
                    let br = Rect::from_min_size(Pos2::new(mr.left() + 12.0 + (bi as f32) * 64.0, mr.top() + 110.0), Vec2::new(60.0, 28.0));
                    p.rect_filled(br, 5.0, Color32::WHITE);
                    p.rect_stroke(br, 5.0, Stroke::new(1.0, Color32::from_rgb(175, 172, 162)), egui::StrokeKind::Middle);
                    p.text(br.center(), egui::Align2::CENTER_CENTER, b_txt, egui::FontId::proportional(11.5), Color32::BLACK);
                    if ui.interact(br, ui.id().with(("m_btn", bi)), egui::Sense::click()).clicked() {
                        match bi {
                            0 => { let _ = self.home_map.cad.on_right_click(MapToolKind::Measure, &mut self.home_map.scene); }
                            1 => ui.ctx().copy_text(readout.clone()),
                            2 => self.home_map.cad.on_backspace(&mut self.home_map.scene),
                            _ => self.home_map.cad.on_escape(&mut self.home_map.scene),
                        }
                    }
                }
                p.text(Pos2::new(mr.left() + 12.0, mr.top() + 154.0), egui::Align2::LEFT_CENTER, "클릭 점 · 우클릭에서 마침 · Backspace 한 점 취소 · Esc\n지우기", egui::FontId::proportional(10.5), Color32::from_rgb(110, 108, 102));
            }

            let zoom_r = Rect::from_min_size(Pos2::new(rect.right() - 40.0, rect.top() + 12.0), Vec2::new(28.0, 84.0));
            p.rect_filled(zoom_r, 5.0, Color32::WHITE);
            p.rect_stroke(zoom_r, 5.0, Stroke::new(1.0, Color32::from_rgb(190, 188, 178)), egui::StrokeKind::Middle);
            for (i, sym) in ["+", "−", "□"].into_iter().enumerate() {
                let zr = Rect::from_min_size(Pos2::new(zoom_r.left(), zoom_r.top() + (i as f32) * 28.0), Vec2::new(28.0, 28.0));
                p.text(zr.center(), egui::Align2::CENTER_CENTER, sym, egui::FontId::proportional(14.0), Color32::from_rgb(45, 44, 40));
                if ui.interact(zr, ui.id().with(("map_zoom_btn", i)), egui::Sense::click()).clicked() {
                    let center_px = [f64::from(rect.width()) * 0.5, f64::from(rect.height()) * 0.5];
                    match i {
                        0 => self.home_map.viewport.zoom_at(0.8, center_px),
                        1 => self.home_map.viewport.zoom_at(1.25, center_px),
                        _ => self.home_map.fit_to_survey(),
                    }
                }
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
            let gc_lbl = if self.home_map.grid_enabled { "☑ 좌표격자" } else { "□ 좌표격자" };
            p.text(gc_r.center(), egui::Align2::CENTER_CENTER, gc_lbl, egui::FontId::proportional(11.0), Color32::from_rgb(55, 54, 50));
            if ui.interact(gc_r, ui.id().with("map_grid_toggle"), egui::Sense::click()).clicked() {
                self.home_map.grid_enabled = !self.home_map.grid_enabled;
            }
        });
    }

    fn paint_undo_redo_pill(&mut self, ui: &mut egui::Ui, p: &egui::Painter, min: Pos2) {
        let ur_r = Rect::from_min_size(min, Vec2::new(56.0, 30.0));
        p.rect_filled(ur_r, 5.0, Color32::WHITE);
        p.rect_stroke(
            ur_r,
            5.0,
            Stroke::new(1.0, Color32::from_rgb(195, 192, 182)),
            egui::StrokeKind::Middle,
        );
        p.line_segment(
            [
                Pos2::new(ur_r.center().x, ur_r.top() + 1.0),
                Pos2::new(ur_r.center().x, ur_r.bottom() - 1.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
        );
        p.text(
            Pos2::new(ur_r.left() + 14.0, ur_r.center().y),
            egui::Align2::CENTER_CENTER,
            "↺",
            egui::FontId::proportional(12.5),
            Color32::from_rgb(90, 88, 82),
        );
        p.text(
            Pos2::new(ur_r.right() - 14.0, ur_r.center().y),
            egui::Align2::CENTER_CENTER,
            "↻",
            egui::FontId::proportional(12.5),
            Color32::from_rgb(90, 88, 82),
        );
        let undo_r = Rect::from_min_max(ur_r.min, Pos2::new(ur_r.center().x, ur_r.max.y));
        let redo_r = Rect::from_min_max(Pos2::new(ur_r.center().x, ur_r.min.y), ur_r.max);
        if ui
            .interact(
                undo_r,
                ui.id().with("ka_canvas_undo_btn"),
                egui::Sense::click(),
            )
            .on_hover_text("실행취소 (Ctrl+Z)")
            .clicked()
        {
            self.home_map.cad.on_backspace(&mut self.home_map.scene);
            self.home_map.draw_guide_message =
                Some("마지막 편집을 되돌렸습니다 (Ctrl+Z).".to_owned());
        }
        if ui
            .interact(
                redo_r,
                ui.id().with("ka_canvas_redo_btn"),
                egui::Sense::click(),
            )
            .on_hover_text("다시실행 (Ctrl+Y)")
            .clicked()
        {
            self.home_map.draw_guide_message =
                Some("되돌린 편집을 다시 적용했습니다 (Ctrl+Y).".to_owned());
        }
    }

    fn paint_opacity_card(
        &mut self,
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
            egui::FontId::proportional(12.0),
            Color32::from_rgb(30, 30, 28),
        );
        let opacity_val = self.home_map.real_survey.layer_opacity.clamp(0.0, 1.0);
        let bright_val = self.home_map.real_survey.layer_brightness.clamp(0.0, 1.0);
        let opacity_txt = format!("{:.0}%", opacity_val * 100.0);
        let bright_txt = if (bright_val - 0.5).abs() < 0.03 {
            "그림만".to_owned()
        } else {
            format!("{:+.0}%", (bright_val - 0.5) * 200.0)
        };
        for (row_i, (lbl, pct_txt, knob_t, col)) in [
            (
                "투명도",
                opacity_txt,
                opacity_val,
                Color32::from_rgb(181, 87, 58),
            ),
            (
                "밝기",
                bright_txt,
                bright_val,
                Color32::from_rgb(125, 124, 118),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let y = card_r.top() + 38.0 + (row_i as f32) * 22.0;
            p.text(
                Pos2::new(card_r.left() + 12.0, y),
                egui::Align2::LEFT_CENTER,
                lbl,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(95, 94, 88),
            );
            let (sx0, sx1) = (card_r.left() + 56.0, card_r.right() - 48.0);
            let kx = sx0 + (sx1 - sx0) * knob_t;
            p.line_segment([Pos2::new(sx0, y), Pos2::new(kx, y)], Stroke::new(3.0, col));
            p.line_segment(
                [Pos2::new(kx, y), Pos2::new(sx1, y)],
                Stroke::new(3.0, Color32::from_rgb(205, 202, 194)),
            );
            p.circle_filled(Pos2::new(kx, y), 5.5, Color32::WHITE);
            p.circle_stroke(Pos2::new(kx, y), 5.5, Stroke::new(1.8, col));
            p.text(
                Pos2::new(card_r.right() - 10.0, y),
                egui::Align2::RIGHT_CENTER,
                &pct_txt,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(75, 74, 70),
            );

            let track_r =
                Rect::from_min_max(Pos2::new(sx0 - 6.0, y - 9.0), Pos2::new(sx1 + 6.0, y + 9.0));
            let resp = ui.interact(
                track_r,
                ui.id().with(("ka_opacity_slider", row_i)),
                egui::Sense::click_and_drag(),
            );
            if resp.clicked() || resp.dragged() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let t = ((pos.x - sx0) / (sx1 - sx0).max(1.0)).clamp(0.0, 1.0);
                    if row_i == 0 {
                        self.home_map.real_survey.layer_opacity = t;
                    } else {
                        self.home_map.real_survey.layer_brightness = t;
                    }
                }
            }
        }
    }

    pub(crate) fn paint_gyeongju_heritage_canvas(
        &self,
        p: &egui::Painter,
        rect: Rect,
        cadastral_detail: bool,
    ) {
        let vp = &self.home_map.viewport;
        let rs = &self.home_map.real_survey;
        let [cx, cy] = rs.survey_center;

        if rs.is_layer_visible("지형 음영") || rs.is_layer_visible("지표모델(DSM) 30m · Coperni...")
        {
            let tl = w2s(rect, vp, [cx - 1050.0, cy + 720.0]);
            let br = w2s(rect, vp, [cx + 1050.0, cy - 720.0]);
            p.rect_filled(
                Rect::from_min_max(tl, br),
                0.0,
                Color32::from_rgb(154, 198, 120),
            );
        }

        if rs.is_layer_visible("지적도 · 조사 주변 5km") {
            let cad_col = if cadastral_detail {
                Color32::from_rgb(38, 37, 34)
            } else {
                Color32::from_rgba_unmultiplied(60, 58, 52, 110)
            };
            let cad_s = Stroke::new(if cadastral_detail { 0.85 } else { 0.55 }, cad_col);
            for parcel in &rs.cadastral_parcels {
                let pts: Vec<Pos2> = parcel.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(pts, cad_s));
            }
        }

        if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
            let green_s = Stroke::new(1.3, Color32::from_rgb(34, 148, 52));
            let mag_s = Stroke::new(1.2, Color32::from_rgb(176, 28, 128));
            for h in &rs.heritage_polys {
                if !rs.is_layer_visible(&h.layer_name) {
                    continue;
                }
                let pts: Vec<Pos2> = h.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(
                    pts,
                    if h.is_green { green_s } else { mag_s },
                ));
            }
        }

        if rs.is_layer_visible("조사구역") {
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
            for ring in &rs.survey_rings {
                let pts: Vec<Pos2> = ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::convex_polygon(
                    pts.clone(),
                    fill_col,
                    Stroke::NONE,
                ));
                p.add(egui::Shape::line(pts, stroke));
            }
            if rs.is_label_visible("조사구역") {
                let sp = w2s(rect, vp, rs.survey_center);
                let lbl_r = Rect::from_center_size(sp, Vec2::new(66.0, 16.0));
                p.rect_filled(
                    lbl_r,
                    3.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 220),
                );
                p.text(
                    lbl_r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.2} m²", rs.survey_area_m2),
                    egui::FontId::proportional(10.0),
                    Color32::BLACK,
                );
            }
        }

        if rs.is_layer_visible("시굴격자") {
            let tr_s = Stroke::new(1.2, Color32::from_rgb(156, 62, 32));
            for (ti, tr) in rs.trenches.iter().enumerate() {
                let pts: Vec<Pos2> = tr.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(pts, tr_s));
                if cadastral_detail && rs.is_label_visible("시굴격자") && ti % 14 == 3 {
                    let tp = w2s(rect, vp, tr.center);
                    p.text(
                        tp,
                        egui::Align2::CENTER_CENTER,
                        format!("{}\n({:.1} m²)", tr.name, tr.area_m2),
                        egui::FontId::proportional(8.5),
                        Color32::WHITE,
                    );
                }
            }
        }

        if rs.is_layer_visible("주변 500m") && !rs.buffer_500m.is_empty() {
            let ring_pts: Vec<Pos2> = rs.buffer_500m.iter().map(|&pt| w2s(rect, vp, pt)).collect();
            p.add(egui::Shape::line(
                ring_pts,
                Stroke::new(2.6, Color32::from_rgb(184, 58, 24)),
            ));
        }
        if rs.show_buffer_1000m && !rs.buffer_1000m.is_empty() {
            let ring_pts: Vec<Pos2> = rs
                .buffer_1000m
                .iter()
                .map(|&pt| w2s(rect, vp, pt))
                .collect();
            p.add(egui::Shape::line(
                ring_pts,
                Stroke::new(2.2, Color32::from_rgb(148, 42, 18)),
            ));
        }
        if rs.is_layer_visible("주변 500m 거리") && rs.is_label_visible("주변 500m 거리") {
            for (lbl, pt) in &rs.dist_labels_500m {
                let bp = w2s(rect, vp, *pt);
                let b500 = Rect::from_center_size(bp, Vec2::new(36.0, 15.0));
                p.rect_filled(b500, 3.0, Color32::WHITE);
                p.text(
                    b500.center(),
                    egui::Align2::CENTER_CENTER,
                    lbl,
                    egui::FontId::proportional(9.5),
                    Color32::from_rgb(184, 58, 24),
                );
            }
        }

        if self.home_map.cad.snap_enabled && self.home_map.cad.snap_preview_enabled {
            for &pt in &self.home_map.cad.preview_markers_world {
                let sp = w2s(rect, vp, pt);
                p.circle_filled(sp, 4.5, Color32::from_rgba_unmultiplied(255, 251, 235, 210));
                p.circle_stroke(
                    sp,
                    4.5,
                    Stroke::new(2.0, Color32::from_rgba_unmultiplied(217, 119, 6, 230)),
                );
            }
        }

        if !self.home_map.scene.active_rubberband.is_empty() {
            let mut active_pts: Vec<Pos2> = self
                .home_map
                .scene
                .active_rubberband
                .iter()
                .map(|&pt| w2s(rect, vp, pt))
                .collect();
            for &v in &active_pts {
                p.circle_filled(v, 4.0, Color32::from_rgb(214, 78, 22));
            }
            active_pts.push(w2s(rect, vp, self.home_map.cursor_world_xy));
            p.add(egui::Shape::line(
                active_pts,
                Stroke::new(2.2, Color32::from_rgb(214, 78, 22)),
            ));
        }
    }
}
