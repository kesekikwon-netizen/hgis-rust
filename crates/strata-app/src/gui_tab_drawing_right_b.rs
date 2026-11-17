//! 도면 탭 오른쪽 패널(축척·용지·도면 정보) 뒷부분.

use eframe::egui::{self, Color32, Stroke, Vec2};

use crate::app_state::MainTab;
use crate::app_state::StrataAppState;

impl StrataAppState {
    /// 축척 프리셋·용지 크기·도면 정보 칸.
    pub(crate) fn paint_drawing_right_items_b(&mut self, ui: &mut egui::Ui) {
        // Equal chips wrap within the inspector (target/cpp-ref/src/app/KaDrawingStudio.cpp:1100-1103).
        let columns = ((ui.available_width() / 46.0) as usize).clamp(1, 7);
        let width = (ui.available_width() - 3.0 * (columns - 1) as f32) / columns as f32;
        egui::Grid::new("ka_drawing_scale_presets").num_columns(columns).spacing([3.0, 3.0]).show(ui, |ui| {
            for (index, value) in [20, 30, 50, 100, 200, 250, 300, 400, 500, 1000, 2000, 5000, 10000, 25000].into_iter().enumerate() {
                let chip = egui::Button::new(egui::RichText::new(value.to_string()).size(10.0)).selected(value == self.drawing.sheet.scale_denominator);
                if ui.add_sized([width, 28.0], chip).clicked() {
                    self.pick_sheet_scale(value);
                }
                if (index + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
        ui.add_space(6.0);
        ui.columns(3, |cols| {
            for (i, style) in strata_layout::sheet_composer::ScaleBarStyle::ALL.into_iter().enumerate() {
                let sel = self.drawing.sheet.scale_bar_style == style;
                let fill = if sel { Color32::from_rgb(228, 223, 210) } else { Color32::WHITE };
                let stroke = if sel { Stroke::new(1.4, Color32::from_rgb(90, 88, 82)) } else { Stroke::new(1.0, Color32::from_rgb(195, 192, 182)) };
                let b = egui::Button::new(egui::RichText::new(format!("{}\n{}", style.symbol_glyph(), style.label_ko())).size(11.0))
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(5.0)
                    .min_size(Vec2::new(cols[i].available_width(), 42.0));
                // beginPlaceScaleBar (KaDrawingStudio.cpp:1629-1638).
                if cols[i].add(b).clicked() {
                    self.drawing.sheet.scale_bar_style = style;
                    self.put_item_on_sheet(crate::tab_drawing::SheetItem::ScaleBar);
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("선택 항목 크기").size(12.0));
            // KaDrawingStudio.cpp:1137-1146 — QSpinBox 25..=200 %
            ui.add(egui::DragValue::new(&mut self.drawing.decoration_size_percent).range(25..=200).speed(1.0).suffix(" %"))
                .on_hover_text("용지의 축척자·축척·좌표계·방위표를 선택해 크기를 조절합니다. 위치는 끌어서 옮깁니다.");
        });
        ui.add_space(4.0);
        ui.columns(3, |cols| {
            for (i, lbl) in ["축척 글자", "좌표계", "좌표 격자"].into_iter().enumerate() {
                let clicked = cols[i].add_sized([cols[i].available_width(), 24.0], egui::Button::new(egui::RichText::new(lbl).size(11.0))).clicked();
                // beginPlaceScaleLabel / beginPlaceCrsLabel (KaDrawingStudio.cpp:1640-1660).
                if clicked && i < 2 {
                    let item = if i == 0 { crate::tab_drawing::SheetItem::ScaleText } else { crate::tab_drawing::SheetItem::CrsLabel };
                    self.put_item_on_sheet(item);
                }
                if clicked && i == 2 {
                    let next_follow = !self.drawing.sheet.grid_follow_map;
                    if next_follow {
                        self.drawing.sheet.apply_map_coordinate_grid(
                            true,
                            0.0,
                            20.0,
                            // 지도 좌표격자의 원점 — C++ 기본은 좌표계 원점(0, 0)(KaGridOriginButton.cpp:36).
                            [0.0, 0.0],
                        );
                        self.home_map.draw_guide_message = Some("도면 격자를 지도 좌표격자와 같게 설정했습니다 (20 m).".to_owned());
                    } else {
                        self.drawing.sheet.grid_follow_map = false;
                    }
                }
            }
        });
        ui.checkbox(&mut self.drawing.sheet.grid_follow_map, "지도 좌표격자와 같게 (20 m)");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let room = (ui.available_width() - ui.spacing().item_spacing.x).max(0.0);
            let pdf_btn = egui::Button::new(egui::RichText::new("PDF 내보내기").color(Color32::WHITE).strong().size(13.0))
                .fill(Color32::from_rgb(181, 87, 58))
                .corner_radius(6.0)
                .min_size(Vec2::new(room * 0.64, 36.0));
            // KaDrawingStudio.cpp:929-932, 3819-3855 — the sheet as a picture at 300 DPI until the
            // vector PDF of R3 (docs/RUST_PLAN.md R3-임시).
            if ui.add_sized([room * 0.64, 36.0], pdf_btn).on_hover_text("지금 용지를 PDF 파일로 저장합니다").clicked() {
                self.export_sheet_pdf();
            }
            let prt_btn = egui::Button::new(egui::RichText::new("인쇄").size(12.5).strong())
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                .corner_radius(6.0)
                .min_size(Vec2::new(room * 0.36, 36.0));
            if ui.add_sized([room * 0.36, 36.0], prt_btn).clicked() {
                self.open_studio_tab(MainTab::Print);
            }
        });
    }
}
