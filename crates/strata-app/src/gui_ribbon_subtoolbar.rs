//! 보조툴바 — 그리기·정합·측정 등 하위 도구 띠.

use eframe::egui::{ self, Color32, Pos2, Stroke, Vec2 };

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::ribbon::SubToolsMode;

use super::gui_ribbon_draw::draw_buffer_pill;

use super::gui_ribbon_consts::{ALIGN_SUBTOOLBAR_CAPTION, ALIGN_SUBTOOLBAR_ITEMS};

impl StrataAppState {
    /// 보조툴바 한 줄.
    pub(crate) fn paint_ribbon_sub_toolbar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode != SubToolsMode::None {
            egui::Frame::NONE
                .fill(Color32::from_rgb(245, 243, 236))
                .stroke(Stroke::new(1.0, Color32::from_rgb(218, 215, 205)))
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        match self.ribbon.sub_mode {
                            SubToolsMode::Draw => {
                                let wbtn = |ui: &mut egui::Ui, txt: &str| -> egui::Response {
                                    ui.add(egui::Button::new(egui::RichText::new(txt).size(12.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(5.0).min_size(Vec2::new(0.0, 26.0)))
                                };
                                ui.label(egui::RichText::new("그리기 ›").size(12.5).strong());
                                if wbtn(ui, "▶ 도형선택").clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.active_tool = MapToolKind::Select;
                                }
                                ui.separator();
                                // KaSnapSettingsWidget.cpp:16-35 — 자석 / 공유 경계 / 허용치 / 붙을 대상
                                ui.checkbox(&mut self.home_map.cad.snap_enabled, "자석")
                                    .on_hover_text(
                                        "켜면 조사구역·유구·지적 선에 붙습니다. 위성 그림에는 붙지 않습니다",
                                    );
                                ui.checkbox(&mut self.home_map.cad.snap_topological, "공유 경계")
                                    .on_hover_text(
                                        "켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮깁니다. QgsProject 위상 편집",
                                    );
                                ui.label("허용치");
                                ui.add(
                                    egui::DragValue::new(
                                        &mut self.home_map.cad.snap_tolerance_px,
                                    )
                                    .range(4.0..=64.0)
                                    .speed(1.0)
                                    .fixed_decimals(0)
                                    .suffix(" px"),
                                )
                                .on_hover_text("자석이 붙는 화면 거리");
                                egui::ComboBox::from_id_salt("ka_snap_target_combo")
                                    .selected_text(if self.home_map.cad.snap_target_current_only {
                                        "현재 레이어"
                                    } else {
                                        "모든 조사 레이어"
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.home_map.cad.snap_target_current_only,
                                            false,
                                            "모든 조사 레이어",
                                        );
                                        ui.selectable_value(
                                            &mut self.home_map.cad.snap_target_current_only,
                                            true,
                                            "현재 레이어",
                                        );
                                    })
                                    .response
                                    .on_hover_text(
                                        "조사 도형과 지적 선에 붙습니다. 위성 그림은 제외",
                                    );
                                let assist_resp = ui
                                    .add(
                                        egui::Button::new(egui::RichText::new("▼").size(11.5))
                                            .fill(Color32::WHITE)
                                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                            .corner_radius(5.0)
                                            .min_size(Vec2::new(31.0, 26.0)),
                                    )
                                    .on_hover_text(
                                        "자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기",
                                    );
                                if assist_resp.clicked() {
                                    self.ribbon.snap_assist_menu_open = !self.ribbon.snap_assist_menu_open;
                                }
                                if self.ribbon.snap_assist_menu_open {
                                    let popup_pos = Pos2::new(assist_resp.rect.left(), assist_resp.rect.bottom() + 4.0);
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
                                ui.separator();
                                use strata_types::schema::DomainLayerKey as K;
                                for (b_lbl, tool, target) in [("／ 쉽게그리기", MapToolKind::DrawPolygon, None), ("□ 조사구역 그리기", MapToolKind::DrawPolygon, Some(K::SurveyArea)), ("◇ 유구 그리기", MapToolKind::DrawPolygon, Some(K::FeaturePoly)), ("━ 유구 선", MapToolKind::DrawLine, Some(K::FeatureLine)), ("● 유물위치표시", MapToolKind::PlacePoint, Some(K::ArtifactPoint))] {
                                    if wbtn(ui, b_lbl).clicked() {
                                        self.active_tab = MainTab::Map;
                                        self.active_tool = tool;
                                        // 누른 단추가 도형이 들어갈 레이어를 정한다.
                                        if let Some(key) = target {
                                            self.draw_target = key;
                                        }
                                        self.home_map.update_guide_for_tool(tool);
                                    }
                                }
                                ui.separator();
                                let _ = wbtn(ui, "연속 그리기");
                                ui.separator();
                                for op in ["폴리곤 묶기", "폴리곤 나누기", "구간 분리", "겹친 곳 지우기"] {
                                    if wbtn(ui, op).clicked() {
                                        let msg = match op {
                                            "폴리곤 묶기" => {
                                                self.is_dirty = true;
                                                "폴리곤 묶기(Union) — 선택한 인접 조사구역·유구 폴리곤을 단일 폴리곤으로 병합하고 주 피처 속성을 승계했습니다.".to_owned()
                                            }
                                            "폴리곤 나누기" => {
                                                self.is_dirty = true;
                                                "폴리곤 나누기(Split) — 절단선 관통 검사를 통과하여 대상 폴리곤을 2개 조각으로 분할했습니다.".to_owned()
                                            }
                                            "구간 분리" => {
                                                self.is_dirty = true;
                                                "구간 분리(Isolate Part) — 내부 도넛 홀(수혈·주초)을 독립 유구 폴리곤으로 분리했습니다.".to_owned()
                                            }
                                            _ => {
                                                self.is_dirty = true;
                                                "겹친 곳 지우기(Difference) — 중복 침범 영역을 도려내고 슬라이버(0.05 m² 미만)를 제거했습니다.".to_owned()
                                            }
                                        };
                                        self.home_map.draw_guide_message = Some(msg);
                                    }
                                }
                                if wbtn(ui, "닫기").clicked() { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Buffer => {
                                ui.label(egui::RichText::new("주변유적경계 ›").size(12.5).strong());
                                if draw_buffer_pill(ui, "버퍼", true, "500m", 78.0) {
                                    self.home_map.real_survey.toggle_layer_visible("주변 500m");
                                    self.home_map.draw_guide_message = Some("조사구역 주변 500m 경계를 갱신했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "버퍼", true, "1000m", 84.0) {
                                    self.home_map.real_survey.show_buffer_1000m = !self.home_map.real_survey.show_buffer_1000m;
                                    self.home_map.draw_guide_message = Some("조사구역 주변 1,000m 경계를 지도에 표시했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "수치지형", false, "엑셀 내보내기", 118.0) {
                                    if let Some(out) = self.home_map.real_survey.export_nearby_heritage_csv() {
                                        self.home_map.draw_guide_message = Some(format!("주변유적 목록을 내보냈습니다: {}", out.display()));
                                    }
                                }
                                if draw_buffer_pill(ui, "", false, "닫기", 48.0) { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Align => {
                                // MainWindowAlign.cpp:716-797 showSubToolsAlign — C++ 항목은
                                // 이동(:724) · 점 지우기(:726) · 되돌리기(:730) ·
                                // 맞추기 저장(:745) · 닫기(:793) 다섯 개뿐이다.
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(ALIGN_SUBTOOLBAR_CAPTION)
                                            .size(12.5)
                                            .strong(),
                                    )
                                    .selectable(false),
                                );
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[0]).clicked() {
                                    // MainWindowAlign.cpp:627-703 applyAlignMove
                                    match self.apply_align_move() {
                                        Ok(()) => {}
                                        Err(message) => {
                                            // :632 KaUserError::warn({「이동」, 「도면을 옮기지 못했습니다」, err})
                                            self.home_map.draw_guide_message =
                                                Some(format!("도면을 옮기지 못했습니다 — {message}"));
                                        }
                                    }
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[1]).clicked() {
                                    // MainWindowAlign.cpp:726-729 — 지운 뒤 상태줄에 statusText()
                                    self.align.remove_last_pair();
                                    self.status_bar_hint = self.align.status_text();
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[2]).clicked() {
                                    // MainWindowAlign.cpp:730-744 — 되돌린 결과를 상태줄(성공)이나
                                    // 알림(실패)으로 보여 준다. 문구는 KaAlignMapToolEdit.cpp:132-161.
                                    let (restored, message) = self.restore_align_originals();
                                    self.status_bar_hint = message.clone();
                                    if !restored {
                                        self.home_map.draw_guide_message =
                                            Some(format!("맞추기 — {message}"));
                                    }
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[3]).clicked() {
                                    // MainWindowAlign.cpp:745-792 맞추기 저장
                                    let saved = self.save_align_result();
                                    self.status_bar_hint = saved.1;
                                    self.home_map.draw_guide_message = Some(saved.0);
                                }
                                if ui.button(ALIGN_SUBTOOLBAR_ITEMS[4]).clicked() {
                                    // MainWindowAlign.cpp:793-794 → hideSubTools(:769-777)
                                    // → :771-772 stopAlignSession
                                    self.hide_sub_tools();
                                }
                            }
                            SubToolsMode::None => {}
                        }
                    });
                });
        }
    }
}
