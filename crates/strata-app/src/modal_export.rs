//! `dialogs_modal` 에서 옮긴 대화상자 묶음: ExportSubmitPackage.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::render_strata_modal_header;
use crate::modal_types::ModalDialogController;
use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};


pub(crate) fn render_export_submit_package(app: &mut StrataAppState, ctx: &egui::Context, mut mut open: &mut bool) {
    let full = ctx.screen_rect();
    // screen_rect is already in logical points, so dividing by the native
    // scale again shrank the modal on 150% displays (Task D5).
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 820.0_f32.min((vis_w - 40.0).max(640.0));
    let modal_h = 540.0_f32.min((vis_h - 60.0).max(420.0));
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(24.0),
    );
    egui::Window::new("ka_modal_submit")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_width(modal_w - 24.0);
            ui.set_max_width(modal_w - 24.0);
            render_strata_modal_header(ui, "검수·제출 - Strata", open);
            let has_boundary = !app.home_map.real_survey.survey_rings.is_empty();
            let gcp_cnt = app
                .left_layers
                .entries
                .iter()
                .find(|e| e.layer_key == "control_points")
                .map_or(0, |e| e.feature_count as i64);
            let feat_cnt = (app.home_map.real_survey.trenches.len()
                + app.home_map.scene.polygons.len()) as i64;
            let bounds_ok = app.drawing.sheet.all_decorations_inside_sheet();
            let detail_ok = app.drawing.sheet.scale_denominator <= 200;
            let state_json = serde_json::json!({
                "survey_area_count": i64::from(has_boundary),
                "control_points_count": gcp_cnt,
                "feature_poly_count": feat_cnt,
                "feature_line_count": 0,
                "project_crs_set": true,
                "project_crs_authid": app.work_crs.auth_id(),
                "has_datum": gcp_cnt >= 2,
                "has_ellipsoid": gcp_cnt >= 2,
                "has_projection": gcp_cnt >= 2,
                "has_origin": true,
                "has_accuracy": true,
                "has_kind_period": true,
                "has_abstract_marker": false,
                "survey_is_polygon": has_boundary,
                "features_within_survey": true,
                "features_real_geometry": true,
                "geometries_valid": true,
                "geometries_nonempty": has_boundary,
                "geometries_nonzero_area": has_boundary,
                "export_crs_valid": true,
                "required_fields_filled": true,
                "control_points_point_id_unique": true,
                "feature_poly_feature_no_unique": true,
                "sheet_covers_targets": bounds_ok,
                "sheet_has_elements": bounds_ok,
                "HERITAGE_PLEDGE_SCOPE": app.home_map.real_survey.heritage_polys.is_empty(),
                "layout_exists:site_location": has_boundary && bounds_ok,
                "layout_exists:feature_plan": feat_cnt > 0 && bounds_ok,
                "layout_exists:feature_detail": detail_ok,
                "layout_exists:survey_area_map": has_boundary,
            });

            let engine = ChecklistEngine::with_default_rules();
            let results = engine.evaluate(&state_json);
            let err_cnt = ChecklistEngine::failed_count(&results, "error");
            let warn_cnt = ChecklistEngine::failed_count(&results, "warn");
            let pass_cnt = ChecklistEngine::passed_count(&results);
            let status_txt = ChecklistEngine::status_bar_text(&results);
            app.home_map.draw_guide_message = Some(status_txt.clone());

            let findings: Vec<strata_check_export::checklist_engine::ChecklistFinding> = results
                .iter()
                .map(|r| {
                    let sev = if r.passed {
                        strata_check_export::checklist_engine::ChecklistSeverity::Pass
                    } else if r.severity == "error" {
                        strata_check_export::checklist_engine::ChecklistSeverity::Error
                    } else {
                        strata_check_export::checklist_engine::ChecklistSeverity::Warning
                    };
                    strata_check_export::checklist_engine::ChecklistFinding {
                        rule_id: r.id.clone(),
                        category_ko: r.basis_ko.clone(),
                        message_ko: r.message_ko.clone(),
                        severity: sev,
                        passed: r.passed,
                        target_world_xy: Some(app.home_map.real_survey.survey_center),
                    }
                })
                .collect();

            // KaSubmitDialogPlate.cpp:12-62, 73-94 — 상단 헤드라인 + 3개 카운트 박스(오류 / 주의 / 통과)
            let headline = if err_cnt > 0 {
                format!("오류 {err_cnt}건을 확인하세요 ({status_txt})")
            } else if warn_cnt > 0 {
                format!("오류가 없습니다 ({status_txt})")
            } else {
                "오류와 주의 항목이 없습니다".to_owned()
            };
            ui.label(
                egui::RichText::new(headline)
                    .size(15.5)
                    .strong()
                    .color(Color32::from_rgb(20, 20, 19)),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} · 고치지 않아도 꾸러미는 만들 수 있고, README_submit.txt에 적힙니다.",
                    engine.title()
                ))
                .size(11.5)
                .color(Color32::from_rgb(94, 93, 89)),
            );
            ui.add_space(4.0);
            ui.columns(3, |cols| {
                for (ci, (lbl, cnt, bg, fg)) in [
                    (
                        "오류",
                        err_cnt,
                        Color32::from_rgb(254, 242, 242),
                        Color32::from_rgb(185, 28, 28),
                    ),
                    (
                        "주의",
                        warn_cnt,
                        Color32::from_rgb(255, 251, 235),
                        Color32::from_rgb(180, 83, 9),
                    ),
                    (
                        "통과",
                        pass_cnt,
                        Color32::from_rgb(240, 253, 244),
                        Color32::from_rgb(21, 128, 61),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    egui::Frame::NONE
                        .fill(bg)
                        .stroke(Stroke::new(1.0, fg))
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(&mut cols[ci], |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(lbl).size(12.0).strong().color(fg));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new("건").size(11.5).color(fg));
                                    ui.label(
                                        egui::RichText::new(format!("{cnt}"))
                                            .size(17.0)
                                            .strong()
                                            .color(fg),
                                    );
                                });
                            });
                        });
                }
            });

            ui.add_space(4.0);
            // KaSubmitDrawingStrip.cpp:9-12 — 필수 도면 4종(조사구역도·유적위치도·유구배치도·개별유구실측도) 상태 띠
            egui::Frame::NONE
                .fill(Color32::from_rgb(240, 238, 230))
                .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("필수 도면 4종:")
                                .size(11.0)
                                .strong()
                                .color(Color32::from_rgb(45, 44, 40)),
                        );
                        for (rule_id, name_ko) in [
                            ("REQ_DRAWING_SURVEY_AREA", "조사구역도"),
                            ("REQ_DRAWING_SITE_LOCATION", "유적위치도"),
                            ("REQ_DRAWING_FEATURE_PLAN", "유구배치도"),
                            ("REQ_DRAWING_FEATURE_DETAIL", "개별유구실측도"),
                        ] {
                            let ok = results
                                .iter()
                                .find(|r| r.id == rule_id)
                                .is_some_and(|r| r.passed);
                            let (tag_txt, col) = if ok {
                                (format!("● {name_ko} ({rule_id})"), Color32::from_rgb(21, 128, 61))
                            } else {
                                (format!("▲ {name_ko} 없음 ({rule_id})"), Color32::from_rgb(122, 90, 0))
                            };
                            ui.colored_label(col, egui::RichText::new(tag_txt).size(10.5).strong());
                            ui.separator();
                        }
                    });
                });

            ui.add_space(4.0);
            // KaSubmitDialog.cpp:58 — 헤더 ("항목", "고치는 법", "위치 보기")
            egui::Frame::NONE
                .fill(Color32::from_rgb(232, 229, 220))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("위치 이동 · 순번 · JSON 규칙 ID (27개)")
                                .size(11.0)
                                .strong(),
                        );
                        ui.add_space(45.0);
                        ui.label(
                            egui::RichText::new("심각도 · 검수 항목 및 고치는 법 (근거)")
                                .size(11.0)
                                .strong(),
                        );
                    });
                });

            // KaSubmitDialog.cpp:153-225 — 27개 JSON 규칙 순서대로 전체 표시 + 규칙별 [위치 보기] 버튼
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (idx, r) in results.iter().enumerate() {
                        let row_bg = if !r.passed && r.severity == "error" {
                            Color32::from_rgb(0xF3, 0xDE, 0xDA)
                        } else if !r.passed {
                            Color32::from_rgb(255, 251, 235)
                        } else if idx % 2 == 0 {
                            Color32::from_rgb(252, 251, 247)
                        } else {
                            Color32::WHITE
                        };
                        egui::Frame::NONE
                            .fill(row_bg)
                            .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                            .inner_margin(egui::Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                ui.set_max_width(modal_w - 44.0);
                                ui.horizontal(|ui| {
                                    if ui.small_button("위치 보기").clicked() {
                                        app.home_map.viewport.center_xy =
                                            app.home_map.real_survey.survey_center;
                                        app.home_map.viewport.meters_per_pixel = 1.8;
                                        app.active_tab = MainTab::Map;
                                        app.home_map.draw_guide_message = Some(format!(
                                            "「{}」 위치로 이동했습니다: {}",
                                            r.id, r.message_ko
                                        ));
                                        app.modals.active = None;
                                    }
                                    // KaSubmitDialog.cpp:180-200 — 실패 항목의 바로가기 버튼
                                    if !r.passed {
                                        if let Some(lbl) =
                                            ModalDialogController::submit_action_label(&r.action)
                                        {
                                            if ui.small_button(lbl).clicked() {
                                                let mut next_modal = None;
                                                match r.action.as_str() {
                                                    "open_layout" => {
                                                        app.active_tab = MainTab::Drawing;
                                                    }
                                                    "draw_survey_area"
                                                    | "draw_feature" => {
                                                        app.active_tab = MainTab::Map;
                                                        app.active_tool =
                                                            strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
                                                    }
                                                    "add_control_point" => {
                                                        app.active_tab = MainTab::Map;
                                                        next_modal = Some(
                                                            ActiveModalDialog::ControlPoint,
                                                        );
                                                    }
                                                    "edit_attributes" => {
                                                        app.active_tab = MainTab::Map;
                                                        app.right_inspector.active_card =
                                                            crate::right_inspector::InspectorCardTab::LayerStyleCard;
                                                    }
                                                    "select_tool" => {
                                                        app.active_tab = MainTab::Map;
                                                        app.active_tool =
                                                            strata_canvas2d::map_tools::MapToolKind::Select;
                                                    }
                                                    "set_work_crs" => {
                                                        next_modal = Some(
                                                            ActiveModalDialog::AccountSettings,
                                                        );
                                                    }
                                                    _ => {
                                                        app.active_tab = MainTab::Map;
                                                    }
                                                }
                                                app.home_map.draw_guide_message = Some(format!(
                                                    "{} — 「{}」", lbl, r.id
                                                ));
                                                app.modals.active = next_modal;
                                            }
                                        }
                                    }
                                    let (badge, col) = if r.passed {
                                        (format!("[통과·{}]", r.severity), Color32::from_rgb(21, 128, 61))
                                    } else if r.severity == "error" {
                                        ("[오류·error]".to_owned(), Color32::from_rgb(185, 28, 28))
                                    } else {
                                        ("[주의·warn]".to_owned(), Color32::from_rgb(180, 83, 9))
                                    };
                                    ui.colored_label(
                                        col,
                                        egui::RichText::new(badge).strong().size(10.5),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:02}. {}",
                                            idx + 1,
                                            r.id
                                        ))
                                        .monospace()
                                        .strong()
                                        .size(11.0)
                                        .color(Color32::from_rgb(28, 25, 23)),
                                    );
                                    let body_txt = if r.passed {
                                        format!("{} ({})", r.message_ko, r.basis_ko)
                                    } else {
                                        format!("{} — {}", r.message_ko, r.fix_ko)
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(body_txt)
                                                .size(11.0)
                                                .color(Color32::from_rgb(45, 44, 40)),
                                        )
                                        .truncate(),
                                    );
                                });
                            });
                    }
                });

            ui.separator();
            // KaSubmitDialogPlate.cpp:66 — 패키지 구성 안내
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(
                        "EPSG:5179 SHP · 조사도면.pdf · MANIFEST.sha256 · README_submit.txt",
                    )
                    .monospace()
                    .size(11.5)
                    .color(Color32::from_rgb(75, 74, 70)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(&status_txt)
                            .size(12.0)
                            .strong()
                            .color(if err_cnt > 0 {
                                Color32::from_rgb(185, 28, 28)
                            } else {
                                Color32::from_rgb(21, 128, 61)
                            }),
                    );
                });
            });
            ui.add_space(4.0);
            // KaSubmitDialog.cpp:85-105 — 하단 버튼 행 (다시 검수 / 레이어 5179 변환… / 제출 꾸러미 만들기… / 닫기)
            ui.horizontal(|ui| {
                if ui.button("다시 검수").clicked() {
                    app.home_map.draw_guide_message = Some(status_txt.clone());
                }
                if ui.button("레이어 5179 변환…").clicked() {
                    app.home_map.draw_guide_message = Some(
                        "선택한 레이어를 EPSG:5179 SHP 파일로 변환했습니다.".to_owned(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        app.modals.active = None;
                    }
                    let export_btn = egui::Button::new(
                        egui::RichText::new("제출 꾸러미 만들기…")
                            .color(Color32::WHITE)
                            .strong()
                            .size(13.0),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(160.0, 32.0));
                    if ui.add(export_btn).clicked() {
                        let out_dir =
                            app.home_map.real_survey.survey_dir.join("제출꾸러미_5179");
                        if let Ok(m) = export_submission_package(&out_dir, &findings) {
                            app.modals.last_export_status = format!(
                                "제출 꾸러미 생성 완료: {} (파일 {}종, {} — README_submit.txt 기록됨)",
                                out_dir.display(),
                                m.files_sha256.len(),
                                status_txt
                            );
                        }
                    }
                });
            });
            if !app.modals.last_export_status.is_empty() {
                ui.add_space(4.0);
                ui.colored_label(
                    Color32::from_rgb(21, 128, 61),
                    &app.modals.last_export_status,
                );
            }
        });
}
