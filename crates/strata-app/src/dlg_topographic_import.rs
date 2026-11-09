//! `dialogs_download_print` 에서 옮긴 「TopographicImport」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;
use crate::dialogs_modal::TOPO_CRS_UNKNOWN;

pub(crate) fn render_topographic_import(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // KaTopographicImportDialog.cpp:287-497 — "수치지형도 — 좌표계 확인과 지도 추가"
    // 받은 폴더는 C++ 처럼 실제로 훑는다(MainWindowTopographic.cpp:131-148
    // → TopographicCatalog::scan → populate). 하드코딩 목록은 없다.
    // 창 크기를 정할 때 이미 DPI를 반영한 논리 픽셀을 쓴다. 여기서 다시 나누면
    // 150% 화면에서 대화상자가 2/3 크기로 줄어 글자가 넘친다.
    let full = ctx.screen_rect();
    let (vis_w, vis_h) = (full.width(), full.height());
    let modal_w = 960.0_f32.min(vis_w - 32.0);
    let modal_h = 620.0_f32.min(vis_h - 32.0);
    let modal_pos = Pos2::new(
        ((vis_w - modal_w) * 0.5).max(16.0),
        ((vis_h - modal_h) * 0.5).max(16.0),
    );
    // 그리는 동안 app.modals 를 고쳐야 하므로 표에 쓸 값만 미리 꺼낸다.
    let rows = app.modals.topo_table_rows();
    let row_count = rows.len();
    let sel_ri = if row_count == 0 {
        0
    } else {
        app.modals.topo_selected_row.min(row_count - 1)
    };
    egui::Window::new("ka_modal_topographic_import")
        .title_bar(false)
        .resizable(false)
        .fixed_pos(modal_pos)
        .fixed_size([modal_w, modal_h])
        .show(ctx, |ui| {
            ui.set_min_width(modal_w - 24.0);
            ui.set_max_width(modal_w - 24.0);
            render_strata_modal_header(
                ui,
                "수치지형도 — 좌표계 확인과 지도 추가",
                open,
            );

            // KaTopographicImportDialog.cpp:292-294 — 상단 안내 문구
            ui.label(
                egui::RichText::new(
                    "도엽의 좌표계를 확인한 뒤 추가하세요. 좌표 숫자만으로 구·신 측지계나 원점대를 확정할 수 없습니다.\n미리보기에서 도로·건물·하천이 기존 지도와 맞는지 확인하세요. 원본 파일은 변경하지 않습니다.",
                )
                .size(11.5)
                .color(Color32::from_rgb(0x37, 0x41, 0x51)),
            );
            ui.add_space(4.0);

            // MainWindowTopographic.cpp:122-129 importTopographicFolder — 폴더를 고른다.
            // 대화상자 제목은 C++ 원문 그대로다.
            ui.horizontal(|ui| {
                if ui.button("받은 폴더 고르기…").clicked() {
                    let mut picker = rfd::FileDialog::new()
                        .set_title("수치지형도 폴더 선택 — 하위 폴더도 확인합니다");
                    let survey_dir = app.home_map.real_survey.survey_dir.clone();
                    // :124-126 — 조사 폴더가 있으면 그 안의 「지형도」에서 시작한다.
                    let start = if survey_dir.as_os_str().is_empty() {
                        std::env::current_dir().unwrap_or_default()
                    } else {
                        survey_dir.join("지형도")
                    };
                    if start.is_dir() {
                        picker = picker.set_directory(start);
                    }
                    if let Some(folder) = picker.pick_folder() {
                        app.modals.rescan_topo_folder(&folder);
                    }
                }
                let folder_label = if app.modals.topo_folder.is_empty() {
                    "아직 받은 폴더를 고르지 않았습니다.".to_owned()
                } else {
                    app.modals.topo_folder.clone()
                };
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(folder_label)
                            .size(10.5)
                            .monospace()
                            .color(Color32::from_rgb(96, 94, 88)),
                    )
                    .truncate(),
                );
            });
            ui.add_space(4.0);

            // KaTopographicImportDialog.cpp:465-472 — 판별이 애매한 항목은
            // [검토 필요]로 표시하고 자동으로 체크하지 않는다.
            let review_count = rows
                .iter()
                .filter(|row| row.review_reason.is_some())
                .count();
            if review_count > 0 {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(255, 251, 235))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(180, 83, 9)))
                    .corner_radius(5.0)
                    .inner_margin(egui::Margin::symmetric(10, 5))
                    .show(ui, |ui| {
                        ui.colored_label(
                            Color32::from_rgb(180, 83, 9),
                            egui::RichText::new(format!(
                                "▲ 검토 필요 {review_count}개: 원본 좌표계나 도형 범위를 확인하지 못해 자동으로 지도에 추가하지 않습니다. 원본을 확인한 뒤 직접 체크하세요."
                            ))
                            .size(11.5)
                            .strong(),
                        );
                    });
                ui.add_space(4.0);
            }

            // 표 헤더 (KaTopographicImportDialog.cpp:297-298)
            egui::Frame::NONE
                .fill(Color32::from_rgb(232, 229, 220))
                .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    egui::Grid::new("ka_topo_import_hdr")
                        .num_columns(4)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.set_min_width(modal_w - 44.0);
                            for (col_lbl, w) in [
                                ("추가 / 레이어", 250.0),
                                ("파일", 175.0),
                                ("확인할 원본 좌표계", 235.0),
                                ("좌표 범위", 215.0),
                            ] {
                                ui.add_sized(
                                    [w, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(col_lbl).size(11.5).strong(),
                                    ),
                                );
                            }
                            ui.end_row();
                        });
                });

            // 스캔 결과 표 (KaTopographicImportDialog.cpp:460-492 populate)
            egui::ScrollArea::vertical()
                .max_height(190.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if rows.is_empty() {
                        ui.add_space(6.0);
                        let empty_text = if app.modals.topo_folder.is_empty() {
                            "받은 수치지형도 폴더를 고르면 그 안의 .shp · .ngi · .dxf · .gpkg 를 확인합니다."
                        } else {
                            "받은 폴더에서 .shp · .ngi · .dxf · .gpkg 파일을 찾지 못했습니다. 하위 폴더까지 확인했으니 다른 폴더를 고르세요."
                        };
                        ui.label(
                            egui::RichText::new(empty_text)
                                .size(11.5)
                                .color(Color32::from_rgb(120, 53, 15)),
                        );
                    }
                    for (ri, row) in rows.iter().enumerate() {
                        let already = app
                            .modals
                            .topo_row_already_loaded
                            .get(ri)
                            .copied()
                            .unwrap_or(false);
                        let is_selected = app.modals.topo_selected_row == ri;
                        let row_bg = if is_selected {
                            Color32::from_rgb(225, 240, 218)
                        } else if row.review_reason.is_some() && !already {
                            Color32::from_rgb(255, 251, 235)
                        } else if ri % 2 == 0 {
                            Color32::from_rgb(252, 251, 247)
                        } else {
                            Color32::WHITE
                        };
                        egui::Frame::NONE
                            .fill(row_bg)
                            .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                egui::Grid::new(("ka_topo_import_row", ri))
                                    .num_columns(4)
                                    .spacing([10.0, 3.0])
                                    .show(ui, |ui| {
                                        // 1) 추가 / 레이어 + [이미 있음] / [검토 필요]
                                        ui.horizontal(|ui| {
                                            ui.set_min_width(250.0);
                                            let mut checked = app
                                                .modals
                                                .topo_row_checks
                                                .get(ri)
                                                .copied()
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut checked, "").changed() {
                                                if let Some(slot) =
                                                    app.modals.topo_row_checks.get_mut(ri)
                                                {
                                                    *slot = checked;
                                                }
                                            }
                                            let tag = if already {
                                                format!("{} [이미 있음]", row.label)
                                            } else if row.review_reason.is_some() {
                                                format!("{} [검토 필요]", row.label)
                                            } else {
                                                row.label.clone()
                                            };
                                            let tag_col = if already {
                                                Color32::from_rgb(21, 128, 61)
                                            } else if row.review_reason.is_some() {
                                                Color32::from_rgb(180, 83, 9)
                                            } else {
                                                Color32::from_rgb(25, 25, 24)
                                            };
                                            if ui
                                                .selectable_label(
                                                    is_selected,
                                                    egui::RichText::new(tag)
                                                        .size(11.5)
                                                        .strong()
                                                        .color(tag_col),
                                                )
                                                .clicked()
                                            {
                                                app.modals.topo_selected_row = ri;
                                                if let Some(reason) =
                                                    row.review_reason.clone()
                                                {
                                                    // KaTopographicImportDialog.cpp:323-325
                                                    app.modals.topo_status_text = format!(
                                                        "검토 필요: {reason}. 자동으로 지도에 추가하지 않습니다."
                                                    );
                                                }
                                            }
                                        });

                                        // 2) 파일 이름 (KaTopographicImportDialog.cpp:473-474)
                                        ui.add_sized(
                                            [175.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(&row.file_name)
                                                    .monospace()
                                                    .size(11.0),
                                            ),
                                        );

                                        // 3) 확인할 원본 좌표계 콤보 (KaTopographicImportDialog.cpp:475-487)
                                        let crs_idx = app
                                            .modals
                                            .topo_row_crs_idx
                                            .get(ri)
                                            .copied()
                                            .unwrap_or(0);
                                        let mut choice = crs_idx;
                                        egui::ComboBox::from_id_salt((
                                            "ka_topo_crs_combo",
                                            ri,
                                        ))
                                        .selected_text(row.choice_label(crs_idx))
                                        .width(230.0)
                                        .show_ui(ui, |ui| {
                                            for ci in 0..row.choices.len() {
                                                ui.selectable_value(
                                                    &mut choice,
                                                    ci,
                                                    row.choice_label(ci),
                                                );
                                            }
                                        });
                                        if choice != crs_idx {
                                            if let Some(slot) =
                                                app.modals.topo_row_crs_idx.get_mut(ri)
                                            {
                                                *slot = choice;
                                            }
                                            app.modals.topo_selected_row = ri;
                                        }

                                        // 4) 좌표 범위 (KaTopographicImportDialog.cpp:488-491)
                                        ui.add_sized(
                                            [215.0, 22.0],
                                            egui::Label::new(
                                                egui::RichText::new(&row.extent_label)
                                                    .monospace()
                                                    .size(10.5),
                                            )
                                            .truncate(),
                                        );
                                        ui.end_row();
                                    });
                            });
                    }
                });

            ui.add_space(6.0);
            // 정합 미리보기 캔버스 (m_preview, KaTopographicImportDialog.cpp:304-306, 498-522)
            let sel_crs_idx = app
                .modals
                .topo_row_crs_idx
                .get(sel_ri)
                .copied()
                .unwrap_or(0);
            let sel_crs_lbl = rows
                .get(sel_ri)
                .map_or_else(|| TOPO_CRS_UNKNOWN.to_owned(), |row| row.choice_label(sel_crs_idx));
            let aligned = rows
                .get(sel_ri)
                .is_some_and(|row| row.choice_is_work_crs_family(sel_crs_idx));
            let (prev_r, _) = ui.allocate_exact_size(
                Vec2::new(modal_w - 28.0, 125.0),
                egui::Sense::hover(),
            );
            let pp = ui.painter_at(prev_r);
            pp.rect_filled(prev_r, 5.0, Color32::from_rgb(248, 247, 243));
            pp.rect_stroke(
                prev_r,
                5.0,
                Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                egui::StrokeKind::Inside,
            );
            // 기준 지적·조사구역 (정위치)
            let pc = prev_r.center();
            for gx in -4..=4 {
                let x = pc.x + (gx as f32) * 85.0;
                pp.line_segment(
                    [
                        Pos2::new(x, prev_r.top() + 6.0),
                        Pos2::new(x, prev_r.bottom() - 6.0),
                    ],
                    Stroke::new(0.6, Color32::from_rgb(218, 215, 206)),
                );
            }
            let survey_box = Rect::from_center_size(pc, Vec2::new(120.0, 74.0));
            pp.rect_filled(
                survey_box,
                2.0,
                Color32::from_rgba_unmultiplied(234, 88, 12, 55),
            );
            pp.rect_stroke(
                survey_box,
                2.0,
                Stroke::new(2.0, Color32::from_rgb(234, 88, 12)),
                egui::StrokeKind::Middle,
            );
            // C++ `previewCurrent(:498-522)` 는 실제 도엽을 좌표변환해 겹쳐 그린다.
            // 여기서는 WGSL/지도 캔버스 없이 같은 판단(원점대가 작업 좌표계 계열인가)만 보인다.
            let shift_x: f32 = if aligned { 0.0 } else { 90.0 };
            let topo_col = if aligned {
                Color32::from_rgb(21, 128, 61)
            } else {
                Color32::from_rgb(185, 28, 28)
            };
            for dy in [-26.0_f32, 0.0, 24.0] {
                pp.line_segment(
                    [
                        Pos2::new(pc.x - 160.0 + shift_x, pc.y + dy - 12.0),
                        Pos2::new(pc.x + 160.0 + shift_x, pc.y + dy + 14.0),
                    ],
                    Stroke::new(1.8, topo_col),
                );
            }
            pp.text(
                Pos2::new(prev_r.left() + 10.0, prev_r.top() + 14.0),
                egui::Align2::LEFT_CENTER,
                format!(
                    "정합 미리보기 — 선택 행 #{}: {} ({})",
                    if row_count == 0 { 0 } else { sel_ri + 1 },
                    sel_crs_lbl,
                    if aligned {
                        "● 기존 조사구역·지적도와 정합 일치"
                    } else {
                        "▲ 좌표계 원점 불일치로 도엽 위치가 어긋남 — 원본 좌표계를 확인하세요"
                    }
                ),
                egui::FontId::proportional(11.5),
                topo_col,
            );

            ui.add_space(6.0);
            // 하단 컨트롤 행 (KaTopographicImportDialog.cpp:307-318)
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut app.modals.topo_maps_enabled, "수치지형도 표시")
                    .changed()
                {
                    app.home_map.real_survey.layer_visible.insert(
                        "수치지형도 1:5,000 (합침)".to_owned(),
                        app.modals.topo_maps_enabled,
                    );
                }
                if ui.button("선택 도엽 정합 미리보기").clicked() {
                    if rows.is_empty() {
                        app.modals.topo_status_text =
                            "먼저 받은 폴더를 골라 자료를 확인하세요.".to_owned();
                    } else if sel_crs_idx == 0 {
                        app.modals.topo_status_text =
                            "먼저 미리 볼 원본 좌표계를 선택하세요.".to_owned();
                    } else {
                        app.modals.topo_status_text = format!(
                            "선택 좌표계({sel_crs_lbl})의 미리보기입니다. 위성·지적과 정합을 확인한 뒤 추가하세요."
                        );
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("닫기").clicked() {
                        app.modals.active = None;
                    }
                    if ui.button("색인 취소").clicked() {
                        app.modals.cancel_topo_scan();
                        app.modals.active = None;
                    }
                    // KaTopographicImportDialog.cpp:493 — 올릴 항목이 없으면 누를 수 없다.
                    let add_btn = egui::Button::new(
                        egui::RichText::new("좌표계 확인 완료 · 지도에 추가")
                            .color(Color32::WHITE)
                            .strong()
                            .size(13.0),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(210.0, 32.0));
                    if ui.add_enabled(!rows.is_empty(), add_btn).clicked() {
                        // KaTopographicImportDialog.cpp:524-537 — 범위·좌표계가 확인된
                        // 항목만 올린다. 하나라도 걸리면 아무것도 올리지 않고 문구로 끝낸다.
                        if let Some(reason) = app.modals.topo_blocking_message() {
                            app.modals.topo_status_text = reason.to_owned();
                        } else {
                            let picked = app.modals.topo_checked_rows();
                            for row in &picked {
                                if let Some(slot) =
                                    app.modals.topo_row_already_loaded.get_mut(*row)
                                {
                                    *slot = true;
                                }
                            }
                            app.modals.topo_imported_count = picked.len();
                            if picked.is_empty() {
                                // C++ `:524-527` — 체크가 없으면 올릴 것이 없다.
                                app.modals.topo_status_text =
                                    "추가할 항목에 체크하세요.".to_owned();
                            } else {
                                app.modals.topo_maps_enabled = true;
                                app.home_map.real_survey.layer_visible.insert(
                                    "수치지형도 1:5,000 (합침)".to_owned(),
                                    true,
                                );
                                app.modals.topo_status_text = format!(
                                    "수치지형도 레이어 {}개를 작업 좌표계(EPSG:5187)로 정합 변환하여 지도에 추가했습니다 ([이미 있음] 표시).",
                                    picked.len()
                                );
                                app.home_map.draw_guide_message =
                                    Some(app.modals.topo_status_text.clone());
                            }
                        }
                    }
                });
            });

            ui.add_space(4.0);
            // 진행률 바 + 상태 문구 (KaTopographicImportDialog.cpp:319-321, 494-495)
            let (pbar_r, _) = ui
                .allocate_exact_size(Vec2::new(modal_w - 28.0, 14.0), egui::Sense::hover());
            let bp = ui.painter_at(pbar_r);
            bp.rect_filled(pbar_r, 3.0, Color32::from_rgb(232, 229, 220));
            bp.rect_filled(
                Rect::from_min_size(
                    pbar_r.min,
                    Vec2::new(
                        pbar_r.width() * app.modals.topo_progress_pct.clamp(0.0, 1.0),
                        pbar_r.height(),
                    ),
                ),
                3.0,
                Color32::from_rgb(44, 43, 41),
            );
            ui.label(
                egui::RichText::new(&app.modals.topo_status_text)
                    .size(11.0)
                    .color(Color32::from_rgb(55, 54, 50)),
            );
        });
}
