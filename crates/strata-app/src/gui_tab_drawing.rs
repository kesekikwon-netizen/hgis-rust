use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_layout::badge_placer::LayoutBadgePlacer;

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// C++ `KaDrawingStudio.cpp` 도면 스튜디오 (좌측 레이어/파일함 + 중앙 도면 용지 & 하단 7버튼 툴바 + 우측 도면 항목/방위/도면 정보) 1:1 재현
    pub(crate) fn render_drawing_tab_ui(&mut self, ui: &mut egui::Ui) {
        self.render_cpp_left_layer_panel(ui, true);

        egui::SidePanel::right("ka_drawing_right_panel")
            .resizable(true)
            .default_width(268.0)
            .width_range(245.0..=320.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
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
                        let pt_lbl =
                            format!("{} pt               ▼", self.drawing.sheet.legend_font_pt);
                        if ui.button(pt_lbl).clicked() {
                            self.drawing.sheet.legend_font_pt =
                                if self.drawing.sheet.legend_font_pt >= 14 {
                                    9
                                } else {
                                    self.drawing.sheet.legend_font_pt + 1
                                };
                        }
                        ui.checkbox(&mut self.drawing.sheet.legend_bold, "B");
                        ui.checkbox(&mut self.drawing.sheet.legend_italic, "I");
                    });
                    ui.checkbox(
                        &mut self.drawing.sheet.show_site_names_under_5000,
                        "1:5,000 이하에서 유적 이름으로 표시",
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
                        let sz_lbl = format!(
                            "{:.0} mm                                  ▼",
                            self.drawing.sheet.north_arrow_size_mm
                        );
                        if ui
                            .add_sized([ui.available_width(), 24.0], egui::Button::new(sz_lbl))
                            .clicked()
                        {
                            self.drawing.sheet.north_arrow_size_mm =
                                if self.drawing.sheet.north_arrow_size_mm >= 36.0 {
                                    20.0
                                } else {
                                    self.drawing.sheet.north_arrow_size_mm + 4.0
                                };
                        }
                    });

                    ui.separator();
                    ui.label(
                        egui::RichText::new("도면 정보")
                            .size(11.5)
                            .color(Color32::from_rgb(95, 94, 88)),
                    );
                    ui.horizontal(|ui| {
                        ui.label("1 :");
                        let scale_txt = format!(
                            "{}                               ▼",
                            self.drawing.sheet.scale_denominator
                        );
                        let _ = ui.add_sized([165.0, 24.0], egui::Button::new(scale_txt));
                        let _ = ui.button("적용");
                    });
                    ui.add_space(4.0);
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
                        let _ = ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::Button::new("100 %                        ▼"),
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
                                        self.home_map.real_survey.survey_center,
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
                        if ui.add(pdf_btn).clicked() {
                            if let Ok(path) = self.drawing.export_pdf_to_temp() {
                                self.home_map.draw_guide_message = Some(format!(
                                    "조사도면 PDF를 저장했습니다: {}",
                                    path.display()
                                ));
                            }
                        }
                        let prt_btn =
                            egui::Button::new(egui::RichText::new("인쇄").size(12.5).strong())
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(68.0, 36.0));
                        if ui.add(prt_btn).clicked() {
                            self.modals.active = Some(ActiveModalDialog::DrawingPrint);
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 0.0, Color32::from_rgb(245, 243, 236));

            let hint_r = Rect::from_min_size(Pos2::new(rect.left() + 18.0, rect.top() + 170.0), Vec2::new(186.0, 32.0));
            p.rect_filled(hint_r, 6.0, Color32::WHITE);
            p.rect_stroke(hint_r, 6.0, Stroke::new(1.0, Color32::from_rgb(190, 188, 178)), egui::StrokeKind::Middle);
            p.text(hint_r.center(), egui::Align2::CENTER_CENTER, "투명도·밝기: 조절할 레이어를 고르세요", egui::FontId::proportional(11.0), Color32::from_rgb(75, 74, 70));

            let sheet_h = (rect.height() - 108.0).clamp(320.0, 620.0);
            let sheet_w = sheet_h * (210.0 / 297.0);
            let sheet_rect = Rect::from_center_size(Pos2::new(rect.center().x, rect.center().y - 18.0), Vec2::new(sheet_w, sheet_h));
            p.rect_filled(sheet_rect.translate(Vec2::new(4.0, 5.0)), 1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 38));
            p.rect_filled(sheet_rect, 1.0, Color32::WHITE);

            let map_rect = Rect::from_min_max(
                sheet_rect.min + Vec2::new(16.0, 16.0),
                Pos2::new(sheet_rect.right() - 16.0, sheet_rect.bottom() - 48.0),
            );
            p.rect_filled(map_rect, 0.0, Color32::from_rgb(138, 192, 108));
            p.rect_stroke(map_rect, 0.0, Stroke::new(1.2, Color32::from_rgb(70, 95, 115)), egui::StrokeKind::Middle);

            let mc = map_rect.center();
            if self.drawing.sheet.grid_follow_map {
                let grid_stroke = Stroke::new(0.6, Color32::from_rgba_unmultiplied(40, 48, 56, 110));
                for gx in 1..8 {
                    let x = map_rect.left() + map_rect.width() * (gx as f32 / 8.0);
                    p.line_segment([Pos2::new(x, map_rect.top()), Pos2::new(x, map_rect.bottom())], grid_stroke);
                }
                for gy in 1..10 {
                    let y = map_rect.top() + map_rect.height() * (gy as f32 / 10.0);
                    p.line_segment([Pos2::new(map_rect.left(), y), Pos2::new(map_rect.right(), y)], grid_stroke);
                }
            }
            if LayoutBadgePlacer::should_render_site_names(
                self.drawing.sheet.scale_denominator,
                self.drawing.sheet.show_site_names_under_5000,
            ) {
                let sample_sites: Vec<(u32, String, f64, f64, f64, f64)> = self
                    .home_map
                    .real_survey
                    .heritage_polys
                    .iter()
                    .take(8)
                    .enumerate()
                    .map(|(idx, hp)| {
                        let ox = f64::from(mc.x) + ((idx as f64 % 3.0) - 1.0) * 38.0;
                        let oy = f64::from(mc.y) + ((idx as f64 / 3.0) - 1.0) * 28.0;
                        ((idx + 1) as u32, hp.site_name.clone(), ox, oy, 64.0, 14.0)
                    })
                    .collect();
                let placed = LayoutBadgePlacer::place_all_boxes(
                    &sample_sites,
                    Some((
                        f64::from(map_rect.left()),
                        f64::from(map_rect.top()),
                        f64::from(map_rect.right()),
                        f64::from(map_rect.bottom()),
                    )),
                );
                for nb in placed {
                    let br = Rect::from_center_size(
                        Pos2::new(nb.center_x_mm as f32, nb.center_y_mm as f32),
                        Vec2::new(nb.width_mm as f32, nb.height_mm as f32),
                    );
                    p.rect_filled(br, 2.5, Color32::from_rgba_unmultiplied(255, 255, 255, 230));
                    p.rect_stroke(br, 2.5, Stroke::new(0.8, Color32::from_rgb(45, 44, 40)), egui::StrokeKind::Middle);
                    p.text(
                        br.center(),
                        egui::Align2::CENTER_CENTER,
                        &nb.name,
                        egui::FontId::proportional(8.5),
                        Color32::BLACK,
                    );
                }
            } else {
                for i in 0..115 {
                    let fx = ((i * 37 + 19) % 100) as f32 / 100.0 - 0.5;
                    let fy = ((i * 61 + 43) % 100) as f32 / 100.0 - 0.5;
                    let dot_p = Pos2::new(mc.x + fx * map_rect.width() * 0.88, mc.y + fy * map_rect.height() * 0.88);
                    let col = if i % 3 == 0 { Color32::from_rgb(45, 165, 75) } else { Color32::from_rgb(198, 28, 118) };
                    p.circle_filled(dot_p, 2.7, col);
                    p.circle_stroke(dot_p, 2.7, Stroke::new(0.7, Color32::BLACK));
                }
            }
            let poly = vec![
                Pos2::new(mc.x - 4.0, mc.y - map_rect.height() * 0.28),
                Pos2::new(mc.x + 14.0, mc.y - map_rect.height() * 0.26),
                Pos2::new(mc.x + 8.0, mc.y + map_rect.height() * 0.28),
                Pos2::new(mc.x - 10.0, mc.y + map_rect.height() * 0.30),
            ];
            p.add(egui::Shape::convex_polygon(
                poly,
                Color32::from_rgba_unmultiplied(214, 92, 32, 95),
                Stroke::new(2.2, Color32::from_rgb(214, 78, 22)),
            ));
            let [scx, scy] = self.home_map.real_survey.survey_center;
            if self.home_map.real_survey.is_layer_visible("시굴격자") {
                let tr_s = Stroke::new(1.0, Color32::from_rgb(156, 62, 32));
                for tr in self.home_map.real_survey.trenches.iter().take(48) {
                    let pts: Vec<Pos2> = tr
                        .ring
                        .iter()
                        .map(|&pt| {
                            Pos2::new(
                                mc.x + ((pt[0] - scx) as f32) * 0.14,
                                mc.y - ((pt[1] - scy) as f32) * 0.14,
                            )
                        })
                        .collect();
                    p.add(egui::Shape::line(pts, tr_s));
                }
            }
            if self.home_map.real_survey.is_layer_visible("주변 500m") && !self.home_map.real_survey.buffer_500m.is_empty() {
                let buf_pts: Vec<Pos2> = self
                    .home_map
                    .real_survey
                    .buffer_500m
                    .iter()
                    .map(|&pt| {
                        Pos2::new(
                            mc.x + ((pt[0] - scx) as f32) * 0.14,
                            mc.y - ((pt[1] - scy) as f32) * 0.14,
                        )
                    })
                    .collect();
                p.add(egui::Shape::line(buf_pts, Stroke::new(1.6, Color32::from_rgb(184, 58, 24))));
            }
            for c_pt in [map_rect.left_top() + Vec2::new(12.0, 12.0), map_rect.right_top() + Vec2::new(-12.0, 12.0), map_rect.left_bottom() + Vec2::new(12.0, -12.0), map_rect.right_bottom() + Vec2::new(-12.0, -12.0)] {
                p.circle_filled(c_pt, 7.0, Color32::from_rgba_unmultiplied(255, 255, 255, 210));
                p.text(c_pt, egui::Align2::CENTER_CENTER, "↻", egui::FontId::proportional(9.5), Color32::from_rgb(55, 54, 50));
            }

            if self.drawing.sheet.legend_visible {
                let leg_r = Rect::from_min_size(
                    Pos2::new(map_rect.left() + 8.0, map_rect.bottom() - 56.0),
                    Vec2::new(82.0, 48.0),
                );
                p.rect_filled(leg_r, 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 232));
                p.rect_stroke(leg_r, 2.0, Stroke::new(0.8, Color32::BLACK), egui::StrokeKind::Middle);
                p.text(
                    Pos2::new(leg_r.left() + 6.0, leg_r.top() + 9.0),
                    egui::Align2::LEFT_CENTER,
                    &self.drawing.sheet.legend_title,
                    egui::FontId::proportional(8.5),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(leg_r.left() + 6.0, leg_r.top() + 24.0),
                    egui::Align2::LEFT_CENTER,
                    "□ 조사구역\n▨ 시굴격자",
                    egui::FontId::proportional(7.5),
                    Color32::from_rgb(45, 44, 40),
                );
            }
            if self.drawing.sheet.title_block_visible {
                let tb_r = Rect::from_min_size(
                    Pos2::new(map_rect.right() - 106.0, map_rect.bottom() - 40.0),
                    Vec2::new(98.0, 32.0),
                );
                p.rect_filled(tb_r, 1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 235));
                p.rect_stroke(tb_r, 1.5, Stroke::new(0.8, Color32::BLACK), egui::StrokeKind::Middle);
                p.text(
                    tb_r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}\n1:{}", self.project_title, self.drawing.sheet.scale_denominator),
                    egui::FontId::proportional(7.5),
                    Color32::BLACK,
                );
            }

            let sb_y = map_rect.bottom() + 18.0;
            p.line_segment([Pos2::new(map_rect.left() + 4.0, sb_y), Pos2::new(map_rect.left() + 92.0, sb_y)], Stroke::new(3.0, Color32::BLACK));
            p.text(Pos2::new(map_rect.left() + 48.0, sb_y + 11.0), egui::Align2::CENTER_CENTER, format!("축척 1 : {} ({})", self.drawing.sheet.scale_denominator, self.drawing.sheet.scale_bar_style.label_ko()), egui::FontId::proportional(9.0), Color32::BLACK);
            p.text(Pos2::new(map_rect.right() - 28.0, sb_y + 2.0), egui::Align2::RIGHT_CENTER, "동부원점(GRS80) · EPSG:5187", egui::FontId::proportional(8.5), Color32::BLACK);
            p.text(Pos2::new(map_rect.right() - 8.0, sb_y + 4.0), egui::Align2::CENTER_CENTER, self.drawing.sheet.north_arrow_style.symbol_glyph(), egui::FontId::proportional(10.0), Color32::BLACK);

            let bar_w = 504.0;
            let bar_r = Rect::from_center_size(Pos2::new(rect.center().x, rect.bottom() - 26.0), Vec2::new(bar_w, 42.0));
            p.rect_filled(bar_r, 6.0, Color32::WHITE);
            p.rect_stroke(bar_r, 6.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);
            let btns = [("■", "좌표점"), ("◎", "레이어를 가운데"), ("□", "지도 조정"), ("▣", "화면 맞춤"), ("✥", "항목 이동"), ("▯", "용지/방향"), ("≡", "도면 목록")];
            let bw = bar_w / (btns.len() as f32);
            for (i, (ico, lbl)) in btns.iter().enumerate() {
                let x0 = bar_r.left() + bw * (i as f32);
                if i > 0 {
                    p.line_segment([Pos2::new(x0, bar_r.top() + 4.0), Pos2::new(x0, bar_r.bottom() - 4.0)], Stroke::new(1.0, Color32::from_rgb(222, 220, 210)));
                }
                let cell_r = Rect::from_min_size(Pos2::new(x0, bar_r.top()), Vec2::new(bw, 42.0));
                p.text(Pos2::new(cell_r.center().x, cell_r.top() + 13.0), egui::Align2::CENTER_CENTER, *ico, egui::FontId::proportional(11.0), Color32::from_rgb(55, 54, 50));
                p.text(Pos2::new(cell_r.center().x, cell_r.bottom() - 10.0), egui::Align2::CENTER_CENTER, *lbl, egui::FontId::proportional(10.0), Color32::from_rgb(55, 54, 50));
                if ui.interact(cell_r, ui.id().with(("drawing_toolbar_btn", i)), egui::Sense::click()).clicked() {
                    match i {
                        0 => {
                            self.drawing.sheet.grid_enabled = !self.drawing.sheet.grid_enabled;
                            self.home_map.draw_guide_message = Some("도면 외곽 좌표점·좌표격자 표시를 전환했습니다.".to_owned());
                        }
                        1 => {
                            self.home_map.fit_to_survey();
                            self.home_map.draw_guide_message = Some("조사구역 레이어를 도면 칸 한가운데로 맞췄습니다.".to_owned());
                        }
                        2 => {
                            self.drawing.sheet.scale_denominator = 5_000;
                            self.home_map.draw_guide_message = Some("도면 지도 칸의 축척과 위치를 조정했습니다 (1:5,000).".to_owned());
                        }
                        3 => {
                            self.drawing.sheet.replace_sheet_chrome();
                            self.drawing.sheet.recover_orphaned_items();
                            self.home_map.draw_guide_message = Some("용지 화면 맞춤 및 하단 장식 띠(축척자·좌표계·방위) 정렬을 완료했습니다.".to_owned());
                        }
                        4 => {
                            self.home_map.draw_guide_message = Some("항목 이동 모드 — 범례·표제란·방위표를 끌어 자리를 옮기세요.".to_owned());
                        }
                        5 => {
                            self.modals.active = Some(ActiveModalDialog::PaperSetup);
                        }
                        _ => {
                            self.home_map.draw_guide_message = Some(format!("현재 도면 목록: 「{}」 (1/1쪽)", self.drawing.title_ko));
                        }
                    }
                }
            }
        });
    }
}
