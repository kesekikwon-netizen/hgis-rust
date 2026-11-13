//! `dialogs_download_print` 에서 옮긴 「TopographicImport」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.
//! 300줄 규칙에 맞춰 스캔 결과 표(`dlg_topo_import_table`), 정합 미리보기
//! (`dlg_topo_import_preview`), 하단 단추·진행률(`dlg_topo_import_actions`)로 나눴다.
//! `crate::dlg_topographic_import::render_topographic_import` 경로와 위젯 순서는 그대로다.

use eframe::egui::{self, Color32, Pos2, Stroke};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

#[path = "dlg_topo_import_actions.rs"]
mod dlg_topo_import_actions;
#[path = "dlg_topo_import_preview.rs"]
mod dlg_topo_import_preview;
#[path = "dlg_topo_import_table.rs"]
mod dlg_topo_import_table;

use dlg_topo_import_actions::render_topo_import_actions;
use dlg_topo_import_preview::render_topo_import_preview;
use dlg_topo_import_table::render_topo_import_table;

pub(crate) fn render_topographic_import(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
) {
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
                    // MainWindowTopographic.cpp:103-106 — start folder derived from the survey path.
                    if let Some(start) = app.topographic_picker_start().filter(|s| s.is_dir()) {
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

            render_topo_import_table(app, ui, &rows);

            let (sel_crs_idx, sel_crs_lbl) =
                render_topo_import_preview(app, ui, modal_w, &rows, sel_ri, row_count);

            render_topo_import_actions(app, ui, modal_w, &rows, sel_crs_idx, &sel_crs_lbl);
        });
}
