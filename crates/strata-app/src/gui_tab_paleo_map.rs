//! 「고지형」 탭 중앙 상단 — 7대 고지형 분류도 실물 렌더(시추 마커·AOI·단면선).
//!
//! `gui_tab_paleo.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 중앙 패널 안 (A) 절을 그대로 옮겼고, 순서·수치·문구는 바꾸지 않았다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_paleo::pipeline::PaleoClass;

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// (A) 상단: 7대 고지형 분류도 실물 렌더 (`KaPaleoStudio.cpp:268-291`)
    pub(crate) fn render_paleo_map_preview(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
            let total_h = ui.available_height();
            let top_h = (total_h * 0.56).clamp(190.0, (total_h - 175.0).max(190.0));

            // (A) 상단: 7대 고지형 분류도 실물 렌더 (KaPaleoStudio.cpp:268-291)
            let (map_rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), top_h),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(map_rect);
            painter.rect_filled(map_rect, 4.0, Color32::from_rgb(242, 240, 234));

            // C++ `KaBoreholeDialog` 의 「적재된 시추공으로 선택 도면 다시 생성」 결과를
            // 실제로 칠한다. 이 블록이 없으면 파일만 만들어지고 화면은 비어 있다.
            let borehole_painted = crate::borehole_map_painter::render_borehole_map_overlay(
                self, ctx, &painter, map_rect,
            );

            if let Some(ref out) = self.paleo_3d.last_paleo_output {
                let w = out.transform.width;
                let h = out.transform.height;
                if w > 0 && h > 0 && out.paleo_rgba_buffer.len() >= w * h * 4 {
                    let rgba = if self.paleo_3d.show_imhof_slope_overlay {
                        out.paleo_rgba_buffer.clone()
                    } else {
                        let mut flat = Vec::with_capacity(w * h * 4);
                        for &cls in &out.paleo_class_grid {
                            flat.extend_from_slice(&cls.color_rgba());
                        }
                        flat
                    };
                    let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    let tex = ctx.load_texture(
                        "paleo_7class_map_preview",
                        img,
                        egui::TextureOptions::NEAREST,
                    );
                    let inner_map = map_rect.shrink(4.0);
                    painter.image(
                        tex.id(),
                        inner_map,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    painter.rect_stroke(
                        inner_map,
                        2.0,
                        Stroke::new(1.0, Color32::from_rgb(120, 113, 108)),
                        egui::StrokeKind::Middle,
                    );

                    // 월드 좌표 -> map_rect 화면 좌표 변환 클로저
                    let map_w_m = (w as f64) * out.transform.cell_size;
                    let map_h_m = (h as f64) * out.transform.cell_size;
                    let world_to_screen = |wx: f64, wy: f64| -> Pos2 {
                        let u = ((wx - out.transform.origin_x) / map_w_m).clamp(0.02, 0.98) as f32;
                        let v = ((out.transform.origin_y - wy) / map_h_m).clamp(0.05, 0.95) as f32;
                        Pos2::new(
                            inner_map.left() + u * inner_map.width(),
                            inner_map.top() + v * inner_map.height(),
                        )
                    };

                    // 조사구역(AOI) 경계 점선 사각형 표시
                    let aoi_r = Rect::from_center_size(
                        inner_map.center(),
                        Vec2::new(inner_map.width() * 0.34, inner_map.height() * 0.36),
                    );
                    painter.rect_stroke(
                        aoi_r,
                        2.0,
                        Stroke::new(1.6, Color32::from_rgb(185, 28, 28)),
                        egui::StrokeKind::Middle,
                    );
                    painter.text(
                        Pos2::new(aoi_r.left() + 5.0, aoi_r.top() + 4.0),
                        egui::Align2::LEFT_TOP,
                        "조사구역(AOI)",
                        egui::FontId::proportional(10.5),
                        Color32::from_rgb(153, 27, 27),
                    );

                    // A–A' 단면 절단선 표시
                    let sec_y = inner_map.center().y;
                    let p_a = Pos2::new(inner_map.left() + inner_map.width() * 0.06, sec_y);
                    let p_ap = Pos2::new(inner_map.right() - inner_map.width() * 0.06, sec_y);
                    painter.line_segment(
                        [p_a, p_ap],
                        Stroke::new(3.2, Color32::from_rgba_unmultiplied(255, 255, 255, 210)),
                    );
                    painter.line_segment(
                        [p_a, p_ap],
                        Stroke::new(2.0, Color32::from_rgb(185, 28, 28)),
                    );
                    for (pt, lbl) in [(p_a, "A"), (p_ap, "A'")] {
                        let badge = Rect::from_center_size(pt, Vec2::new(20.0, 18.0));
                        painter.rect_filled(badge, 4.0, Color32::from_rgb(185, 28, 28));
                        painter.text(
                            badge.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(11.0),
                            Color32::WHITE,
                        );
                    }

                    // 시추공(BH-01 ~ BH-08) 마커 및 충적층 두께 라벨 (PaleoAnalysisRunner.cpp:81-105)
                    for (idx, bh) in out.parsed_boreholes.iter().enumerate() {
                        let pt = world_to_screen(bh.x, bh.y);
                        let is_coarse = bh.sed_type.eq_ignore_ascii_case("COARSE");
                        let fill_c = if is_coarse {
                            Color32::from_rgb(255, 152, 0)
                        } else {
                            Color32::from_rgb(2, 132, 199)
                        };
                        painter.circle_filled(pt, 6.5, Color32::WHITE);
                        painter.circle_filled(pt, 5.0, fill_c);
                        painter.circle_stroke(pt, 5.0, Stroke::new(1.2, Color32::from_rgb(31, 41, 55)));

                        let dy = if idx % 2 == 0 { -16.0 } else { 15.0 };
                        let lbl = format!("{}({:.1}m)", bh.hole_id, bh.alluvial_thickness_m);
                        let pill = Rect::from_center_size(
                            Pos2::new(pt.x, pt.y + dy),
                            Vec2::new(66.0, 15.0),
                        );
                        painter.rect_filled(
                            pill,
                            3.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 230),
                        );
                        painter.rect_stroke(
                            pill,
                            3.0,
                            Stroke::new(0.7, Color32::from_rgb(120, 113, 108)),
                            egui::StrokeKind::Middle,
                        );
                        painter.text(
                            pill.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(28, 25, 23),
                        );
                    }

                    // 상단 제목 배지 및 하단 7대 미지형 색상 칩 바
                    let title_bg = Rect::from_min_size(
                        Pos2::new(inner_map.left() + 8.0, inner_map.top() + 8.0),
                        Vec2::new(410.0_f32.min(inner_map.width() - 16.0), 22.0),
                    );
                    painter.rect_filled(
                        title_bg,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 232),
                    );
                    painter.rect_stroke(
                        title_bg,
                        4.0,
                        Stroke::new(0.8, Color32::from_rgb(180, 176, 168)),
                        egui::StrokeKind::Middle,
                    );
                    painter.text(
                        Pos2::new(title_bg.left() + 8.0, title_bg.center().y),
                        egui::Align2::LEFT_CENTER,
                        "7대 고지형 분류도 (DEM 고도·경사·TPI·하천EDT 실물 분류 · A–A' 시추 단면선)",
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(28, 25, 23),
                    );

                    let strip_w = (inner_map.width() - 16.0).min(540.0);
                    let strip_r = Rect::from_min_size(
                        Pos2::new(inner_map.left() + 8.0, inner_map.bottom() - 24.0),
                        Vec2::new(strip_w, 19.0),
                    );
                    painter.rect_filled(
                        strip_r,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 235),
                    );
                    painter.rect_stroke(
                        strip_r,
                        4.0,
                        Stroke::new(0.8, Color32::from_rgb(180, 176, 168)),
                        egui::StrokeKind::Middle,
                    );
                    let cell_w = strip_w / 7.0;
                    for (i, cls) in PaleoClass::ALL.iter().enumerate() {
                        let [r, g, b, _] = cls.color_rgba();
                        let cx0 = strip_r.left() + (i as f32) * cell_w + 5.0;
                        let sw = Rect::from_min_size(
                            Pos2::new(cx0, strip_r.center().y - 5.0),
                            Vec2::new(10.0, 10.0),
                        );
                        painter.rect_filled(sw, 2.0, Color32::from_rgb(r, g, b));
                        painter.rect_stroke(
                            sw,
                            2.0,
                            Stroke::new(0.6, Color32::from_rgb(45, 43, 40)),
                            egui::StrokeKind::Middle,
                        );
                        painter.text(
                            Pos2::new(sw.right() + 3.5, strip_r.center().y),
                            egui::Align2::LEFT_CENTER,
                            cls.name_ko(),
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(28, 25, 23),
                        );
                    }
                }
            }

            // 칠한 시추 도면 표제를 지도 위에 남긴다(C++ 은 범례에 레이어명이 뜬다).
            if let Some(title) = borehole_painted {
                let chip = Rect::from_min_size(
                    Pos2::new(map_rect.left() + 10.0, map_rect.top() + 8.0),
                    Vec2::new(320.0, 20.0),
                );
                painter.rect_filled(
                    chip,
                    4.0,
                    Color32::from_rgba_unmultiplied(0x92, 0x40, 0x0E, 232),
                );
                painter.text(
                    Pos2::new(chip.left() + 7.0, chip.center().y),
                    egui::Align2::LEFT_CENTER,
                    title,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );
            }

            ui.add_space(4.0);
    }
}
