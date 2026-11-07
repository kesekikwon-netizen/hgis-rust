use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::path::PathBuf;

use crate::app_state::StrataAppState;

#[derive(Clone, Copy)]
enum SwatchKind {
    CadastralOutline,
    TrenchBeige,
    RasterChecker,
    SurveyOrange,
    DistanceBox,
    BufferBand,
    GroupFolder,
    HeritageSub,
}

fn paint_checkbox_state(p: &egui::Painter, center: Pos2, check_state: u8) {
    let r = Rect::from_center_size(center, Vec2::new(13.0, 13.0));
    if check_state == 2 {
        p.rect_filled(r, 2.5, Color32::from_rgb(28, 28, 26));
        let c = r.center();
        p.line_segment(
            [Pos2::new(c.x - 3.2, c.y + 0.2), Pos2::new(c.x - 0.8, c.y + 2.8)],
            Stroke::new(1.7, Color32::WHITE),
        );
        p.line_segment(
            [Pos2::new(c.x - 0.8, c.y + 2.8), Pos2::new(c.x + 3.6, c.y - 2.6)],
            Stroke::new(1.7, Color32::WHITE),
        );
    } else if check_state == 1 {
        p.rect_filled(r, 2.5, Color32::from_rgb(88, 86, 80));
        let c = r.center();
        p.line_segment(
            [Pos2::new(c.x - 3.5, c.y), Pos2::new(c.x + 3.5, c.y)],
            Stroke::new(2.0, Color32::WHITE),
        );
    } else {
        p.rect_filled(r, 2.5, Color32::WHITE);
        p.rect_stroke(r, 2.5, Stroke::new(1.2, Color32::from_rgb(135, 133, 126)), egui::StrokeKind::Middle);
    }
}

fn paint_symbol_swatch(p: &egui::Painter, center: Pos2, kind: SwatchKind) {
    let r = Rect::from_center_size(center, Vec2::new(15.0, 11.0));
    match kind {
        SwatchKind::CadastralOutline => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(r, 1.0, Stroke::new(1.0, Color32::from_rgb(200, 197, 188)), egui::StrokeKind::Middle);
        }
        SwatchKind::TrenchBeige => {
            p.rect_filled(r, 1.0, Color32::from_rgb(237, 226, 216));
            p.rect_stroke(r, 1.0, Stroke::new(1.1, Color32::from_rgb(38, 37, 34)), egui::StrokeKind::Middle);
        }
        SwatchKind::RasterChecker => {
            let s = 4.5;
            let tl = Pos2::new(center.x - s, center.y - s);
            p.rect_filled(Rect::from_min_size(tl, Vec2::new(s, s)), 0.0, Color32::from_rgb(56, 108, 176));
            p.rect_filled(Rect::from_min_size(tl + Vec2::new(s, 0.0), Vec2::new(s, s)), 0.0, Color32::from_rgb(35, 35, 32));
            p.rect_filled(Rect::from_min_size(tl + Vec2::new(0.0, s), Vec2::new(s, s)), 0.0, Color32::from_rgb(35, 35, 32));
            p.rect_filled(Rect::from_min_size(tl + Vec2::new(s, s), Vec2::new(s, s)), 0.0, Color32::from_rgb(135, 168, 210));
        }
        SwatchKind::SurveyOrange => {
            p.rect_filled(r, 1.0, Color32::from_rgb(245, 208, 182));
            p.rect_stroke(r, 1.0, Stroke::new(2.0, Color32::from_rgb(214, 78, 22)), egui::StrokeKind::Middle);
        }
        SwatchKind::DistanceBox => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(r, 1.0, Stroke::new(1.1, Color32::from_rgb(38, 37, 34)), egui::StrokeKind::Middle);
        }
        SwatchKind::BufferBand => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            let top_band = Rect::from_min_size(r.min + Vec2::new(1.0, 2.0), Vec2::new(r.width() - 2.0, 3.0));
            p.rect_filled(top_band, 0.0, Color32::from_rgb(176, 62, 26));
            p.rect_stroke(r, 1.0, Stroke::new(1.1, Color32::from_rgb(38, 37, 34)), egui::StrokeKind::Middle);
        }
        SwatchKind::GroupFolder => {
            let b1 = Rect::from_center_size(center + Vec2::new(1.5, 1.0), Vec2::new(10.0, 9.0));
            let b0 = Rect::from_center_size(center - Vec2::new(1.5, 1.0), Vec2::new(10.0, 9.0));
            p.rect_filled(b1, 1.0, Color32::WHITE);
            p.rect_stroke(b1, 1.0, Stroke::new(1.0, Color32::from_rgb(95, 94, 88)), egui::StrokeKind::Middle);
            p.rect_filled(b0, 1.0, Color32::from_rgb(220, 230, 244));
            p.rect_stroke(b0, 1.0, Stroke::new(1.0, Color32::from_rgb(72, 98, 138)), egui::StrokeKind::Middle);
        }
        SwatchKind::HeritageSub => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(r, 1.0, Stroke::new(1.0, Color32::from_rgb(212, 209, 200)), egui::StrokeKind::Middle);
        }
    }
}

