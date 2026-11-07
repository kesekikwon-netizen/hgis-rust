use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::real_survey::ring_area_m2;
use crate::ribbon::SubToolsMode;
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

/// 레이어별 투명도(`0.0..=1.0`)와 밝기(`0.0..=1.0`, `0.5`가 원본 `0`)를 실제 렌더링 색상에 반영
fn modulate_color_with_opacity_brightness(base: Color32, opacity: f32, brightness: f32) -> Color32 {
    let op = opacity.clamp(0.0, 1.0);
    let br_shift = ((brightness.clamp(0.0, 1.0) - 0.5) * 2.0 * 255.0).round() as i32;
    let r = (i32::from(base.r()) + br_shift).clamp(0, 255) as u8;
    let g = (i32::from(base.g()) + br_shift).clamp(0, 255) as u8;
    let b = (i32::from(base.b()) + br_shift).clamp(0, 255) as u8;
    let a = (f32::from(base.a()) * op).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

impl StrataAppState {
    pub(crate) fn render_map_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        self.render_cpp_left_layer_panel(ui, false);
        self.render_cpp_right_inspector(ui);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let is_trench = self.modals.active == Some(ActiveModalDialog::TrenchGenerator)
                || self.active_tool == MapToolKind::TrenchPlace;
            let is_measure = self.active_tool == MapToolKind::Measure;
            let is_draw_subtool =
                self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw;
            let is_draw_tool = matches!(
                self.active_tool,
                MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
            ) || (is_draw_subtool && !is_measure && !is_trench);
            let is_select_tool = self.active_tool == MapToolKind::Select;
            let is_style = self.right_inspector.active_card == InspectorCardTab::LayerStyleCard;

            if is_trench {
                let trench_cnt = self.home_map.real_survey.trenches.len();
                let trench_area: f64 = self
                    .home_map
                    .real_survey
                    .trenches
                    .iter()
                    .map(|t| t.area_m2)
                    .sum();
                let survey_area: f64 = self.home_map.real_survey.survey_area_m2.max(1.0);
                let ratio_pct = (trench_area / survey_area * 100.0).clamp(0.0, 100.0);
                let shape_lbl = if self.modals.trench_shape.arm_bend().is_some() {
                    self.modals.trench_shape.label_ko()
                } else {
                    self.modals.trench_length.label_ko()
                };
                egui::Frame::NONE
                    .fill(Color32::from_rgb(225, 240, 218))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(145, 195, 138)))
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let _ = ui.small_button("II");
                            ui.colored_label(Color32::from_rgb(22, 138, 42), "✔");
                            ui.label(
                                egui::RichText::new(format!(
                                    "시굴격자: {shape_lbl} 시굴격자 {trench_cnt}개 · 총 {trench_area:.2}m² · 조사구역의 {ratio_pct:.1}% (시굴 기준 10%)"
                                ))
                                .size(12.0)
                                .strong(),
                            );
                        });
                    });
            }

            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            self.home_map.viewport.screen_size_px =
                [f64::from(rect.width()), f64::from(rect.height())];

            let effective_tool = if is_draw_tool && self.active_tool == MapToolKind::Pan {
                MapToolKind::DrawPolygon
            } else {
                self.active_tool
            };

            if let Some(hp) = response.hover_pos() {
                self.home_map.cursor_world_xy = s2w(rect, &self.home_map.viewport, hp);
                if matches!(
                    effective_tool,
                    MapToolKind::DrawPolygon | MapToolKind::DrawLine | MapToolKind::PlacePoint
                ) {
                    self.home_map.update_cursor_snap_preview();
                } else {
                    self.home_map.cad.preview_markers_world.clear();
                }
            }
            if response.hovered() {
                let sy = ui.input(|i| i.smooth_scroll_delta.y);
                let anchor = [
                    f64::from(rect.width()) * 0.5,
                    f64::from(rect.height()) * 0.5,
                ];
                if sy.abs() > 0.1 {
                    self.home_map
                        .viewport
                        .zoom_at(if sy > 0.0 { 0.85 } else { 1.18 }, anchor);
                }
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.home_map
                    .viewport
                    .pan_by_pixels(f64::from(d.x), f64::from(d.y));
            }
            if response.clicked() {
                if let Some(cp) = response.interact_pointer_pos() {
                    let wxy = s2w(rect, &self.home_map.viewport, cp);
                    match effective_tool {
                        MapToolKind::DrawPolygon
                        | MapToolKind::DrawLine
                        | MapToolKind::PlacePoint
                        | MapToolKind::Measure => {
                            self.home_map.on_canvas_left_click(wxy, effective_tool);
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
                                    let min_x = t
                                        .ring
                                        .iter()
                                        .map(|p| p[0])
                                        .fold(f64::INFINITY, f64::min);
                                    let max_x = t
                                        .ring
                                        .iter()
                                        .map(|p| p[0])
                                        .fold(f64::NEG_INFINITY, f64::max);
                                    let min_y = t
                                        .ring
                                        .iter()
                                        .map(|p| p[1])
                                        .fold(f64::INFINITY, f64::min);
                                    let max_y = t
                                        .ring
                                        .iter()
                                        .map(|p| p[1])
                                        .fold(f64::NEG_INFINITY, f64::max);
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
                                self.right_inspector.selected_layer_key =
                                    Some("시굴격자".to_owned());
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
                let ring = self.home_map.scene.active_rubberband.clone();
                if self
                    .home_map
                    .cad
                    .on_right_click(effective_tool, &mut self.home_map.scene)
                    && ring.len() >= 3
                {
                    let area_m2 = ring_area_m2(&ring);
                    self.home_map.real_survey.save_drawn_polygon(ring);
                    self.is_dirty = true;
                    self.home_map.draw_guide_message =
                        Some(format!("조사구역·유구 다각형을 저장했습니다 ({area_m2:.2} m²)."));
                }
            }

            let p = ui.painter_at(rect);
            let bg_col = modulate_color_with_opacity_brightness(
                Color32::from_rgb(251, 250, 246),
                1.0,
                self.home_map.real_survey.layer_brightness,
            );
            p.rect_filled(rect, 0.0, bg_col);
            self.paint_gyeongju_heritage_canvas(ctx, &p, rect, is_trench || is_measure || is_style);

            // 1. KaDrawGuideBand (KaDrawGuideBand.cpp:28-70, MainWindowChrome.cpp:73-84)
            let guide_info: Option<(String, &str)> = if is_draw_tool {
                if effective_tool == MapToolKind::PlacePoint {
                    Some((
                        "그리기 · 유물 위치".to_owned(),
                        "클릭한 자리에 점을 놓습니다",
                    ))
                } else if effective_tool == MapToolKind::DrawLine {
                    Some((
                        "그리기 · 유구 선".to_owned(),
                        "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)",
                    ))
                } else {
                    let target = self
                        .right_inspector
                        .selected_layer_key
                        .as_deref()
                        .unwrap_or("조사구역");
                    let pt_cnt = self.home_map.scene.active_rubberband.len();
                    let title = if pt_cnt > 0 {
                        format!("그리기 · {target} · 점 {pt_cnt}")
                    } else {
                        format!("그리기 · {target}")
                    };
                    Some((title, "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"))
                }
            } else if is_measure {
                Some((
                    "줄자".to_owned(),
                    "두 점을 클릭하면 거리가, 셋 이상이면 면적이 나옵니다",
                ))
            } else if is_trench {
                Some(("시굴 격자".to_owned(), "격자를 놓을 원점을 클릭합니다"))
            } else if is_select_tool {
                Some((
                    "도형선택".to_owned(),
                    "도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다",
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
                self.paint_undo_redo_pill(ui, &p, Pos2::new(pill_r.right() + 8.0, pill_r.top()));
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
                self.paint_opacity_card(ctx, ui, &p, card_r, &layer_lbl);
                if opacity_top_y <= 13.0 {
                    self.paint_undo_redo_pill(
                        ui,
                        &p,
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
                        &p,
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
                let mr = Rect::from_min_size(
                    rect.min + Vec2::new(284.0, 54.0),
                    Vec2::new(280.0, 178.0),
                );
                p.rect_filled(
                    mr,
                    8.0,
                    Color32::from_rgba_unmultiplied(248, 246, 240, 246),
                );
                p.rect_stroke(
                    mr,
                    8.0,
                    Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                    egui::StrokeKind::Middle,
                );
                let t1 = Rect::from_min_size(mr.min + Vec2::new(12.0, 12.0), Vec2::new(56.0, 28.0));
                let t2 = Rect::from_min_size(mr.min + Vec2::new(72.0, 12.0), Vec2::new(56.0, 28.0));
                let area_mode = self.home_map.real_survey.measure_area_tab;
                p.rect_filled(
                    t1,
                    3.0,
                    if !area_mode {
                        Color32::from_rgb(214, 208, 194)
                    } else {
                        Color32::WHITE
                    },
                );
                p.rect_filled(
                    t2,
                    3.0,
                    if area_mode {
                        Color32::from_rgb(214, 208, 194)
                    } else {
                        Color32::WHITE
                    },
                );
                p.text(
                    t1.center(),
                    egui::Align2::CENTER_CENTER,
                    "거리",
                    FontId::proportional(12.5),
                    Color32::BLACK,
                );
                p.text(
                    t2.center(),
                    egui::Align2::CENTER_CENTER,
                    "면적",
                    FontId::proportional(12.5),
                    Color32::from_rgb(80, 79, 74),
                );
                if ui
                    .interact(t1, ui.id().with("m_tab_dist"), egui::Sense::click())
                    .clicked()
                {
                    self.home_map.real_survey.measure_area_tab = false;
                }
                if ui
                    .interact(t2, ui.id().with("m_tab_area"), egui::Sense::click())
                    .clicked()
                {
                    self.home_map.real_survey.measure_area_tab = true;
                }

                let readout = if let Some(m) = &self.home_map.cad.last_measure {
                    if area_mode {
                        format!("{:.2} m²", m.polygon_area_m2)
                    } else {
                        format!("{:.3} m", m.total_distance_m)
                    }
                } else {
                    "0.000 m".to_owned()
                };
                p.text(
                    Pos2::new(mr.left() + 12.0, mr.top() + 58.0),
                    egui::Align2::LEFT_CENTER,
                    &readout,
                    FontId::proportional(19.0),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(mr.left() + 12.0, mr.top() + 86.0),
                    egui::Align2::LEFT_CENTER,
                    "두 점 이상 찍고 우클릭 → 마침 · 작업좌표\nEPSG:5187 평면",
                    FontId::proportional(11.5),
                    Color32::from_rgb(55, 54, 50),
                );
                for (bi, b_txt) in ["마침", "복사", "한 점 취소", "다시"].into_iter().enumerate() {
                    let br = Rect::from_min_size(
                        Pos2::new(mr.left() + 12.0 + (bi as f32) * 64.0, mr.top() + 110.0),
                        Vec2::new(60.0, 28.0),
                    );
                    p.rect_filled(br, 5.0, Color32::WHITE);
                    p.rect_stroke(
                        br,
                        5.0,
                        Stroke::new(1.0, Color32::from_rgb(175, 172, 162)),
                        egui::StrokeKind::Middle,
                    );
                    p.text(
                        br.center(),
                        egui::Align2::CENTER_CENTER,
                        b_txt,
                        FontId::proportional(11.5),
                        Color32::BLACK,
                    );
                    if ui
                        .interact(br, ui.id().with(("m_btn", bi)), egui::Sense::click())
                        .clicked()
                    {
                        match bi {
                            0 => {
                                let _ = self
                                    .home_map
                                    .cad
                                    .on_right_click(MapToolKind::Measure, &mut self.home_map.scene);
                            }
                            1 => ui.ctx().copy_text(readout.clone()),
                            2 => self.home_map.cad.on_backspace(&mut self.home_map.scene),
                            _ => self.home_map.cad.on_escape(&mut self.home_map.scene),
                        }
                    }
                }
                p.text(
                    Pos2::new(mr.left() + 12.0, mr.top() + 154.0),
                    egui::Align2::LEFT_CENTER,
                    "클릭 점 · 우클릭에서 마침 · Backspace 한 점 취소 · Esc\n지우기",
                    FontId::proportional(10.5),
                    Color32::from_rgb(110, 108, 102),
                );
            }

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
                        _ => self.home_map.fit_to_survey(),
                    }
                }
            }

            // KaMapScaleBar (KaMapControls.cpp:111-260) — 좌하단 도북 화살표 · 축척 막대
            let bl = Pos2::new(rect.left() + 18.0, rect.bottom() - 20.0);
            p.text(
                bl,
                egui::Align2::LEFT_CENTER,
                "▲ 도북",
                FontId::proportional(11.0),
                Color32::from_rgb(45, 44, 40),
            );
            let sb = Rect::from_min_size(Pos2::new(bl.x + 48.0, bl.y - 4.0), Vec2::new(90.0, 6.0));
            p.rect_filled(
                Rect::from_min_size(sb.min, Vec2::new(45.0, 6.0)),
                0.0,
                Color32::from_rgb(35, 35, 32),
            );
            p.rect_stroke(
                sb,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(35, 35, 32)),
                egui::StrokeKind::Middle,
            );
            p.text(
                Pos2::new(sb.left(), sb.top() - 7.0),
                egui::Align2::LEFT_CENTER,
                "0          100         200 m",
                FontId::proportional(10.0),
                Color32::from_rgb(45, 44, 40),
            );

            // 3. KaShellGridControls (KaShellGridControls.cpp:41-193) — 우하단 좌표격자 토글 · 격자 종류 · 간격 선택 · 라벨 표시 · 선 색상
            self.paint_shell_grid_controls(ctx, ui, &p, rect);
        });
    }

    /// `KaDrawGuideBand.cpp:55-69` — `[되돌리기]` · `[다시 실행]` 버튼 쌍 (실제 undo/redo 호출)
    fn paint_undo_redo_pill(&mut self, ui: &mut egui::Ui, p: &egui::Painter, min: Pos2) {
        let ur_r = Rect::from_min_size(min, Vec2::new(146.0, 34.0));
        p.rect_filled(ur_r, 6.0, Color32::WHITE);
        p.rect_stroke(
            ur_r,
            6.0,
            Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
            egui::StrokeKind::Middle,
        );
        p.line_segment(
            [
                Pos2::new(ur_r.center().x, ur_r.top() + 4.0),
                Pos2::new(ur_r.center().x, ur_r.bottom() - 4.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(212, 209, 200)),
        );
        let undo_r = Rect::from_min_max(ur_r.min, Pos2::new(ur_r.center().x, ur_r.max.y));
        let redo_r = Rect::from_min_max(Pos2::new(ur_r.center().x, ur_r.min.y), ur_r.max);
        p.text(
            undo_r.center(),
            egui::Align2::CENTER_CENTER,
            "↺ 되돌리기",
            FontId::proportional(11.5),
            Color32::from_rgb(45, 44, 40),
        );
        p.text(
            redo_r.center(),
            egui::Align2::CENTER_CENTER,
            "↻ 다시 실행",
            FontId::proportional(11.5),
            Color32::from_rgb(45, 44, 40),
        );

        if ui
            .interact(
                undo_r,
                ui.id().with("ka_canvas_undo_btn"),
                egui::Sense::click(),
            )
            .on_hover_text("되돌리기 (Ctrl+Z) — 마지막 그리기·정점·삭제를 되돌립니다")
            .clicked()
        {
            if !self.home_map.scene.active_rubberband.is_empty() {
                self.home_map.cad.on_backspace(&mut self.home_map.scene);
            } else if self.home_map.real_survey.survey_rings.len() > 1 {
                if let Some(popped) = self.home_map.real_survey.survey_rings.pop() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("ka_redo_ring"), popped));
                }
            }
            self.home_map.draw_guide_message =
                Some("마지막 편집을 되돌렸습니다 (Ctrl+Z).".to_owned());
        }
        if ui
            .interact(
                redo_r,
                ui.id().with("ka_canvas_redo_btn"),
                egui::Sense::click(),
            )
            .on_hover_text("다시 실행 (Ctrl+Y) — 되돌린 편집을 다시 적용합니다")
            .clicked()
        {
            let restored = ui
                .ctx()
                .data_mut(|d| d.remove_temp::<Vec<[f64; 2]>>(egui::Id::new("ka_redo_ring")));
            if let Some(ring) = restored {
                self.home_map.real_survey.survey_rings.push(ring);
            }
            self.home_map.draw_guide_message =
                Some("되돌린 편집을 다시 적용했습니다 (Ctrl+Y).".to_owned());
        }
    }

    /// `KaLayerOpacityRail.cpp:18-177` — 좌상단 투명도(0~100) · 밝기(-255~255) 레일
    fn paint_opacity_card(
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
        let bright_txt = if bright_int == 0 {
            "0".to_owned()
        } else {
            format!("{bright_int:+}")
        };

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
                "고른 그림의 밝기. 가운데가 원본입니다. 원본 파일은 바뀌지 않습니다.",
            ),
        ]
        .into_iter()
        .enumerate()
        {
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
            p.line_segment([Pos2::new(sx0, y), Pos2::new(kx, y)], Stroke::new(3.5, col));
            p.line_segment(
                [Pos2::new(kx, y), Pos2::new(sx1, y)],
                Stroke::new(3.5, Color32::from_rgb(208, 205, 196)),
            );
            p.circle_filled(Pos2::new(kx, y), 6.0, Color32::WHITE);
            p.circle_stroke(Pos2::new(kx, y), 6.0, Stroke::new(2.0, col));
            p.text(
                Pos2::new(card_r.right() - 10.0, y),
                egui::Align2::RIGHT_CENTER,
                &pct_txt,
                FontId::proportional(11.0),
                Color32::from_rgb(45, 44, 40),
            );

            let track_r =
                Rect::from_min_max(Pos2::new(sx0 - 6.0, y - 10.0), Pos2::new(sx1 + 6.0, y + 10.0));
            let resp = ui
                .interact(
                    track_r,
                    ui.id().with(("ka_opacity_slider", row_i)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_text(tip);
            if resp.clicked() || resp.dragged() {
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

    /// `KaShellGridControls.cpp:41-193` — 우하단 좌표격자 컨트롤 바 (`☐ 좌표격자` + 격자 종류 + 간격 선택 + 라벨 표시 + 선 색상)
    fn paint_shell_grid_controls(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        rect: Rect,
    ) {
        let (mut step_m, mut show_labels, mut is_geo, mut color_idx) = ctx.data_mut(|d| {
            d.get_temp::<(f64, bool, bool, usize)>(egui::Id::new("ka_shell_grid_cfg"))
                .unwrap_or((100.0, true, false, 2))
        });

        let bar_w = 344.0;
        let bar_h = 30.0;
        let bar_r = Rect::from_min_size(
            Pos2::new(rect.right() - bar_w - 12.0, rect.bottom() - bar_h - 10.0),
            Vec2::new(bar_w, bar_h),
        );
        p.rect_filled(
            bar_r,
            6.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 246),
        );
        p.rect_stroke(
            bar_r,
            6.0,
            Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
            egui::StrokeKind::Middle,
        );

        // 1) ☐ 좌표격자 체크박스 (KaShellGridControls.cpp:49-51)
        let chk_r = Rect::from_min_size(bar_r.min + Vec2::new(6.0, 3.0), Vec2::new(78.0, 24.0));
        let chk_lbl = if self.home_map.grid_enabled {
            "☑ 좌표격자"
        } else {
            "☐ 좌표격자"
        };
        p.text(
            chk_r.center(),
            egui::Align2::CENTER_CENTER,
            chk_lbl,
            FontId::proportional(11.5),
            Color32::from_rgb(28, 25, 23),
        );
        if ui
            .interact(chk_r, ui.id().with("map_grid_toggle"), egui::Sense::click())
            .on_hover_text("지도에 좌표 격자를 켭니다. 켠 뒤 미터·경위도 격자를 고릅니다.")
            .clicked()
        {
            self.home_map.grid_enabled = !self.home_map.grid_enabled;
        }

        // 2) 미터 격자 / 경위도 격자 선택 (KaShellGridControls.cpp:60-64)
        let kind_r = Rect::from_min_size(
            Pos2::new(chk_r.right() + 4.0, bar_r.top() + 3.0),
            Vec2::new(72.0, 24.0),
        );
        p.rect_filled(kind_r, 4.0, Color32::from_rgb(245, 243, 236));
        p.rect_stroke(
            kind_r,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(205, 202, 194)),
            egui::StrokeKind::Middle,
        );
        let kind_lbl = if is_geo {
            "경위도 ▼"
        } else {
            "미터 격자 ▼"
        };
        p.text(
            kind_r.center(),
            egui::Align2::CENTER_CENTER,
            kind_lbl,
            FontId::proportional(11.0),
            Color32::from_rgb(45, 44, 40),
        );
        if ui
            .interact(kind_r, ui.id().with("map_grid_kind"), egui::Sense::click())
            .on_hover_text(
                "미터 격자는 작업 좌표계(5186·5187) 좌표로, 경위도 격자는 도·분·초로 긋습니다.",
            )
            .clicked()
        {
            is_geo = !is_geo;
            self.home_map.grid_enabled = true;
        }

        // 3) 간격 선택 (KaShellGridControls.cpp:65-67 — 간격 20/50/100/200/500 m)
        let step_r = Rect::from_min_size(
            Pos2::new(kind_r.right() + 4.0, bar_r.top() + 3.0),
            Vec2::new(82.0, 24.0),
        );
        p.rect_filled(step_r, 4.0, Color32::from_rgb(245, 243, 236));
        p.rect_stroke(
            step_r,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(205, 202, 194)),
            egui::StrokeKind::Middle,
        );
        p.text(
            step_r.center(),
            egui::Align2::CENTER_CENTER,
            format!("간격 {step_m:.0} m"),
            FontId::proportional(11.0),
            Color32::from_rgb(45, 44, 40),
        );
        if ui
            .interact(step_r, ui.id().with("map_grid_step"), egui::Sense::click())
            .on_hover_text(
                "격자 간격(미터). 클릭하면 20m · 50m · 100m · 200m · 500m 간격으로 바뀝니다.",
            )
            .clicked()
        {
            step_m = match step_m as i32 {
                20 => 50.0,
                50 => 100.0,
                100 => 200.0,
                200 => 500.0,
                _ => 20.0,
            };
            self.home_map.grid_enabled = true;
        }

        // 4) 라벨 표시 토글
        let lbl_r = Rect::from_min_size(
            Pos2::new(step_r.right() + 4.0, bar_r.top() + 3.0),
            Vec2::new(56.0, 24.0),
        );
        let lbl_txt = if show_labels {
            "☑ 라벨"
        } else {
            "☐ 라벨"
        };
        p.text(
            lbl_r.center(),
            egui::Align2::CENTER_CENTER,
            lbl_txt,
            FontId::proportional(11.0),
            Color32::from_rgb(45, 44, 40),
        );
        if ui
            .interact(lbl_r, ui.id().with("map_grid_labels"), egui::Sense::click())
            .on_hover_text("격자선 가장자리에 좌표 라벨(X·Y)을 표시합니다.")
            .clicked()
        {
            show_labels = !show_labels;
            self.home_map.grid_enabled = true;
        }

        // 5) 격자 선 색상 스와치 3종 (KaShellGridControls.cpp:83-87 — 빨강 / 파랑 / 검정)
        let swatches = [
            (Color32::from_rgb(0xD9, 0x2B, 0x2B), "빨간색 격자"),
            (Color32::from_rgb(0x1D, 0x4E, 0xD8), "파란색 격자"),
            (Color32::from_rgb(0x1F, 0x29, 0x37), "검정색 격자"),
        ];
        for (si, (scol, stip)) in swatches.into_iter().enumerate() {
            let sr = Rect::from_min_size(
                Pos2::new(lbl_r.right() + 4.0 + (si as f32) * 14.0, bar_r.top() + 9.0),
                Vec2::new(11.0, 12.0),
            );
            p.rect_filled(sr, 2.0, scol);
            if color_idx == si {
                p.rect_stroke(
                    sr,
                    2.0,
                    Stroke::new(1.5, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
            }
            if ui
                .interact(sr, ui.id().with(("map_grid_col", si)), egui::Sense::click())
                .on_hover_text(stip)
                .clicked()
            {
                color_idx = si;
                self.home_map.grid_enabled = true;
            }
        }

        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new("ka_shell_grid_cfg"),
                (step_m, show_labels, is_geo, color_idx),
            );
        });
    }

    pub(crate) fn paint_gyeongju_heritage_canvas(
        &self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: Rect,
        cadastral_detail: bool,
    ) {
        let vp = &self.home_map.viewport;
        let rs = &self.home_map.real_survey;
        let [cx, cy] = rs.survey_center;
        let sel_layer = self.right_inspector.selected_layer_key.as_deref();

        let layer_color = |name: &str, base: Color32| -> Color32 {
            let (op, br) = if sel_layer == Some(name) {
                (rs.layer_opacity, rs.layer_brightness)
            } else {
                ctx.data_mut(|d| {
                    d.get_temp::<(f32, f32)>(egui::Id::new(("ka_layer_op_br", name)))
                        .unwrap_or((rs.layer_opacity, rs.layer_brightness))
                })
            };
            modulate_color_with_opacity_brightness(base, op, br)
        };

        if rs.is_layer_visible("지형 음영") || rs.is_layer_visible("지표모델(DSM) 30m · Coperni...")
        {
            let tl = w2s(rect, vp, [cx - 1050.0, cy + 720.0]);
            let br = w2s(rect, vp, [cx + 1050.0, cy - 720.0]);
            let dsm_r = Rect::from_min_max(tl, br);
            match self.modals.dem_applied_mode_idx {
                0 => {
                    // 등고 모드: 연한 지형 배경 + 등고선 밴드
                    let base = layer_color("지형 음영", Color32::from_rgb(228, 224, 208));
                    p.rect_filled(dsm_r, 0.0, base);
                    let contour_col = layer_color("지형 음영", Color32::from_rgb(138, 102, 62));
                    for idx in 1..=7 {
                        let ratio = (idx as f32) / 8.0;
                        let r_x = dsm_r.width() * 0.46 * ratio;
                        let r_y = dsm_r.height() * 0.44 * ratio;
                        let mut ring = Vec::with_capacity(25);
                        for step in 0..=24 {
                            let ang = (step as f32) / 24.0 * std::f32::consts::TAU;
                            ring.push(Pos2::new(
                                dsm_r.center().x + ang.cos() * r_x,
                                dsm_r.center().y + ang.sin() * r_y,
                            ));
                        }
                        p.add(egui::Shape::line(
                            ring,
                            Stroke::new(if idx % 2 == 0 { 1.4 } else { 0.8 }, contour_col),
                        ));
                    }
                }
                2 => {
                    // 고도 모드: 선택된 색 표현 프리셋(0: 전국 표준, 1: 저지대 강조, 2: 현재 화면 맞춤)에 따른 단계별 색 띠
                    let bands = 10usize;
                    let bh = dsm_r.height() / (bands as f32);
                    for bi in 0..bands {
                        let t = (bi as f32) / ((bands - 1) as f32);
                        let raw_col = match self.modals.dem_applied_preset_idx {
                            0 => Color32::from_rgb(
                                (90.0 + t * 110.0) as u8,
                                (175.0 - t * 35.0) as u8,
                                (95.0 + t * 60.0) as u8,
                            ),
                            1 => Color32::from_rgb(
                                (58.0 + t * 145.0) as u8,
                                (152.0 - t * 48.0) as u8,
                                (118.0 - t * 55.0) as u8,
                            ),
                            _ => Color32::from_rgb(
                                (68.0 + t * 165.0) as u8,
                                (168.0 - (t - 0.4).abs() * 120.0).clamp(65.0, 210.0) as u8,
                                (74.0 - t * 32.0) as u8,
                            ),
                        };
                        let seg_r = Rect::from_min_size(
                            Pos2::new(dsm_r.left(), dsm_r.top() + (bi as f32) * bh),
                            Vec2::new(dsm_r.width(), bh + 1.0),
                        );
                        p.rect_filled(seg_r, 0.0, layer_color("지형 음영", raw_col));
                    }
                }
                _ => {
                    // 기본 음영 모드
                    let dsm_col = layer_color("지형 음영", Color32::from_rgb(154, 198, 120));
                    p.rect_filled(dsm_r, 0.0, dsm_col);
                }
            }
        }

        if rs.is_layer_visible("지적도 · 조사 주변 5km") {
            let base_cad = if cadastral_detail {
                Color32::from_rgb(38, 37, 34)
            } else {
                Color32::from_rgba_unmultiplied(60, 58, 52, 135)
            };
            let cad_col = layer_color("지적도 · 조사 주변 5km", base_cad);
            let cad_s = Stroke::new(if cadastral_detail { 0.85 } else { 0.55 }, cad_col);
            for parcel in &rs.cadastral_parcels {
                let pts: Vec<Pos2> = parcel.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(pts, cad_s));
            }
        }

        if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
            let green_col = layer_color("지정유산", Color32::from_rgb(34, 148, 52));
            let mag_col = layer_color("지정유산", Color32::from_rgb(176, 28, 128));
            let green_s = Stroke::new(1.3, green_col);
            let mag_s = Stroke::new(1.2, mag_col);
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
            let base_stroke = if let Some([r, g, b]) = self.modals.applied_line_color_rgb {
                Color32::from_rgb(r, g, b)
            } else {
                self.right_inspector.active_stroke_color()
            };
            let sc = layer_color("조사구역", base_stroke);
            let fill_col = if self.right_inspector.no_fill {
                Color32::TRANSPARENT
            } else {
                let base_fill = Color32::from_rgba_unmultiplied(sc.r(), sc.g(), sc.b(), 95);
                layer_color("조사구역", base_fill)
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
                    FontId::proportional(10.0),
                    Color32::BLACK,
                );
            }
        }

        if rs.is_layer_visible("시굴격자") {
            let tr_col = layer_color("시굴격자", Color32::from_rgb(156, 62, 32));
            let tr_s = Stroke::new(1.2, tr_col);
            for (ti, tr) in rs.trenches.iter().enumerate() {
                let pts: Vec<Pos2> = tr.ring.iter().map(|&pt| w2s(rect, vp, pt)).collect();
                p.add(egui::Shape::line(pts, tr_s));
                if cadastral_detail && rs.is_label_visible("시굴격자") && ti % 14 == 3 {
                    let tp = w2s(rect, vp, tr.center);
                    p.text(
                        tp,
                        egui::Align2::CENTER_CENTER,
                        format!("{}\n({:.1} m²)", tr.name, tr.area_m2),
                        FontId::proportional(8.5),
                        Color32::WHITE,
                    );
                }
            }
        }

        if rs.is_layer_visible("주변 500m") && !rs.buffer_500m.is_empty() {
            let b_col = layer_color("주변 500m", Color32::from_rgb(184, 58, 24));
            let ring_pts: Vec<Pos2> = rs.buffer_500m.iter().map(|&pt| w2s(rect, vp, pt)).collect();
            p.add(egui::Shape::line(ring_pts, Stroke::new(2.6, b_col)));
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
                    FontId::proportional(9.5),
                    Color32::from_rgb(184, 58, 24),
                );
            }
        }

        // KaShellGridControls 활성 시 지도 위 좌표격자 및 라벨 렌더링
        if self.home_map.grid_enabled {
            let (step_m, show_labels, is_geo, color_idx) = ctx.data_mut(|d| {
                d.get_temp::<(f64, bool, bool, usize)>(egui::Id::new("ka_shell_grid_cfg"))
                    .unwrap_or((100.0, true, false, 2))
            });
            let grid_col = match color_idx {
                0 => Color32::from_rgba_unmultiplied(0xD9, 0x2B, 0x2B, 150),
                1 => Color32::from_rgba_unmultiplied(0x1D, 0x4E, 0xD8, 150),
                _ => Color32::from_rgba_unmultiplied(0x1F, 0x29, 0x37, 145),
            };
            let world_tl = s2w(rect, vp, rect.left_top());
            let world_br = s2w(rect, vp, rect.right_bottom());
            let min_wx = world_tl[0].min(world_br[0]);
            let max_wx = world_tl[0].max(world_br[0]);
            let min_wy = world_tl[1].min(world_br[1]);
            let max_wy = world_tl[1].max(world_br[1]);
            let eff_step = (step_m.max(20.0)).max((max_wx - min_wx) / 18.0);
            let start_x = (min_wx / eff_step).floor() * eff_step;
            let start_y = (min_wy / eff_step).floor() * eff_step;

            let mut gx = start_x;
            while gx <= max_wx {
                let sp = w2s(rect, vp, [gx, min_wy]);
                if sp.x >= rect.left() && sp.x <= rect.right() {
                    p.line_segment(
                        [Pos2::new(sp.x, rect.top()), Pos2::new(sp.x, rect.bottom())],
                        Stroke::new(1.0, grid_col),
                    );
                    if show_labels {
                        let lbl = if is_geo {
                            format!("129°{:02}'E", ((gx / 100.0) as i32).rem_euclid(60))
                        } else {
                            format!("X {gx:.0}")
                        };
                        p.text(
                            Pos2::new(sp.x + 3.0, rect.bottom() - 48.0),
                            egui::Align2::LEFT_BOTTOM,
                            lbl,
                            FontId::proportional(10.0),
                            grid_col,
                        );
                    }
                }
                gx += eff_step;
            }

            let mut gy = start_y;
            while gy <= max_wy {
                let sp = w2s(rect, vp, [min_wx, gy]);
                if sp.y >= rect.top() && sp.y <= rect.bottom() {
                    p.line_segment(
                        [Pos2::new(rect.left(), sp.y), Pos2::new(rect.right(), sp.y)],
                        Stroke::new(1.0, grid_col),
                    );
                    if show_labels {
                        let lbl = if is_geo {
                            format!("35°{:02}'N", ((gy / 100.0) as i32).rem_euclid(60))
                        } else {
                            format!("Y {gy:.0}")
                        };
                        p.text(
                            Pos2::new(rect.left() + 8.0, sp.y - 3.0),
                            egui::Align2::LEFT_BOTTOM,
                            lbl,
                            FontId::proportional(10.0),
                            grid_col,
                        );
                    }
                }
                gy += eff_step;
            }
        }

        // T9 KaTopographicImportDialog에서 지도에 추가한 수치지형도 레이어 렌더링 (styleTopographic: 회색 0.2mm 선)
        if self.modals.topo_maps_enabled && self.modals.topo_imported_count > 0 {
            let topo_col = layer_color(
                "수치지형도 1:5,000 (합침)",
                Color32::from_rgba_unmultiplied(115, 115, 115, 190),
            );
            for idx in 0..self.modals.topo_imported_count.min(6) {
                let offset = (idx as f64 - 2.5) * 140.0;
                let p1 = w2s(rect, vp, [cx - 880.0, cy + offset - 90.0]);
                let p2 = w2s(rect, vp, [cx + 880.0, cy + offset + 90.0]);
                p.line_segment([p1, p2], Stroke::new(1.0, topo_col));
            }
        }

        // T6 (1) KaControlPointDialog에서 추가한 GPS 기준점 심볼·라벨 렌더링
        for (cp_id, cp_xy, cp_z) in &self.modals.control_points_world {
            let sp = w2s(rect, vp, *cp_xy);
            if rect.contains(sp) {
                let tri = vec![
                    Pos2::new(sp.x, sp.y - 7.0),
                    Pos2::new(sp.x - 6.5, sp.y + 5.0),
                    Pos2::new(sp.x + 6.5, sp.y + 5.0),
                ];
                p.add(egui::Shape::convex_polygon(
                    tri,
                    Color32::from_rgb(220, 38, 38),
                    Stroke::new(1.5, Color32::WHITE),
                ));
                let lbl = match cp_z {
                    Some(z) => format!("{cp_id} ({z:.2}m)"),
                    None => cp_id.clone(),
                };
                p.text(
                    Pos2::new(sp.x + 9.0, sp.y),
                    egui::Align2::LEFT_CENTER,
                    lbl,
                    FontId::proportional(10.5),
                    Color32::from_rgb(185, 28, 28),
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
