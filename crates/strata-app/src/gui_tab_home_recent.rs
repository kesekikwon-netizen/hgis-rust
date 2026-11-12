//! 「홈」 탭 — 최근 조사 목록과 작업 순서 안내.
//!
//! `gui_tab_home.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 개수·「<조사> 기준」은 최근 조사 목록에서 온다. 작업 순서의 단계 상태(C++ SurveyFacts)는 아직 고정이다.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 최근 조사(왼쪽) + 작업 순서(오른쪽) 두 칸.
    pub(crate) fn render_home_recent_and_guide(&mut self, ui: &mut egui::Ui, inner_w: f32) {
                // C++ KaStartPage.cpp: recent list takes ~77% and the guide column the rest.
                // Reserve the gap AND the guide column before sizing the table, otherwise the
                // guide column is pushed past the right edge (window 2308 -> table ran to 2307).
                const GUIDE_GAP: f32 = 18.0;
                const GUIDE_MIN_W: f32 = 300.0;
                let recent_w = (inner_w - GUIDE_GAP - GUIDE_MIN_W)
                    .min((inner_w * 0.77).max(520.0))
                    .max(320.0);
                let guide_w = (inner_w - recent_w - GUIDE_GAP).max(GUIDE_MIN_W);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(recent_w, 250.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("최근 조사").size(15.0).strong());
                            // `KaHomeRecentCard.cpp:121` — 목록이 비면 개수를 쓰지 않는다.
                            if !self.recent_surveys.is_empty() {
                                let count = format!("{}개", self.recent_surveys.len());
                                ui.label(egui::RichText::new(count).size(12.0).color(Color32::from_rgb(110, 108, 102)));
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let rf = ui.add(
                                    egui::TextEdit::singleline(&mut self.home_map.recent_filter)
                                        .hint_text("     이름·폴더로 찾기")
                                        .desired_width(260.0),
                                );
                                let sp = ui.painter();
                                let c = Pos2::new(rf.rect.left() + 11.0, rf.rect.center().y - 1.0);
                                let ink = Color32::from_rgb(110, 108, 102);
                                sp.circle_stroke(c, 4.4, Stroke::new(1.4, ink));
                                sp.line_segment(
                                    [Pos2::new(c.x + 3.2, c.y + 3.2), Pos2::new(c.x + 6.6, c.y + 6.6)],
                                    Stroke::new(1.5, ink),
                                );
                            });
                        });
                        ui.add_space(6.0);
                        self.render_recent_survey_box(ui, recent_w);
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("목록에서 빼려면 조사 위에서 오른쪽 단추를 누르세요.").size(11.5).color(Color32::from_rgb(105, 103, 96)));
                    });
                    ui.add_space(18.0);
                    ui.allocate_ui_with_layout(Vec2::new(guide_w, 250.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("작업 순서").size(15.0).strong());
                            // `KaHomeGuideCard.cpp:172-173` — 「<이어서 작업 조사> 기준」, 없으면 숨긴다.
                            if let Some(basis) = self.continue_survey().map(|it| format!("{} 기준", it.name)) {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(basis).size(11.5).color(Color32::from_rgb(105, 103, 96)));
                                });
                            }
                        });
                        ui.add_space(10.0);
                        // C++ KaHomeGuideCard.cpp: 18px circle nodes joined by a 1px vertical
                        // connector. Step 1 is done (check), step 2 is current (orange dot),
                        // step 3 is pending (number).
                        let ring_rgb = Color32::from_rgb(45, 44, 40);
                        let link_rgb = Color32::from_rgb(185, 182, 172);
                        let node_size = 18.0;
                        for (step_no, title, is_current, desc, step_id) in [
                            ("1", "새 조사", false, "조사 이름과 좌표계(5186·5187)를 정하면 조사 파일(GPKG)이 만들어집니다.", 1),
                            ("2", "조사구역 그리기", true, "지도 탭에서 위성·지적을 보며 「그리기」로 구역과 유구를 그립니다.", 2),
                            ("3", "도면 · 제출", false, "「도면」으로 종이에 옮기고, 「검수·제출」에서 검수한 뒤 제출 꾸러미(5179 SHP·조사도면.pdf·MANIFEST)를 만듭니다.\n아직 검수하지 않았습니다 — 지도 탭 「검수·제출」에서 확인합니다.", 3),
                        ] {
                            let row_top = ui.cursor().top();
                            ui.horizontal_top(|ui| {
                                let (node_r, _) = ui.allocate_exact_size(
                                    Vec2::new(node_size, node_size),
                                    egui::Sense::hover(),
                                );
                                let np = ui.painter();
                                let c = node_r.center();
                                // connector runs from this node's centre down past the row gap
                                let connector_top = c.y + node_size * 0.5;
                                let connector_bottom = row_top + 100.0;
                                if step_no != "3" {
                                    np.line_segment(
                                        [
                                            Pos2::new(c.x, connector_top),
                                            Pos2::new(c.x, connector_bottom),
                                        ],
                                        Stroke::new(1.0, link_rgb),
                                    );
                                }
                                np.circle_filled(c, node_size * 0.5, Color32::WHITE);
                                if is_current {
                                    np.circle_filled(c, 4.0, Color32::from_rgb(181, 87, 58));
                                } else {
                                    np.circle_stroke(
                                        c,
                                        node_size * 0.5,
                                        Stroke::new(1.2, ring_rgb),
                                    );
                                    if step_no == "1" {
                                        np.line_segment(
                                            [
                                                Pos2::new(c.x - 3.6, c.y + 0.2),
                                                Pos2::new(c.x - 1.0, c.y + 2.8),
                                            ],
                                            Stroke::new(1.5, ring_rgb),
                                        );
                                        np.line_segment(
                                            [
                                                Pos2::new(c.x - 1.0, c.y + 2.8),
                                                Pos2::new(c.x + 3.8, c.y - 2.4),
                                            ],
                                            Stroke::new(1.5, ring_rgb),
                                        );
                                    } else {
                                        np.text(
                                            c,
                                            egui::Align2::CENTER_CENTER,
                                            step_no,
                                            egui::FontId::proportional(9.5),
                                            ring_rgb,
                                        );
                                    }
                                }
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.link(egui::RichText::new(title).strong().size(13.0)).clicked() {
                                            match step_id {
                                                1 => self.modals.active = Some(ActiveModalDialog::NewSurvey),
                                                2 => self.trigger_map_ribbon_tool(MapToolKind::DrawPolygon),
                                                _ => self.modals.active = Some(ActiveModalDialog::PaperSetup),
                                            }
                                        }
                                        if is_current {
                                            ui.colored_label(Color32::from_rgb(181, 87, 58), egui::RichText::new("• 다음").size(11.5).strong());
                                        }
                                    });
                                    ui.label(egui::RichText::new(desc).size(11.5).color(Color32::from_rgb(95, 94, 88)));
                                });
                            });
                            ui.add_space(8.0);
                        }
                        // Workflow state is still evaluated (callers rely on the side effects of
                        // the counters below), but C++ shows no bottom progress sentence.
                        let _wf_states = strata_storage::project_actions::evaluate_workflow_guide(
                            &strata_storage::project_actions::WorkflowProjectInput {
                                survey_name: self.project_title.clone(),
                                layer_count: self.left_layers.entries.len(),
                                survey_area_count: self.home_map.real_survey.survey_rings.len(),
                                feature_poly_count: self.home_map.real_survey.trenches.len(),
                                feature_line_count: 0,
                                control_points_count: 2,
                                has_datum: true,
                                has_ellipsoid: true,
                                has_projection: true,
                                has_visible_reference_layer: true,
                                checklist_error_count: 0,
                                package_created: false,
                            },
                        );
                    });
                });
    }
}
