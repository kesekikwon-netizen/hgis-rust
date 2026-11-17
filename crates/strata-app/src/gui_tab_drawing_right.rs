//! 도면 탭 오른쪽 패널(도면 항목·범례·방위) 앞부분.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// 오른쪽 「도면 항목」 패널 껍데기(폭 268) — 원문 `SidePanel::right` 블록.
    pub(crate) fn paint_drawing_right_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::right("ka_drawing_right_panel")
            .resizable(true)
            .default_width(286.0)
            .width_range(200.0_f32.min(ui.available_width())..=360.0_f32.min(ui.available_width()))
            .show_inside(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.spacing_mut().interact_size.y = 28.0;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    self.paint_drawing_right_items(ui);
                    self.paint_drawing_right_items_b(ui);
                });
            });
    }

    /// 도면 항목 단추·범례 제목·글자 크기·방위표 종류.
    fn paint_drawing_right_items(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("도면 항목").size(11.5).color(Color32::from_rgb(95, 94, 88)));
        ui.columns(2, |cols| {
            let legend_sel = self.drawing.sheet.legend_visible;
            let b0 = egui::Button::new(egui::RichText::new("≡  범례").size(12.0).strong())
                .fill(if legend_sel { Color32::from_rgb(228, 223, 210) } else { Color32::WHITE })
                .stroke(Stroke::new(if legend_sel { 1.4 } else { 1.0 }, Color32::from_rgb(135, 132, 122)))
                .corner_radius(5.0)
                .min_size(Vec2::new(cols[0].available_width(), 40.0));
            // beginPlaceLegend (KaDrawingStudio.cpp:1606-1616): puts it on, never takes it off.
            if cols[0].add(b0).clicked() {
                self.put_item_on_sheet(crate::tab_drawing::SheetItem::Legend);
            }
            let title_sel = self.drawing.sheet.title_block_visible;
            let b1 = egui::Button::new(egui::RichText::new("⊞  표제란").size(12.0).strong())
                .fill(if title_sel { Color32::from_rgb(228, 223, 210) } else { Color32::WHITE })
                .stroke(Stroke::new(if title_sel { 1.4 } else { 1.0 }, Color32::from_rgb(135, 132, 122)))
                .corner_radius(5.0)
                .min_size(Vec2::new(cols[1].available_width(), 40.0));
            // placeTitleBlock (KaDrawingStudioSheets.cpp:35-69): puts it on; Delete takes it off.
            if cols[1].add(b1).clicked() {
                self.put_item_on_sheet(crate::tab_drawing::SheetItem::TitleBlock);
            }
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new("범례 제목").size(12.0));
        ui.add(egui::TextEdit::singleline(&mut self.drawing.sheet.legend_title).desired_width(f32::INFINITY));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("글자").size(12.0));
            // KaDrawingStudio.cpp:953-956 — QSpinBox 5..=24 pt
            ui.add(egui::DragValue::new(&mut self.drawing.sheet.legend_font_pt).range(5..=24).speed(0.2).suffix(" pt"));
            ui.checkbox(&mut self.drawing.sheet.legend_bold, "B");
            ui.checkbox(&mut self.drawing.sheet.legend_italic, "I");
        });
        ui.checkbox(&mut self.drawing.sheet.show_site_names_under_5000, "1:5,000 이하에서 유적 이름으로 표시")
            .on_hover_text("켜면 1:5,000과 더 크게 본 축척에서 주변유적 번호 대신 유적 이름을 쓰고, 범례에는 유적 종류만 남깁니다");

        ui.separator();
        ui.label(egui::RichText::new("방위 (도북)").size(11.5).color(Color32::from_rgb(95, 94, 88)));
        let columns = if ui.available_width() < 280.0 { 2 } else { 4 };
        let width = (ui.available_width() - ui.spacing().item_spacing.x * (columns - 1) as f32) / columns as f32;
        egui::Grid::new("drawing_north_styles").num_columns(columns).show(ui, |ui| {
            for (i, style) in strata_layout::sheet_composer::NorthArrowStyle::ALL.into_iter().enumerate() {
                let sel = self.drawing.sheet.north_arrow_style == style;
                let fill = if sel { Color32::from_rgb(228, 223, 210) } else { Color32::WHITE };
                let stroke = if sel { Stroke::new(1.4, Color32::from_rgb(90, 88, 82)) } else { Stroke::new(1.0, Color32::from_rgb(195, 192, 182)) };
                let b = egui::Button::new(egui::RichText::new(format!("{}\n{}", style.symbol_glyph(), style.label_ko())).size(11.0))
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(5.0)
                    .min_size(Vec2::new(width, 44.0));
                // beginPlaceNorth (KaDrawingStudio.cpp:1618-1627).
                if ui.add_sized([width, 44.0], b).clicked() {
                    self.drawing.sheet.north_arrow_style = style;
                    self.put_item_on_sheet(crate::tab_drawing::SheetItem::NorthArrow);
                }
                if (i + 1) % columns == 0 { ui.end_row(); }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("크기").size(12.0));
            // KaDrawingStudio.cpp:1018-1022 — QDoubleSpinBox 3..=80 mm
            ui.add(egui::DragValue::new(&mut self.drawing.sheet.north_arrow_size_mm).range(3.0..=80.0).speed(1.0).fixed_decimals(0).suffix(" mm"));
        });

        ui.separator();
        ui.label(egui::RichText::new("도면 정보").size(11.5).color(Color32::from_rgb(95, 94, 88)));
        ui.horizontal(|ui| {
            ui.label("1 :");
            // KaDrawingStudio.cpp:1061-1082, 3121-3137 — the box keeps what is typed or dragged
            // until 「적용」, Enter or leaving it (keyboardTracking off).
            let mut entry = self.sheet_scale_entry();
            let response = ui.add(egui::DragValue::new(&mut entry).range(10..=5_000_000).speed(50.0));
            if response.changed() {
                self.edit_sheet_scale(entry);
            }
            let finished = response.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape));
            if ui.button("적용").clicked() || finished {
                self.apply_sheet_scale_entry();
            }
        });
        ui.add_space(4.0);
    }
}
