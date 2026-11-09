//! 「수치지형도 좌표계 확인」 스캔 결과 표 — 추가/레이어·파일·좌표계·범위 네 칸.
//!
//! `dlg_topographic_import.rs` 가 300줄 규칙에 맞춰 나눈 파일이다
//! (`KaTopographicImportDialog.cpp:460-492` populate). 원래 표 본문을 **그대로** 옮겼고,
//! 열 순서·폭·문구는 하나도 바꾸지 않았다.

use eframe::egui::{self, Color32, Stroke};

use crate::app_state::StrataAppState;

/// 스캔 결과 표. `rows` 는 창 밖에서 한 번만 뽑아 둔 표 행들이다.
pub(crate) fn render_topo_import_table(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    rows: &[crate::modal_types::TopoTableRow],
) {
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
}
