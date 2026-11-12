//! 「자석」 옆 ▾ 자석 보조 메뉴 — C++ addDrawSketchButtons 의 assistMenu(KaDrawSketchTools.cpp:42-90).
//! `gui_ribbon_subtoolbar.rs` 에 있던 펼침 메뉴를 그리기 줄 파일을 나누며 그대로 옮겼다.

use eframe::egui::{self, Color32, Pos2, Stroke};

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// ▾ 를 눌러 연 자석 보조 메뉴(단추 바로 아래).
    pub(crate) fn paint_snap_assist_popup(&mut self, ctx: &egui::Context, ui: &egui::Ui, assist_rect: egui::Rect) {
        if !self.ribbon.snap_assist_menu_open {
            return;
        }
        let popup_pos = Pos2::new(assist_rect.left(), assist_rect.bottom() + 4.0);
        egui::Area::new(ui.id().with("ka_snap_assist_menu"))
            .fixed_pos(popup_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgb(252, 251, 247))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_min_width(188.0);
                        let snap_now = ui
                            .button("가까운 선에 붙이기")
                            .on_hover_text(
                                "지적도·사업자 도면 위에 그린 도형(방금 그린 도형 또는 선택한 도형)을\n가장 가까운 지적선·도면선에 자동으로 붙이고 빠진 사이 꺾임점도 채웁니다 (Ctrl+Z 되돌리기 가능).",
                            );
                        if snap_now.clicked() {
                            let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                            if changed > 0 {
                                self.is_dirty = true;
                            }
                            self.ribbon.snap_assist_menu_open = false;
                        }
                        ui.separator();
                        let prev_trace = self.home_map.cad.snap_trace_enabled;
                        if ui
                            .checkbox(&mut self.home_map.cad.snap_trace_enabled, "선 따라 자동채움")
                            .on_hover_text(
                                "1번 기능(선 따라 빠진 사이 자석점 자동 채우기)\n· 그릴 때 자석점 한두 칸을 건너뛰어도 사이 꼭짓점을 선을 따라 자동으로 채우며,\n  이미 그린 도형이 있을 때 누르면 가장 가까운 선에 바로 붙입니다.",
                            )
                            .changed()
                        {
                            let on = self.home_map.cad.snap_trace_enabled;
                            if on && !self.home_map.cad.snap_enabled {
                                self.home_map.cad.snap_enabled = true;
                            }
                            if on && !prev_trace {
                                let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                if changed == 0 {
                                    self.home_map.draw_guide_message = Some(
                                        "선 따라 자동채움 켜짐 — 자석점을 한두 칸 건너뛰어 찍어도 사이 꼭짓점을 자동으로 채웁니다."
                                            .to_owned(),
                                    );
                                } else {
                                    self.is_dirty = true;
                                }
                            } else if !on {
                                self.home_map.draw_guide_message = Some("선 따라 자동채움 꺼짐".to_owned());
                            }
                        }
                        if ui
                            .checkbox(&mut self.home_map.cad.snap_preview_enabled, "근접 자석점 표시")
                            .on_hover_text(
                                "3번 기능(마우스가 가까이 갔을 때만 주변 자석점 미리 표시)\n· 마우스가 선·꼭짓점에 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다.",
                            )
                            .changed()
                        {
                            let on = self.home_map.cad.snap_preview_enabled;
                            if on && !self.home_map.cad.snap_enabled {
                                self.home_map.cad.snap_enabled = true;
                            }
                            self.home_map.draw_guide_message = Some(if on {
                                "근접 자석점 표시 켜짐 — 마우스가 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다."
                                    .to_owned()
                            } else {
                                "근접 자석점 표시 꺼짐".to_owned()
                            });
                        }
                    });
            });
    }
}
