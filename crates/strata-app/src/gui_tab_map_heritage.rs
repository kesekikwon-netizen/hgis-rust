//! 경주시 유적·지적·조사 레이어를 지도 캔버스에 그린다 — 원문 `paint_gyeongju_heritage_canvas`.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_map_canvas_overlay::HeritageOverlayCtx;
use super::gui_tab_map_format::modulate_color_with_opacity_brightness;
use super::gui_tab_map_geom::w2s;
impl StrataAppState {
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

        // 「지형 음영」·「지표모델(DSM)」은 **실제 DEM 래스터**가 있을 때만 그린다.
        //
        // 예전에는 데이터가 없어도 `survey_center` 기준 2100×1440m 사각형에
        // 합성 등고선·색 띠를 칠했다. 그래서 새 조사 화면 한가운데에 초록
        // 사각형이 떴다(사용자 지적 2026-11-09). C++ `LayerOps::addElevationHillshadeMap`
        // 은 실제 QgsRasterLayer(GeoTIFF)만 쓰고, 없으면 아무 것도 그리지 않는다.
        // 여기서도 같게 한다 — 자료를 받기 전에는 지형 음영이 없다.
        let has_real_dem = self.home_map.dem_grid.is_some();
        if has_real_dem
            && (rs.is_layer_visible("지형 음영")
                || rs.is_layer_visible("지표모델(DSM) 30m · Coperni..."))
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

        // 조사 파일의 유구 면·선·단면선·유물·기준점 (조사구역 위, 시굴격자 아래).
        self.paint_survey_doc_layers(p, rect);

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
        self.paint_heritage_overlays(HeritageOverlayCtx {
            ctx,
            p,
            rect,
            vp,
            rs,
            center: [cx, cy],
            layer_color: &layer_color,
        });
    }
}
