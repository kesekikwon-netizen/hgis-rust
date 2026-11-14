//! 하단 7버튼 툴바 — C++ `KaDrawingStudio.cpp:1292-1323`.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// 일곱 단추(좌표점·가운데·지도조정·화면맞춤·항목이동·용지/방향·도면목록).
    pub(crate) fn paint_sheet_toolbar(&mut self, ui: &mut egui::Ui, f: &SheetFrame) {
        let rect = f.rect;
        let p = f.p.clone();
        let compact_v = f.compact_v;
                    let bar_w = 504.0_f32.min((rect.width() - 12.0).max(210.0));
                    let bar_h_px = if compact_v { 34.0 } else { 42.0 };
                    let bar_cy = rect.bottom() - bar_h_px * 0.5 - 4.0;
                    let bar_r = Rect::from_center_size(Pos2::new(rect.center().x, bar_cy), Vec2::new(bar_w, bar_h_px));
                    p.rect_filled(bar_r, 6.0, Color32::WHITE);
                    p.rect_stroke(bar_r, 6.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
                                  // KaDrawingStudio.cpp:1292-1323 — 7버튼 툴팁
                      let btns: [(&str, &str, &str); 7] = [
                          ("■", "좌표점", "용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다"),
                          ("◎", "레이어를 가운데", "축척은 두고, 고른 레이어를 용지 한가운데로 옮깁니다"),
                          ("□", "지도 조정", "지도 칸 안의 지도를 끌어 옮기고 휠로 확대합니다. 다시 누르면 끝냅니다"),
                          ("▣", "화면 맞춤", "지도 화면과 같은 범위·축척으로 지도 칸을 다시 맞춥니다"),
                          ("✥", "항목 이동", "좌표 상자를 끌어 옮깁니다"),
                          ("▯", "용지/방향", "용지 크기(A4/A3) 및 방향(가로/세로)을 전환합니다"),
                          ("≡", "도면 목록", "지금 도면을 이름 붙여 보관하고, 보관한 도면을 다시 엽니다. 제출 PDF는 지금 열린 도면입니다"),
                      ];
                    let bw = bar_w / (btns.len() as f32);
                    for (i, (_ico, lbl, tip)) in btns.iter().enumerate() {
                        let x0 = bar_r.left() + bw * (i as f32);
                        if i > 0 {
                            p.line_segment([Pos2::new(x0, bar_r.top() + 4.0), Pos2::new(x0, bar_r.bottom() - 4.0)], Stroke::new(1.0, Color32::from_rgb(222, 220, 210)));
                        }
                        let cell_r = Rect::from_min_size(Pos2::new(x0, bar_r.top()), Vec2::new(bw, bar_h_px));
                        let ic_c = Pos2::new(cell_r.center().x, cell_r.top() + bar_h_px * 0.30);
                        let ic_col = Color32::from_rgb(55, 54, 50);
                        match i {
                            0 => {
                                p.circle_stroke(ic_c, 4.5, Stroke::new(1.4, ic_col));
                                p.circle_filled(ic_c, 1.5, ic_col);
                            }
                            1 => {
                                p.circle_stroke(ic_c, 5.0, Stroke::new(1.3, ic_col));
                                p.line_segment([Pos2::new(ic_c.x - 6.5, ic_c.y), Pos2::new(ic_c.x + 6.5, ic_c.y)], Stroke::new(1.2, ic_col));
                                p.line_segment([Pos2::new(ic_c.x, ic_c.y - 6.5), Pos2::new(ic_c.x, ic_c.y + 6.5)], Stroke::new(1.2, ic_col));
                            }
                            2 => {
                                let r = Rect::from_center_size(ic_c, Vec2::new(11.0, 9.0));
                                p.rect_stroke(r, 1.5, Stroke::new(1.3, ic_col), egui::StrokeKind::Inside);
                                p.circle_filled(ic_c, 1.8, ic_col);
                            }
                            3 => {
                                let r = Rect::from_center_size(ic_c, Vec2::new(11.0, 9.0));
                                p.rect_stroke(r, 1.5, Stroke::new(1.3, ic_col), egui::StrokeKind::Inside);
                                p.rect_filled(r.shrink(2.8), 0.5, ic_col);
                            }
                            4 => {
                                p.line_segment([Pos2::new(ic_c.x - 5.5, ic_c.y), Pos2::new(ic_c.x + 5.5, ic_c.y)], Stroke::new(1.4, ic_col));
                                p.line_segment([Pos2::new(ic_c.x, ic_c.y - 5.5), Pos2::new(ic_c.x, ic_c.y + 5.5)], Stroke::new(1.4, ic_col));
                            }
                            5 => {
                                let r = Rect::from_center_size(ic_c, Vec2::new(9.0, 11.0));
                                p.rect_stroke(r, 1.5, Stroke::new(1.3, ic_col), egui::StrokeKind::Inside);
                            }
                            _ => {
                                for dy in [-3.5_f32, 0.0, 3.5] {
                                    p.line_segment([Pos2::new(ic_c.x - 5.5, ic_c.y + dy), Pos2::new(ic_c.x + 5.5, ic_c.y + dy)], Stroke::new(1.4, ic_col));
                                }
                            }
                        }
                        p.text(Pos2::new(cell_r.center().x, cell_r.bottom() - bar_h_px * 0.24), egui::Align2::CENTER_CENTER, *lbl, egui::FontId::proportional(if compact_v { 8.0 } else { 10.0 }), Color32::from_rgb(55, 54, 50));
                        let cell_resp = ui
                              .interact(cell_r, ui.id().with(("drawing_toolbar_btn", i)), egui::Sense::click())
                              .on_hover_text(*tip);
                        cell_resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), *lbl)
                        });
                          if cell_resp.clicked() {
                            match i {
                                0 => {
            // KaDrawingCoordCallout.cpp — 용지 위 좌표점 콜아웃 모드
            self.drawing.coord_callout_mode = !self.drawing.coord_callout_mode;
            self.home_map.draw_guide_message = Some(if self.drawing.coord_callout_mode {
                "좌표점 모드 — 용지에서 꼭짓점을 찍으면 화살표와 X·Y가 붙습니다.".to_owned()
            } else {
                "좌표점 모드를 껐습니다.".to_owned()
            });
        }
                                1 => {
                                    self.center_layer_in_sheet();
                                }
                                2 => {
            // KaDrawingStudio.cpp:822-838 m_adjustBar — 지도 조정 모드
            self.drawing.map_adjust_active = !self.drawing.map_adjust_active;
            self.home_map.draw_guide_message = Some(if self.drawing.map_adjust_active {
                "지도 조정: 드래그로 이동, 휠로 확대. 끝나면 「지도조정끝」.".to_owned()
            } else {
                "지도 조정을 끝냈습니다.".to_owned()
            });
        }
                                3 => {
                                    // 지도 화면과 같은 범위로 — 「레이어를 가운데」로 옮긴 가운데를 푼다(C++ centerOnMapCanvas).
                                    self.drawing.sheet_center = None;
                                    self.drawing.sheet.replace_sheet_chrome();
                                    self.drawing.sheet.recover_orphaned_items();
                                    self.home_map.draw_guide_message = Some("용지 화면 맞춤 및 하단 장식 띠(축척자·좌표계·방위) 정렬을 완료했습니다.".to_owned());
                                }
                                4 => {
                                    self.home_map.draw_guide_message = Some("항목 이동 모드 — 범례·표제란·방위표를 끌어 자리를 옮기세요.".to_owned());
                                }
                                5 => {
                                    // target/cpp-ref/src/app/KaDrawingStudio.cpp:447-454, 1317-1319 — openPaperSettingsDialog()
                                    self.modals.active =
                                        Some(crate::dialogs_modal::ActiveModalDialog::PaperSetup);
                                }
                                _ => {
            // KaDrawingSheetSet.cpp:135 — 도면 목록 팝업
            self.drawing.sheet_set_menu_open = !self.drawing.sheet_set_menu_open;
        }
                            }
                        }
                    }
    }
}
