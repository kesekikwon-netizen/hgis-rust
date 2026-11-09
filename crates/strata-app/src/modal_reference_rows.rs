//! `render_reference_status` 의 7종 참조 자료 표 행 렌더링.
//!
//! `modal_reference.rs` 가 300줄을 넘어 한 함수의 한 구간을 옮겼다.
//! 실행 순서는 호출부가 보장한다.
use crate::dialogs_modal::render_strata_modal_header;
use crate::dialogs_modal::ActiveModalDialog;
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2, Stroke};

pub(crate) fn paint_reference_status_rows(
    app: &mut StrataAppState,
    ui: &mut egui::Ui,
    open: &mut bool,
    rows: &[crate::modal_reference::ReferenceRow],
) {
        // 7개 자료 행 렌더링
        let mut next_modal: Option<ActiveModalDialog> = None;
        egui::ScrollArea::vertical()
            .max_height(245.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (
                    idx,
                    (
                        name,
                        kind_lbl,
                        extent_txt,
                        received_txt,
                        size_txt,
                        offline_txt,
                        is_offline,
                        dl_modal,
                        cfg_modal,
                    ),
                ) in rows.into_iter().enumerate()
                {
                    let bg = if idx % 2 == 0 {
                        Color32::from_rgb(252, 251, 247)
                    } else {
                        Color32::WHITE
                    };
                    egui::Frame::NONE
                        .fill(bg)
                        .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            egui::Grid::new(("ka_ref_status_row", idx))
                                .num_columns(7)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.add_sized(
                                        [176.0, 22.0],
                                        egui::Label::new(
                                            egui::RichText::new(name)
                                                .size(11.5)
                                                .strong(),
                                        )
                                        .truncate(),
                                    );
                                    ui.add_sized(
                                        [68.0, 22.0],
                                        egui::Label::new(
                                            egui::RichText::new(kind_lbl).size(11.5),
                                        ),
                                    );
                                    ui.add_sized(
                                        [96.0, 22.0],
                                        egui::Label::new(
                                            egui::RichText::new(extent_txt).size(11.0),
                                        ),
                                    );
                                    ui.add_sized(
                                        [96.0, 22.0],
                                        egui::Label::new(
                                            egui::RichText::new(received_txt)
                                                .monospace()
                                                .size(11.0),
                                        ),
                                    );
                                    // KaReferenceStatusDialog.cpp:106-109 — 크기 열 우측 정렬
                                    ui.with_layout(
                                        egui::Layout::right_to_left(
                                            egui::Align::Center,
                                        ),
                                        |ui| {
                                            ui.add_sized(
                                                [56.0, 22.0],
                                                egui::Label::new(
                                                    egui::RichText::new(size_txt)
                                                        .monospace()
                                                        .size(11.5),
                                                ),
                                            );
                                        },
                                    );
                                    // KaReferenceStatusDialog.cpp:110 — Offline 아니면 warnMark(#7A5A00) 표시
                                    let (status_prefix, status_col) = if is_offline {
                                        ("● ", Color32::from_rgb(21, 128, 61))
                                    } else {
                                        ("▲ ", Color32::from_rgb(0x7A, 0x5A, 0x00))
                                    };
                                    ui.add_sized(
                                        [150.0, 22.0],
                                        egui::Label::new(
                                            egui::RichText::new(format!(
                                                "{status_prefix}{offline_txt}"
                                            ))
                                            .size(11.0)
                                            .strong()
                                            .color(status_col),
                                        )
                                        .truncate(),
                                    );
                                    ui.horizontal(|ui| {
                                        if ui.small_button("받기").clicked() {
                                            next_modal = Some(dl_modal);
                                        }
                                        if ui.small_button("설정").clicked() {
                                            next_modal = Some(cfg_modal);
                                        }
                                    });
                                    ui.end_row();
                                });
                        });
                }
            });

        if let Some(target_modal) = next_modal {
            app.modals.active = Some(target_modal);
        }

        ui.add_space(6.0);
}
