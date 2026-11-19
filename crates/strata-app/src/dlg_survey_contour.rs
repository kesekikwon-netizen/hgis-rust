//! `dialogs_download_print` 에서 옮긴 「SurveyContour」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

/// Order W-29 is the TIN, the GeoPackage and the reference layers.
const DEFERRED_W29: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 W-29)";

/// `KaSurveyContourDialog.cpp:142-145`. Tests never open a native dialog.
fn pick_contour_file() -> Option<std::path::PathBuf> {
    if cfg!(test) {
        return None;
    }
    rfd::FileDialog::new()
        .set_title("측량 좌표")
        .add_filter("측량 좌표", &["dxf", "xlsx", "xls", "csv"])
        .pick_file()
}

pub(crate) fn render_survey_contour(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    open: &mut bool,
) {
    egui::Window::new("ka_modal_survey_contour")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([
            440.0_f32.min(ctx.screen_rect().width() - 32.0),
            290.0_f32.min(ctx.screen_rect().height() - 32.0),
        ])
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "측량 등고선 - Strata", open);
            if !*open {
                app.modals.contour_empty_rejected = false;
            }
            ui.label(
                egui::RichText::new(
                    "엑셀·CSV·DXF의 측량 좌표(X, Y, Z)로 등고선과 표고점 레이어를 만듭니다.",
                )
                .size(12.0)
                .color(Color32::from_rgb(80, 78, 72)),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("좌표 파일"));
                if ui
                    .add_sized(
                        [264.0, 26.0],
                        egui::TextEdit::singleline(&mut app.modals.contour_file_path),
                    )
                    .changed()
                {
                    app.modals.contour_empty_rejected = false;
                }
                // KaSurveyContourDialog.cpp:142-145 — 「측량 좌표」, filter dxf/xlsx/xls/csv.
                if ui
                    .add_sized([74.0, 26.0], egui::Button::new("찾기…"))
                    .clicked()
                {
                    if let Some(picked) = pick_contour_file() {
                        app.modals.contour_file_path = picked.display().to_string();
                        app.modals.contour_empty_rejected = false;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("등고선 간격"));
                ui.add(
                    egui::DragValue::new(&mut app.modals.contour_interval_m)
                        .speed(0.1)
                        .range(0.1..=100.0)
                        .suffix(" m"),
                );
                ui.label(
                    egui::RichText::new("(계곡선 5줄마다 굵게 표시)")
                        .size(11.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                );
            });
            ui.add_space(4.0);
            // KaSurveyContourDialog.cpp:31-55 축 순서 / 이상한 점 + 고급 설정
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("축 순서"));
                ui.checkbox(
                    &mut app.modals.contour_swap_axes,
                    "X를 북쪽, Y를 동쪽으로 읽기",
                );
            });
            ui.horizontal(|ui| {
                ui.add_sized([74.0, 26.0], egui::Label::new("이상한 점"));
                ui.checkbox(&mut app.modals.contour_exclude_suspicious, "의심점 빼기");
            });
            let adv_label = if app.modals.contour_advanced_open {
                "▾ 고급 설정"
            } else {
                "▸ 고급 설정"
            };
            if ui
                .button(egui::RichText::new(adv_label).size(12.5))
                .clicked()
            {
                app.modals.contour_advanced_open = !app.modals.contour_advanced_open;
            }
            if app.modals.contour_advanced_open {
                const CRS_LABELS: [&str; 5] = [
                    "작업 좌표계와 같음 (EPSG:5187)",
                    "EPSG:5185 서부원점",
                    "EPSG:5186 중부원점",
                    "EPSG:5179 UTM-K",
                    "EPSG:4326 경위도 WGS84",
                ];
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("파일 좌표계"));
                    let idx = app.modals.contour_file_crs_idx.min(4);
                    if ui
                        .add_sized(
                            [260.0, 26.0],
                            egui::Button::new(format!("{}  ▼", CRS_LABELS[idx])),
                        )
                        .clicked()
                    {
                        app.modals.contour_file_crs_idx = (idx + 1) % 5;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("최대 삼각형 변"));
                    ui.add(
                        egui::DragValue::new(&mut app.modals.contour_max_edge_m)
                            .range(0.0..=1000.0)
                            .speed(1.0)
                            .suffix(" m"),
                    );
                    if app.modals.contour_max_edge_m == 0.0 {
                        ui.label(
                            egui::RichText::new("쓰지 않음")
                                .size(11.5)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("단절선"));
                    ui.checkbox(
                        &mut app.modals.contour_use_breaklines,
                        "높이가 있는 DXF 선을 단절선으로 쓰기",
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_sized([86.0, 26.0], egui::Label::new("계산 격자"));
                    ui.add(
                        egui::DragValue::new(&mut app.modals.contour_cell_size_m)
                            .range(0.0..=5.0)
                            .speed(0.05)
                            .suffix(" m"),
                    );
                    if app.modals.contour_cell_size_m == 0.0 {
                        ui.label(
                            egui::RichText::new("자동")
                                .size(11.5)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                });
            }
            // KaSurveyContourDialog.cpp:105,126-131 — column note until a file is chosen, and
            // 「좌표 파일을 고르세요.」 when accept is clicked with an empty path.
            // A chosen file uses showPreview (`:211-230`). Making the lines is order W-29.
            ui.add_space(6.0);
            let path = app.modals.contour_file_path.trim();
            let path_empty = path.is_empty();
            let summary = if path_empty && app.modals.contour_empty_rejected {
                "좌표 파일을 고르세요.".to_owned()
            } else if path_empty {
                "DXF·엑셀의 점번호, X(북), Y(동), 표고는 자동으로 읽습니다.".to_owned()
            } else {
                survey_contour_sheet::contour_preview(
                    path,
                    app.modals.contour_interval_m,
                    app.modals.contour_exclude_suspicious,
                )
            };
            egui::Frame::NONE
                .fill(Color32::from_rgb(225, 240, 218))
                .stroke(Stroke::new(1.0, Color32::from_rgb(155, 195, 145)))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(summary)
                            .size(12.0)
                            .color(Color32::from_rgb(28, 72, 32)),
                    );
                });
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([68.0, 30.0], egui::Button::new("취소"))
                    .clicked()
                {
                    app.modals.contour_empty_rejected = false;
                    app.modals.active = None;
                }
                let make_btn = egui::Button::new(
                    egui::RichText::new("등고선 만들기")
                        .color(Color32::WHITE)
                        .strong(),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(108.0, 30.0));
                // W-29 still owns the TIN, contours.gpkg and the reference-map group.
                // Until then the button only rejects an empty path (KaSurveyContourDialog.cpp:126-131).
                let make = ui.add_enabled(path_empty, make_btn);
                if path_empty {
                    if make.clicked() {
                        app.modals.contour_empty_rejected = true;
                    }
                } else {
                    make.on_disabled_hover_text(DEFERRED_W29);
                }
            });
        });
}

#[path = "survey_contour_sheet.rs"]
mod survey_contour_sheet;

#[cfg(test)]
#[path = "dlg_survey_contour_tests.rs"]
mod tests;
