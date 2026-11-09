//! 지도 측정 HUD — 클릭 점·거리·면적·각도 표시.
//!
//! `gui_tab_map_hud` 에서 300줄을 지키려고 나눴다(원래 한 블록).

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;

use super::gui_tab_map_geom::{
    format_area_m2, format_length_m, grid_bearing_degrees, ring_perimeter_m,
};

impl StrataAppState {
    /// 측정 중 HUD (원문 `if is_measure` 블록 그대로).
    pub(crate) fn paint_map_measure_hud(
        &mut self,
        ui: &mut egui::Ui,
        p: &egui::Painter,
        rect: Rect,
    ) {
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

            // KaMeasureMapTool.cpp:357-405 refreshHud — 총계는 MeasureOps 포맷,
            // 상세줄은 「· 」로 이은 조각들 + `작업좌표 … 평면`.
            let auth_id = self.work_crs.auth_id();
            // 아래에서 씬을 다시 빌리므로(마침·한 점 취소·다시) 사본으로 둔다.
            let ring = self.home_map.scene.active_rubberband.clone();
            let finished = self.home_map.cad.last_measure.is_some();
            let readout = if area_mode {
                let area = self
                    .home_map
                    .cad
                    .last_measure
                    .as_ref()
                    .map_or(0.0, |m| m.polygon_area_m2);
                format_area_m2(area)
            } else {
                let len = self
                    .home_map
                    .cad
                    .last_measure
                    .as_ref()
                    .map_or(0.0, |m| m.total_distance_m);
                format_length_m(len)
            };
            p.text(
                Pos2::new(mr.left() + 12.0, mr.top() + 58.0),
                egui::Align2::LEFT_CENTER,
                &readout,
                FontId::proportional(19.0),
                Color32::BLACK,
            );
            // :372-399 — 면적은 둘레·확정 안내, 거리는 구간별 길이·도북 방위.
            let mut bits: Vec<String> = Vec::new();
            if area_mode {
                let perimeter = ring_perimeter_m(&ring);
                if ring.len() >= 3 {
                    bits.push(format!("둘레 {}", format_length_m(perimeter)));
                }
                if finished {
                    bits.push("면적만 산출".to_owned());
                } else if ring.len() < 3 {
                    bits.push("세 점 이상 찍고 우클릭 → 마침".to_owned());
                } else {
                    bits.push("우클릭에서 마침을 고르면 면적만 확정".to_owned());
                }
            } else if ring.len() < 2 {
                bits.push("두 점 이상 찍고 우클릭 → 마침".to_owned());
            } else {
                for (i, w) in ring.windows(2).take(8).enumerate() {
                    let d = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
                    bits.push(format!("{}구간 {}", i + 1, format_length_m(d)));
                }
                let n = ring.len();
                bits.push(format!(
                    "도북 방위 {}",
                    strata_canvas2d::cad_interaction::format_bearing_dms(
                        grid_bearing_degrees(ring[n - 2], ring[n - 1])
                    )
                ));
                if !finished {
                    bits.push("우클릭 → 마침".to_owned());
                }
            }
            bits.push(format!("작업좌표 {auth_id} 평면"));
            let detail = bits.join(" · ");
            p.text(
                Pos2::new(mr.left() + 12.0, mr.top() + 86.0),
                egui::Align2::LEFT_CENTER,
                &detail,
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
                        1 => {
                            // KaMeasureMapTool.cpp:331-355 copyText() — 면적은 면적+둘레,
                            // 거리는 거리+구간별(길이·도북 방위), 끝에 작업좌표 한 줄.
                            let mut lines = Vec::new();
                            if area_mode {
                                let area = self
                                    .home_map
                                    .cad
                                    .last_measure
                                    .as_ref()
                                    .map_or(0.0, |m| m.polygon_area_m2);
                                lines.push(format!("면적 {}", format_area_m2(area)));
                                if ring.len() >= 3 {
                                    lines.push(format!(
                                        "둘레 {}",
                                        format_length_m(ring_perimeter_m(&ring))
                                    ));
                                }
                            } else {
                                let len = self
                                    .home_map
                                    .cad
                                    .last_measure
                                    .as_ref()
                                    .map_or(0.0, |m| m.total_distance_m);
                                lines.push(format!("거리 {}", format_length_m(len)));
                                for (i, w) in ring.windows(2).enumerate() {
                                    let d = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
                                    lines.push(format!(
                                        "{}구간 {} · 도북 방위 {}",
                                        i + 1,
                                        format_length_m(d),
                                        strata_canvas2d::cad_interaction::format_bearing_dms(
                                            grid_bearing_degrees(w[0], w[1])
                                        )
                                    ));
                                }
                            }
                            lines.push(format!("작업좌표 {auth_id} 평면"));
                            self.status_bar_hint =
                                "줄자 숫자를 복사했습니다.".to_owned();
                            ui.ctx().copy_text(lines.join("\n"));
                        }
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
}
