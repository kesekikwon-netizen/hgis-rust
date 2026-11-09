//! 도면 탭 오른쪽 패널(도면 항목·범례·방위) 앞부분.

use eframe::egui::{self, Color32, Stroke, Vec2};


use crate::app_state::StrataAppState;

impl StrataAppState {
    /// 오른쪽 「도면 항목」 패널 껍데기(폭 268) — 원문 `SidePanel::right` 블록.
    pub(crate) fn paint_drawing_right_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::right("ka_drawing_right_panel")
            .resizable(false)
            .exact_width(268.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.paint_drawing_right_items(ui);
                    self.paint_drawing_right_items_b(ui);
                });
            });
    }

    /// 도면 항목 단추·범례 제목·글자 크기·방위표 종류.
    fn paint_drawing_right_items(&mut self, ui: &mut egui::Ui) {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("도면 항목")
                            .size(11.5)
                            .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.columns(2, |cols| {
                        let legend_sel = self.drawing.sheet.legend_visible;
                        let b0 =
                            egui::Button::new(egui::RichText::new("≡  범례").size(12.0).strong())
                                .fill(if legend_sel {
                                    Color32::from_rgb(228, 223, 210)
                                } else {
                                    Color32::WHITE
                                })
                                .stroke(Stroke::new(
                                    if legend_sel { 1.4 } else { 1.0 },
                                    Color32::from_rgb(135, 132, 122),
                                ))
                                .corner_radius(5.0)
                                .min_size(Vec2::new(cols[0].available_width(), 40.0));
                        if cols[0].add(b0).clicked() {
                            self.drawing.sheet.legend_visible = !self.drawing.sheet.legend_visible;
                        }
                        let title_sel = self.drawing.sheet.title_block_visible;
                        let b1 =
                            egui::Button::new(egui::RichText::new("⊞  표제란").size(12.0).strong())
                                .fill(if title_sel {
                                    Color32::from_rgb(228, 223, 210)
                                } else {
                                    Color32::WHITE
                                })
                                .stroke(Stroke::new(
                                    if title_sel { 1.4 } else { 1.0 },
                                    Color32::from_rgb(135, 132, 122),
                                ))
                                .corner_radius(5.0)
                                .min_size(Vec2::new(cols[1].available_width(), 40.0));
                        if cols[1].add(b1).clicked() {
                            self.drawing.sheet.title_block_visible =
                                !self.drawing.sheet.title_block_visible;
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("범례 제목").size(12.0));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drawing.sheet.legend_title)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("글자").size(12.0));
                        // KaDrawingStudio.cpp:953-956 — QSpinBox 5..=24 pt
                        ui.add(
                            egui::DragValue::new(&mut self.drawing.sheet.legend_font_pt)
                                .range(5..=24)
                                .speed(0.2)
                                .suffix(" pt"),
                        );
                        ui.checkbox(&mut self.drawing.sheet.legend_bold, "B");
                        ui.checkbox(&mut self.drawing.sheet.legend_italic, "I");
                    });
                    ui.checkbox(
                        &mut self.drawing.sheet.show_site_names_under_5000,
                        "1:5,000 이하에서 유적 이름으로 표시",
                    )
                    .on_hover_text(
                        "켜면 1:5,000과 더 크게 본 축척에서 주변유적 번호 대신 유적 이름을 쓰고, 범례에는 유적 종류만 남깁니다",
                    );

                    ui.separator();
                    ui.label(
                        egui::RichText::new("방위 (도북)")
                            .size(11.5)
                            .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.columns(4, |cols| {
                        for (i, style) in strata_layout::sheet_composer::NorthArrowStyle::ALL
                            .into_iter()
                            .enumerate()
                        {
                            let sel = self.drawing.sheet.north_arrow_style == style;
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
                            .min_size(Vec2::new(cols[i].available_width(), 44.0));
                            if cols[i].add(b).clicked() {
                                self.drawing.sheet.north_arrow_style = style;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("크기").size(12.0));
                        // KaDrawingStudio.cpp:1018-1022 — QDoubleSpinBox 3..=80 mm
                        ui.add(
                            egui::DragValue::new(&mut self.drawing.sheet.north_arrow_size_mm)
                                .range(3.0..=80.0)
                                .speed(1.0)
                                .fixed_decimals(0)
                                .suffix(" mm"),
                        );
                    });

                    ui.separator();
                    ui.label(
                        egui::RichText::new("도면 정보")
                            .size(11.5)
                            .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.horizontal(|ui| {
                        ui.label("1 :");
                        // KaDrawingStudio.cpp:158-166 — 축척 값을 직접 끌어 바꾼다
                        ui.add(
                            egui::DragValue::new(&mut self.drawing.sheet.scale_denominator)
                                .range(10..=5_000_000)
                                .speed(50.0),
                        );
                        if ui.button("적용").clicked() {
                            self.home_map.draw_guide_message = Some(format!(
                                "축척을 1:{} 로 적용했습니다.",
                                self.drawing.sheet.scale_denominator
                            ));
                        }
                    });
                    ui.add_space(4.0);
    }
}
