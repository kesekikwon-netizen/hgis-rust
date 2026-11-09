//! 「고지형」 탭 중앙 하단 — 2D 가상 지층 단면도와 시추공 주상도.
//!
//! `gui_tab_paleo.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 중앙 패널 안 (B) 절을 그대로 옮겼고, 순서·수치·문구는 바꾸지 않았다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// (B) 하단: `KaPaleoCrossSectionWidget` 지층 단면도 (`KaPaleoCrossSectionWidget.cpp:27-97`)
    pub(crate) fn render_paleo_cross_section(&mut self, ui: &mut egui::Ui) {
            // (B) 하단: KaPaleoCrossSectionWidget 지층 단면도 + 시추공 주상도 (KaPaleoCrossSectionWidget.cpp:27-97)
            let sec_avail = ui.available_size();
            let (sec_rect, _) = ui.allocate_exact_size(
                Vec2::new(sec_avail.x, sec_avail.y.max(150.0)),
                egui::Sense::hover(),
            );
            let sp = ui.painter_at(sec_rect);
            sp.rect_filled(sec_rect, 4.0, Color32::from_rgb(250, 250, 249));
            sp.rect_stroke(
                sec_rect,
                4.0,
                Stroke::new(1.0, Color32::from_rgb(214, 211, 209)),
                egui::StrokeKind::Middle,
            );

            sp.text(
                Pos2::new(sec_rect.left() + 10.0, sec_rect.top() + 11.0),
                egui::Align2::LEFT_CENTER,
                "2D 가상 지층 단면도 (현 지표면 · 매립 제거 원지형 · 충적 문화층 · 생토면 · 하천 수면 · 시추공 주상도)",
                egui::FontId::proportional(11.5),
                Color32::from_rgb(28, 25, 23),
            );

            let plot = Rect::from_min_max(
                Pos2::new(sec_rect.left() + 48.0, sec_rect.top() + 24.0),
                Pos2::new(sec_rect.right() - 14.0, sec_rect.bottom() - 24.0),
            );
            sp.rect_filled(plot, 0.0, Color32::WHITE);
            sp.rect_stroke(
                plot,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(214, 211, 209)),
                egui::StrokeKind::Middle,
            );

            if let Some(ref out) = self.paleo_3d.last_paleo_output {
                let cs = &out.center_cross_section;
                let n = cs.dist_m.len();
                if n >= 2 && plot.width() > 40.0 && plot.height() > 30.0 {
                    let mut z_min = cs
                        .z_bedrock_base_m
                        .iter()
                        .copied()
                        .fold(f64::INFINITY, f64::min);
                    let mut z_max = cs
                        .z_surface_curr_m
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max);
                    for bh in &out.parsed_boreholes {
                        z_min = z_min.min(bh.weathered_top_elev_m - 3.5);
                        z_max = z_max.max(bh.ground_elev_m + 1.5);
                    }
                    z_min -= 1.5;
                    z_max = (z_max + 2.0).max(z_min + 8.0);
                    let max_dist = cs.dist_m.last().copied().unwrap_or(1.0).max(1.0);

                    let to_pt = |d: f64, z: f64| -> Pos2 {
                        let x = plot.left() + ((d / max_dist) as f32) * plot.width();
                        let y =
                            plot.bottom() - (((z - z_min) / (z_max - z_min)) as f32) * plot.height();
                        Pos2::new(
                            x.clamp(plot.left(), plot.right()),
                            y.clamp(plot.top(), plot.bottom()),
                        )
                    };

                    // 수평 표고 보조선 및 좌측 표고 눈금
                    for step in 0..=4 {
                        let z_val = z_min + (z_max - z_min) * (step as f64 / 4.0);
                        let py = to_pt(0.0, z_val).y;
                        sp.line_segment(
                            [Pos2::new(plot.left(), py), Pos2::new(plot.right(), py)],
                            Stroke::new(0.6, Color32::from_rgb(238, 236, 232)),
                        );
                        sp.text(
                            Pos2::new(plot.left() - 5.0, py),
                            egui::Align2::RIGHT_CENTER,
                            format!("{z_val:.0}m"),
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(87, 83, 78),
                        );
                    }

                    // 3대 지층 단면 영역 채움 (성토층 / 충적 문화층 / 기반암 풍화토)
                    for i in 0..(n - 1) {
                        let d0 = cs.dist_m[i];
                        let d1 = cs.dist_m[i + 1];
                        let p_s0 = to_pt(d0, cs.z_surface_curr_m[i]);
                        let p_s1 = to_pt(d1, cs.z_surface_curr_m[i + 1]);
                        let p_p0 = to_pt(d0, cs.z_paleo_top_m[i]);
                        let p_p1 = to_pt(d1, cs.z_paleo_top_m[i + 1]);
                        let p_b0 = to_pt(d0, cs.z_bedrock_base_m[i]);
                        let p_b1 = to_pt(d1, cs.z_bedrock_base_m[i + 1]);

                        // (1) 기반암·풍화대 영역
                        sp.add(egui::Shape::convex_polygon(
                            vec![
                                p_b0,
                                p_b1,
                                Pos2::new(p_b1.x, plot.bottom()),
                                Pos2::new(p_b0.x, plot.bottom()),
                            ],
                            Color32::from_rgba_unmultiplied(168, 160, 144, 95),
                            Stroke::NONE,
                        ));
                        // (2) 충적 문화층 영역 (KaPaleoCrossSectionWidget.cpp:73 QColor(251, 192, 45, 70))
                        sp.add(egui::Shape::convex_polygon(
                            vec![p_p0, p_p1, p_b1, p_b0],
                            Color32::from_rgba_unmultiplied(251, 192, 45, 95),
                            Stroke::NONE,
                        ));
                        // (3) 현대 매립·성토층 영역
                        sp.add(egui::Shape::convex_polygon(
                            vec![p_s0, p_s1, p_p1, p_p0],
                            Color32::from_rgba_unmultiplied(194, 158, 120, 105),
                            Stroke::NONE,
                        ));
                    }

                    // 4대 경계선 곡선 (KaPaleoCrossSectionWidget.cpp:84-87)
                    let river_z0 = 22.0_f64;
                    for i in 0..(n - 1) {
                        let d0 = cs.dist_m[i];
                        let d1 = cs.dist_m[i + 1];
                        // 하천 수면 (파란 파선)
                        if i % 2 == 0 {
                            let r0 = river_z0 + 2.4 * (d0 / max_dist);
                            let r1 = river_z0 + 2.4 * (d1 / max_dist);
                            sp.line_segment(
                                [to_pt(d0, r0), to_pt(d1, r1)],
                                Stroke::new(1.4, Color32::from_rgb(2, 132, 199)),
                            );
                        }
                        // 생토면(풍화토) 갈색 실선
                        sp.line_segment(
                            [
                                to_pt(d0, cs.z_bedrock_base_m[i]),
                                to_pt(d1, cs.z_bedrock_base_m[i + 1]),
                            ],
                            Stroke::new(1.8, Color32::from_rgb(120, 53, 15)),
                        );
                        // 복원 원지형(Z_paleo) 주황 실선
                        sp.line_segment(
                            [
                                to_pt(d0, cs.z_paleo_top_m[i]),
                                to_pt(d1, cs.z_paleo_top_m[i + 1]),
                            ],
                            Stroke::new(2.2, Color32::from_rgb(217, 119, 6)),
                        );
                        // 현 지표면 점선
                        if i % 2 == 0 {
                            sp.line_segment(
                                [
                                    to_pt(d0, cs.z_surface_curr_m[i]),
                                    to_pt(d1, cs.z_surface_curr_m[i + 1]),
                                ],
                                Stroke::new(1.5, Color32::from_rgb(87, 83, 78)),
                            );
                        }
                    }

                    // 시추공 주상도 기둥 (BH-01 ~ BH-08: 성토층·충적층·풍화토 심도별 주상도 + 지하수위)
                    let col_half_w = 6.5_f32;
                    for bh in &out.parsed_boreholes {
                        let dist_along = (bh.x - out.transform.origin_x).clamp(0.0, max_dist);
                        let z_top = bh.ground_elev_m;
                        let z_paleo = bh.paleo_surface_elev_m;
                        let z_weathered = bh.weathered_top_elev_m;
                        let z_bot = (z_weathered - 3.2).max(z_min + 0.3);

                        let p_top = to_pt(dist_along, z_top);
                        let p_pal = to_pt(dist_along, z_paleo);
                        let p_wea = to_pt(dist_along, z_weathered);
                        let p_bot = to_pt(dist_along, z_bot);

                        let r_fill = Rect::from_min_max(
                            Pos2::new(p_top.x - col_half_w, p_top.y),
                            Pos2::new(p_top.x + col_half_w, p_pal.y.max(p_top.y + 3.0)),
                        );
                        let r_alluv = Rect::from_min_max(
                            Pos2::new(p_top.x - col_half_w, r_fill.bottom()),
                            Pos2::new(p_top.x + col_half_w, p_wea.y.max(r_fill.bottom() + 5.0)),
                        );
                        let r_weath = Rect::from_min_max(
                            Pos2::new(p_top.x - col_half_w, r_alluv.bottom()),
                            Pos2::new(p_top.x + col_half_w, p_bot.y.max(r_alluv.bottom() + 4.0)),
                        );

                        let is_coarse = bh.sed_type.eq_ignore_ascii_case("COARSE");
                        let alluv_col = if is_coarse {
                            Color32::from_rgb(245, 158, 11)
                        } else {
                            Color32::from_rgb(16, 185, 129)
                        };

                        sp.rect_filled(r_fill, 0.0, Color32::from_rgb(194, 158, 120));
                        sp.rect_filled(r_alluv, 0.0, alluv_col);
                        sp.rect_filled(r_weath, 0.0, Color32::from_rgb(120, 53, 15));

                        let full_col = Rect::from_min_max(r_fill.min, r_weath.max);
                        sp.rect_stroke(
                            full_col,
                            0.0,
                            Stroke::new(1.0, Color32::from_rgb(28, 25, 23)),
                            egui::StrokeKind::Middle,
                        );
                        sp.line_segment(
                            [
                                Pos2::new(r_fill.left(), r_fill.bottom()),
                                Pos2::new(r_fill.right(), r_fill.bottom()),
                            ],
                            Stroke::new(0.8, Color32::from_rgb(28, 25, 23)),
                        );
                        sp.line_segment(
                            [
                                Pos2::new(r_alluv.left(), r_alluv.bottom()),
                                Pos2::new(r_alluv.right(), r_alluv.bottom()),
                            ],
                            Stroke::new(0.8, Color32::from_rgb(28, 25, 23)),
                        );

                        // 지하수위 마커 (▼)
                        let p_water = to_pt(dist_along, z_top - bh.water_depth_m);
                        sp.line_segment(
                            [
                                Pos2::new(p_top.x - col_half_w - 3.0, p_water.y),
                                Pos2::new(p_top.x + col_half_w + 3.0, p_water.y),
                            ],
                            Stroke::new(1.2, Color32::from_rgb(2, 132, 199)),
                        );

                        // 시추공 코드 및 충적층 심도 라벨
                        sp.text(
                            Pos2::new(p_top.x, (p_top.y - 8.0).max(plot.top() + 7.0)),
                            egui::Align2::CENTER_CENTER,
                            &bh.hole_id,
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(28, 25, 23),
                        );
                        sp.text(
                            Pos2::new(p_top.x + col_half_w + 3.0, r_alluv.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{}:{:.1}m", bh.dominant_uscs, bh.alluvial_thickness_m),
                            egui::FontId::proportional(8.5),
                            Color32::from_rgb(45, 43, 40),
                        );
                    }
                }
            }

            // 하단 단면도 범례 문구 (KaPaleoCrossSectionWidget.cpp:96)
            sp.text(
                Pos2::new(plot.left(), sec_rect.bottom() - 11.0),
                egui::Align2::LEFT_CENTER,
                "점선: 현 지표면 | 주황 실선: 복원 원지형(Z_paleo) | 갈색 실선: 생토면(풍화토) | 파란 파선: 하천 수면 | 주상도 기둥: 시추공(성토·충적·풍화토)",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(87, 83, 78),
            );
    }
}
