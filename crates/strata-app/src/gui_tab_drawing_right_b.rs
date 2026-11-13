//! 도면 탭 오른쪽 패널(축척·용지·도면 정보) 뒷부분.

use eframe::egui::{self, Color32, Stroke, Vec2};


use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

/// Tooltip of a drawing feature not yet ported (rules_v1 deferred-control text).
const DEFERRED_R3: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 R3)";

impl StrataAppState {
    /// 축척 프리셋·용지 크기·도면 정보 칸.
    pub(crate) fn paint_drawing_right_items_b(&mut self, ui: &mut egui::Ui) {
                    egui::Grid::new("ka_drawing_scale_presets")
                        .num_columns(7)
                        .spacing([3.0, 3.0])
                        .show(ui, |ui| {
                            for s in ["20", "30", "50", "100", "200", "250", "300"] {
                                if ui
                                    .add_sized(
                                        [31.0, 20.0],
                                        egui::Button::new(egui::RichText::new(s).size(10.0)),
                                    )
                                    .clicked()
                                {
                                    if let Ok(v) = s.parse::<u32>() {
                                        self.drawing.sheet.scale_denominator = v;
                                    }
                                }
                            }
                            ui.end_row();
                            for s in ["400", "500", "1000", "2000", "5000", "10000", "25000"] {
                                if ui
                                    .add_sized(
                                        [31.0, 20.0],
                                        egui::Button::new(egui::RichText::new(s).size(10.0)),
                                    )
                                    .clicked()
                                {
                                    if let Ok(v) = s.parse::<u32>() {
                                        self.drawing.sheet.scale_denominator = v;
                                    }
                                }
                            }
                            ui.end_row();
                        });
                    ui.add_space(6.0);
                    ui.columns(3, |cols| {
                        for (i, style) in strata_layout::sheet_composer::ScaleBarStyle::ALL
                            .into_iter()
                            .enumerate()
                        {
                            let sel = self.drawing.sheet.scale_bar_style == style;
                            let fill = if sel {
                                Color32::from_rgb(228, 223, 210)
                            } else {
                                Color32::WHITE
                            };
                            let stroke = if sel {
                                Stroke::new(1.4, Color32::from_rgb(90, 88, 82))
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(195, 192, 182))
                            };
                            let b = egui::Button::new(
                                egui::RichText::new(format!(
                                    "{}\n{}",
                                    style.symbol_glyph(),
                                    style.label_ko()
                                ))
                                .size(11.0),
                            )
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(5.0)
                            .min_size(Vec2::new(cols[i].available_width(), 42.0));
                            if cols[i].add(b).clicked() {
                                self.drawing.sheet.scale_bar_style = style;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("선택 항목 크기").size(12.0));
                        // KaDrawingStudio.cpp:1137-1146 — QSpinBox 25..=200 %
                        ui.add(
                            egui::DragValue::new(&mut self.drawing.decoration_size_percent)
                                .range(25..=200)
                                .speed(1.0)
                                .suffix(" %"),
                        )
                        .on_hover_text(
                            "용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다.",
                        );
                    });
                    ui.add_space(4.0);
                    ui.columns(3, |cols| {
                        for (i, lbl) in ["축척 글자", "좌표계", "좌표 격자"].into_iter().enumerate()
                        {
                            if cols[i]
                                .add_sized(
                                    [cols[i].available_width(), 24.0],
                                    egui::Button::new(egui::RichText::new(lbl).size(11.0)),
                                )
                                .clicked()
                                && i == 2
                            {
                                let next_follow = !self.drawing.sheet.grid_follow_map;
                                if next_follow {
                                    self.drawing.sheet.apply_map_coordinate_grid(
                                        true,
                                        0.0,
                                        20.0,
                                        // 지도 좌표격자의 원점 — C++ 기본은 좌표계 원점(0, 0)(KaGridOriginButton.cpp:36).
                                        [0.0, 0.0],
                                    );
                                    self.home_map.draw_guide_message = Some(
                                        "도면 격자를 지도 좌표격자와 같게 설정했습니다 (20 m)."
                                            .to_owned(),
                                    );
                                } else {
                                    self.drawing.sheet.grid_follow_map = false;
                                }
                            }
                        }
                    });
                    ui.checkbox(
                        &mut self.drawing.sheet.grid_follow_map,
                        "지도 좌표격자와 같게 (20 m)",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let pdf_btn = egui::Button::new(
                            egui::RichText::new("PDF 내보내기")
                                .color(Color32::WHITE)
                                .strong()
                                .size(13.0),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(172.0, 36.0));
                        // Deferred until R3: cpp saves the real layout PDF
                        // (KaDrawingStudio.cpp:3819-3851); the Rust writer drew empty rectangles.
                        ui.add_enabled(false, pdf_btn).on_disabled_hover_text(DEFERRED_R3);
                        let prt_btn =
                            egui::Button::new(egui::RichText::new("인쇄").size(12.5).strong())
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(68.0, 36.0));
                        if ui.add(prt_btn).clicked() {
                            // KaPrintDialog.cpp:124 — 용지 기본값은 프린터가 A3 를 지원할 때만 A3.
                            self.modals.print_paper =
                                crate::dialogs_modal::default_print_paper_for_printer();
                            self.modals.active = Some(ActiveModalDialog::DrawingPrint);
                        }
                    });
    }
}
