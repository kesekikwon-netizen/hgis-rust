//! 레이어 카드의 「표시 설정」 접이식 부분과 하단 파일함 카드.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_layer_panel_consts::{
    FILE_BTN_DESK_TIP, FILE_BTN_DOCS_TIP, FILE_BTN_DOWN_TIP, FILE_BTN_PC_TIP, FILE_BTN_PICK_TIP,
    FILE_BTN_UP_TIP, FILE_LIST_TIP, KA_LABEL_INKS, UX_FILE_DRAWER_HEIGHT,
};

impl StrataAppState {
    /// KaLayerInformationPanel.cpp:54-132 - 「표시 설정」 접이식 머리글과 내용.
    pub(crate) fn paint_layer_display_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        // KaLayerInformationPanel.cpp:54,132 — clickable collapsible header
        let open = self.home_map.layer_display_settings_open;
        let header = if open { "▼ 표시 설정" } else { "▶ 표시 설정" };
        let resp = ui.add(
            egui::Label::new(
                egui::RichText::new(header)
                    .size(11.5)
                    .color(Color32::from_rgb(80, 79, 74)),
            )
            .sense(egui::Sense::click()),
        );
        if resp.clicked() {
            self.home_map.layer_display_settings_open = !open;
        }
        if self.home_map.layer_display_settings_open {
            ui.add_space(2.0);
            // KaLayerInformationPanel.cpp:62-97 compact form
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("내용").size(11.0));
                const FIELDS: [&str; 3] = ["이름 (name)", "지번 (jibun)", "번호 (id)"];
                let idx = self.home_map.layer_label_field_idx.min(2);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(format!("{}  ▼", FIELDS[idx]))
                                .size(11.0),
                        )
                        .min_size(Vec2::new(96.0, 20.0)),
                    )
                    .clicked()
                {
                    self.home_map.layer_label_field_idx = (idx + 1) % 3;
                }
                ui.checkbox(
                    &mut self.home_map.layer_label_with_area,
                    egui::RichText::new("면적(㎡) 함께 표시").size(11.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("크기").size(11.0));
                ui.add(
                    egui::DragValue::new(&mut self.home_map.layer_label_size_pt)
                        .range(6..=24)
                        .speed(0.2)
                        .suffix(" pt"),
                );
                ui.label(egui::RichText::new("글자 색").size(11.0));
                let ink_idx = self.home_map.layer_label_ink_idx.min(5);
                let (ink_name, ink_rgb) = KA_LABEL_INKS[ink_idx];
                let mut btn = egui::Button::new(
                    egui::RichText::new(format!("{ink_name}  ▼")).size(11.0),
                )
                .min_size(Vec2::new(96.0, 20.0));
                if ink_idx != 0 {
                    btn = btn.fill(Color32::from_rgb(ink_rgb.0, ink_rgb.1, ink_rgb.2));
                }
                if ui.add(btn).clicked() {
                    self.home_map.layer_label_ink_idx = (ink_idx + 1) % 6;
                }
                let swatch = ui
                    .allocate_exact_size(Vec2::new(14.0, 12.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    swatch.0,
                    2.0,
                    Color32::from_rgb(ink_rgb.0, ink_rgb.1, ink_rgb.2),
                );
            });
            ui.checkbox(
                &mut self.home_map.layer_label_buffer_halo,
                egui::RichText::new("흰 테두리 (위성·지적 위에서 읽기)").size(11.0),
            );
        }
    }

    /// KaFileBrowserPanel.cpp:92-136 - 하단 파일함 카드(폴더 단추·목록 칸).
    pub(crate) fn paint_file_drawer(&mut self, ui: &mut egui::Ui, panel_w: f32) {
        // 2. 왼쪽 하단 파일함 카드 (w=351, h=168)
        let file_card_h = UX_FILE_DRAWER_HEIGHT.min(ui.available_height().max(140.0));
        let (file_card_rect, _) = ui.allocate_exact_size(
            Vec2::new(panel_w, file_card_h),
            egui::Sense::hover(),
        );
        let mut f_ui = ui.new_child(egui::UiBuilder::new().max_rect(file_card_rect));
        f_ui.vertical(|ui| {
            ui.label(egui::RichText::new("파일함").size(13.0).strong());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                // KaFileBrowserPanel.cpp:92-111 — 여섯 폴더 단추의 안내말
                for (t, sub, tip) in [
                    ("내 PC", r"C:\Users\권을", FILE_BTN_PC_TIP),
                    ("바탕화면", r"C:\Users\권을\Desktop", FILE_BTN_DESK_TIP),
                    ("문서", r"C:\Users\권을\Documents", FILE_BTN_DOCS_TIP),
                ] {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(t).size(11.5))
                                .frame(false),
                        )
                        .on_hover_text(tip)
                        .clicked()
                    {
                        self.home_map.real_survey.file_browser_dir = PathBuf::from(sub);
                        self.home_map.real_survey.refresh_file_browser();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                for (t, sub, tip) in [
                    ("내려받기", r"C:\Users\권을\Downloads", FILE_BTN_DOWN_TIP),
                    (
                        "폴더 직접 찾기",
                        r"C:\Users\권을\Documents\경주시",
                        FILE_BTN_PICK_TIP,
                    ),
                ] {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(t).size(11.5))
                                .frame(false),
                        )
                        .on_hover_text(tip)
                        .clicked()
                    {
                        self.home_map.real_survey.file_browser_dir = PathBuf::from(sub);
                        self.home_map.real_survey.refresh_file_browser();
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let up_b = egui::Button::new(egui::RichText::new("상위").size(11.5))
                        .fill(Color32::from_rgb(229, 224, 212))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                        .corner_radius(5.0)
                        .min_size(Vec2::new(42.0, 22.0));
                    if ui.add(up_b).on_hover_text(FILE_BTN_UP_TIP).clicked() {
                        if let Some(parent) = self
                            .home_map
                            .real_survey
                            .file_browser_dir
                            .parent()
                            .map(PathBuf::from)
                        {
                            self.home_map.real_survey.file_browser_dir = parent;
                            self.home_map.real_survey.refresh_file_browser();
                        }
                    }
                });
            });
            let box_w = ui.available_width();
            let box_h = (file_card_rect.bottom() - ui.cursor().top() - 2.0).max(86.0);
            let (box_r, box_resp) =
                ui.allocate_exact_size(Vec2::new(box_w, box_h), egui::Sense::hover());
            // KaFileBrowserPanel.cpp:136 — 목록 칸에 끌어 넣으면 레이어가 된다
            box_resp.on_hover_text(FILE_LIST_TIP);
            let bp = ui.painter_at(box_r);
            bp.rect_filled(box_r, 4.0, Color32::WHITE);
            bp.rect_stroke(
                box_r,
                4.0,
                Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                egui::StrokeKind::Middle,
            );
            for (fi, (fname, fpath, is_dir)) in self
                .home_map
                .real_survey
                .file_browser_entries
                .clone()
                .iter()
                .take(5)
                .enumerate()
            {
                let row_r = Rect::from_min_size(
                    Pos2::new(box_r.left() + 4.0, box_r.top() + 3.0 + fi as f32 * 18.0),
                    Vec2::new(box_w - 8.0, 17.0),
                );
                let prefix = if *is_dir { "📁 " } else { "📄 " };
                bp.text(
                    Pos2::new(row_r.left() + 4.0, row_r.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{prefix}{fname}"),
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(45, 44, 41),
                );
                if ui
                    .interact(row_r, ui.id().with(("fb_entry", fi)), egui::Sense::click())
                    .clicked()
                {
                    if *is_dir {
                        self.home_map.real_survey.file_browser_dir = fpath.clone();
                        self.home_map.real_survey.refresh_file_browser();
                    } else if let Ok(driver) =
                        crate::left_layer_panel::detect_file_drawer_driver(fpath)
                    {
                        self.home_map.draw_guide_message = Some(format!(
                            "파일함에서 「{}」 (드라이버: {:?}) 레이어를 적재했습니다.",
                            fname, driver
                        ));
                    }
                }
            }
        });
    }
}