type LayerRowSpec = (
    &'static str,
    bool,
    f32,
    Option<&'static str>,
    SwatchKind,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    bool,
    Color32,
);

impl StrataAppState {
    pub(crate) fn render_cpp_left_layer_panel(&mut self, ui: &mut egui::Ui, show_set_btn: bool) {
        if self.right_inspector.left_panel_collapsed || self.right_inspector.map_focused {
            return;
        }
        egui::SidePanel::left("ka_cpp_left_panel")
            .resizable(true)
            .default_width(272.0)
            .width_range(250.0..=320.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("레이어").size(13.0).strong());
                    ui.add_space(8.0);
                    let f_btn = egui::Button::new(egui::RichText::new("파일함").size(12.0))
                        .fill(Color32::from_rgb(229, 224, 212))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(45, 44, 40)))
                        .corner_radius(5.0)
                        .min_size(Vec2::new(52.0, 24.0));
                    if ui.add(f_btn).clicked() {
                        self.home_map.files_open = !self.home_map.files_open;
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("전체 끄기").size(12.0)).frame(false)).clicked() {
                        let any_on = self.home_map.real_survey.is_layer_visible("지적도 · 조사 주변 5km");
                        self.home_map.real_survey.set_all_layers_visible(!any_on);
                        self.left_layers.set_all_visible(!any_on);
                    }
                    if show_set_btn {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let _ = ui.add(egui::Button::new(egui::RichText::new("세트 ▼").size(11.5)).frame(false));
                        });
                    }
                });

                if !show_set_btn {
                    ui.add_space(2.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.home_map.layer_filter)
                            .hint_text("Q 레이어 찾기")
                            .desired_width(f32::INFINITY),
                    );
                }
                ui.add_space(4.0);

                egui::Frame::NONE
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152)))
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(4, 4))
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();
                        let (hdr_r, _) = ui.allocate_exact_size(Vec2::new(avail_w, 22.0), egui::Sense::hover());
                        let hp = ui.painter_at(hdr_r);
                        hp.text(Pos2::new(hdr_r.left() + 4.0, hdr_r.center().y), egui::Align2::LEFT_CENTER, "도면 · 레이어", egui::FontId::proportional(11.0), Color32::from_rgb(95, 94, 88));
                        hp.text(Pos2::new(hdr_r.left() + avail_w - 42.0, hdr_r.center().y), egui::Align2::LEFT_CENTER, "글자", egui::FontId::proportional(11.0), Color32::from_rgb(95, 94, 88));
                        hp.line_segment([Pos2::new(hdr_r.left(), hdr_r.bottom() - 1.0), Pos2::new(hdr_r.right(), hdr_r.bottom() - 1.0)], Stroke::new(1.0, Color32::from_rgb(185, 182, 172)));

                        let dark = Color32::from_rgb(35, 35, 32);
                        let blue = Color32::from_rgb(38, 72, 132);
                        let purple_grp = Color32::from_rgb(98, 28, 118);
                        let purple_leaf = Color32::from_rgb(112, 36, 138);

                        let rows: &[LayerRowSpec] = &[
                            ("지적도", true, 0.0, None, SwatchKind::CadastralOutline, "지적도 · 조사 주변 5km", None, Some("지번"), false, dark),
                            ("조사 데이터", true, 0.0, None, SwatchKind::TrenchBeige, "시굴격자", Some("232"), Some("이름·면적"), true, dark),
                            ("주변·참조", true, 0.0, None, SwatchKind::RasterChecker, "지형 음영", None, None, false, dark),
                            ("", false, 0.0, Some("▶"), SwatchKind::RasterChecker, "지표모델(DSM) 30m · Coperni...", None, None, true, dark),
                            ("", true, 0.0, None, SwatchKind::SurveyOrange, "조사구역", Some("1"), Some("면적"), false, dark),
                            ("", false, 0.0, None, SwatchKind::DistanceBox, "주변 500m 거리", Some("4"), Some("이름"), true, dark),
                            ("", false, 0.0, None, SwatchKind::BufferBand, "주변 500m", Some("1"), None, false, dark),
                            ("", true, 0.0, Some("▼"), SwatchKind::GroupFolder, "참조 지도", None, Some("글자"), true, blue),
                            ("", false, 10.0, Some("▼"), SwatchKind::GroupFolder, "지정유산", None, Some("글자"), false, purple_grp),
                            ("", false, 18.0, None, SwatchKind::HeritageSub, "국가등록문화유산", None, Some("번호"), true, purple_leaf),
                            ("", false, 18.0, None, SwatchKind::HeritageSub, "국가지정유산", None, Some("번호"), false, purple_leaf),
                            ("", false, 18.0, None, SwatchKind::HeritageSub, "국가지정유산보호구역", None, Some("번호"), true, purple_leaf),
                            ("", false, 18.0, None, SwatchKind::HeritageSub, "시도지정유산", None, Some("번호"), false, purple_leaf),
                            ("", false, 18.0, None, SwatchKind::HeritageSub, "시도지정유산보호구역", None, Some("번호"), true, purple_leaf),
                        ];

                        let filter = self.home_map.layer_filter.trim().to_owned();
                        for (sec_title, top_rule, indent, arrow, sw_kind, name, cnt, right_lbl, alt_bg, txt_col) in rows {
                            if !filter.is_empty() && !name.contains(&filter) {
                                continue;
                            }
                            let band_h = if !sec_title.is_empty() { 16.0 } else if *top_rule { 4.0 } else { 0.0 };
                            let row_h = 20.0;
                            let (full_r, resp) = ui.allocate_exact_size(Vec2::new(avail_w, band_h + row_h), egui::Sense::click());
                            let right_col_x = full_r.right() - 52.0;
                            if resp.clicked() {
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    if pos.x < full_r.left() + 42.0 + *indent {
                                        self.home_map.real_survey.toggle_layer_visible(name);
                                    } else if pos.x >= right_col_x && right_lbl.is_some() {
                                        self.home_map.real_survey.toggle_label_visible(name);
                                    } else {
                                        self.right_inspector.selected_layer_key = Some((*name).to_owned());
                                    }
                                } else {
                                    self.right_inspector.selected_layer_key = Some((*name).to_owned());
                                }
                            }
                            let is_sel = self.right_inspector.selected_layer_key.as_deref() == Some(*name);
                            let p = ui.painter_at(full_r);
                            if is_sel {
                                p.rect_filled(Rect::from_min_max(Pos2::new(full_r.left(), full_r.top() + band_h), full_r.max), 0.0, Color32::from_rgb(219, 212, 196));
                            } else if *alt_bg {
                                p.rect_filled(Rect::from_min_max(Pos2::new(full_r.left(), full_r.top() + band_h), full_r.max), 0.0, Color32::from_rgb(239, 236, 228));
                            }
                            if *top_rule {
                                p.line_segment([Pos2::new(full_r.left(), full_r.top() + 1.0), Pos2::new(full_r.right(), full_r.top() + 1.0)], Stroke::new(1.0, Color32::from_rgb(205, 202, 194)));
                            }
                            if !sec_title.is_empty() {
                                p.text(Pos2::new(full_r.left() + 4.0, full_r.top() + 9.0), egui::Align2::LEFT_CENTER, *sec_title, egui::FontId::proportional(10.5), Color32::from_rgb(110, 108, 102));
                            }
                            let ry = full_r.top() + band_h + row_h * 0.5;
                            let mut x = full_r.left() + 6.0 + *indent;
                            if let Some(arr) = arrow {
                                p.text(Pos2::new(x, ry), egui::Align2::LEFT_CENTER, *arr, egui::FontId::proportional(10.0), Color32::from_rgb(95, 94, 88));
                            }
                            x += 12.0;
                            let l_state = self.home_map.real_survey.layer_check_state(name);
                            paint_checkbox_state(&p, Pos2::new(x + 6.0, ry), l_state);
                            x += 18.0;
                            paint_symbol_swatch(&p, Pos2::new(x + 7.5, ry), *sw_kind);
                            x += 20.0;
                            p.text(Pos2::new(x, ry), egui::Align2::LEFT_CENTER, *name, egui::FontId::proportional(11.0), *txt_col);

                            if let Some(c_str) = cnt {
                                let dyn_cnt = if *name == "조사구역" {
                                    self.home_map.real_survey.survey_rings.len().to_string()
                                } else if *name == "시굴격자" {
                                    self.home_map.real_survey.trenches.len().max(232).to_string()
                                } else {
                                    (*c_str).to_owned()
                                };
                                p.text(Pos2::new(right_col_x - 8.0, ry), egui::Align2::RIGHT_CENTER, dyn_cnt, egui::FontId::proportional(10.5), Color32::from_rgb(115, 113, 106));
                            }
                            if let Some(rl) = right_lbl {
                                let lbl_vis = self.home_map.real_survey.is_label_visible(name);
                                paint_checkbox_state(&p, Pos2::new(right_col_x + 6.0, ry), if lbl_vis { 2 } else { 0 });
                                p.text(Pos2::new(right_col_x + 16.0, ry), egui::Align2::LEFT_CENTER, *rl, egui::FontId::proportional(10.5), Color32::from_rgb(75, 74, 70));
                            } else {
                                p.text(Pos2::new(right_col_x + 4.0, ry), egui::Align2::LEFT_CENTER, "—", egui::FontId::proportional(10.5), Color32::from_rgb(135, 133, 126));
                            }
                        }
                    });

                ui.add_space(4.0);
                ui.label(egui::RichText::new("▶ 표시 설정").size(11.5).color(Color32::from_rgb(80, 79, 74)));
                ui.add_space(4.0);
                let (div_r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 3.0), egui::Sense::hover());
                ui.painter().rect_filled(div_r, 0.0, Color32::from_rgb(125, 123, 116));
                ui.add_space(4.0);
                ui.label(egui::RichText::new("파일함").size(13.0).strong());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    for (t, sub) in [("내 PC", r"C:\Users\권을"), ("바탕화면", r"C:\Users\권을\Desktop"), ("문서", r"C:\Users\권을\Documents")] {
                        if ui.add(egui::Button::new(egui::RichText::new(t).size(11.5)).frame(false)).clicked() {
                            self.home_map.real_survey.file_browser_dir = PathBuf::from(sub);
                            self.home_map.real_survey.refresh_file_browser();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    for (t, sub) in [("내려받기", r"C:\Users\권을\Downloads"), ("폴더 직접 찾기", r"C:\Users\권을\Documents\경주시")] {
                        if ui.add(egui::Button::new(egui::RichText::new(t).size(11.5)).frame(false)).clicked() {
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
                        if ui.add(up_b).clicked() {
                            if let Some(parent) = self.home_map.real_survey.file_browser_dir.parent().map(PathBuf::from) {
                                self.home_map.real_survey.file_browser_dir = parent;
                                self.home_map.real_survey.refresh_file_browser();
                            }
                        }
                    });
                });
                let box_w = ui.available_width();
                let (box_r, _) = ui.allocate_exact_size(Vec2::new(box_w, 82.0), egui::Sense::hover());
                let bp = ui.painter_at(box_r);
                bp.rect_filled(box_r, 4.0, Color32::WHITE);
                bp.rect_stroke(box_r, 4.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
                for (fi, (fname, fpath, is_dir)) in self.home_map.real_survey.file_browser_entries.clone().iter().take(4).enumerate() {
                    let row_r = Rect::from_min_size(Pos2::new(box_r.left() + 4.0, box_r.top() + 3.0 + fi as f32 * 18.0), Vec2::new(box_w - 8.0, 17.0));
                    let prefix = if *is_dir { "📁 " } else { "📄 " };
                    bp.text(Pos2::new(row_r.left() + 4.0, row_r.center().y), egui::Align2::LEFT_CENTER, format!("{prefix}{fname}"), egui::FontId::proportional(11.0), Color32::from_rgb(45, 44, 41));
                    if ui.interact(row_r, ui.id().with(("fb_entry", fi)), egui::Sense::click()).clicked() && *is_dir {
                        self.home_map.real_survey.file_browser_dir = fpath.clone();
                        self.home_map.real_survey.refresh_file_browser();
                    }
                }
            });
    }
}
