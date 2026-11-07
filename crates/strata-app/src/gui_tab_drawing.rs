use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::viewport::format_scale_denominator;
use strata_layout::badge_placer::LayoutBadgePlacer;
use strata_layout::sheet_composer::{
    format_title_block_lines, nice_grid_interval_meters, nice_scale_bar_segment_meters,
    NorthArrowStyle, ScaleBarStyle,
};

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

            let hint_r = Rect::from_min_size(
                Pos2::new(rect.left() + 18.0, rect.top() + 16.0),
                Vec2::new(196.0, 30.0),
            );
            p.rect_filled(hint_r, 6.0, Color32::WHITE);
            p.rect_stroke(
                hint_r,
                6.0,
                Stroke::new(1.0, Color32::from_rgb(190, 188, 178)),
                egui::StrokeKind::Middle,
            );
            p.text(
                hint_r.center(),
                egui::Align2::CENTER_CENTER,
                "투명도·밝기: 조절할 레이어를 고르세요",
                egui::FontId::proportional(11.0),
                Color32::from_rgb(75, 74, 70),
            );

            // 1. 용지(mm) 비율에 맞춘 화면 용지 사각형 (절대 용지 밖으로 넘치지 않음)
            let paper_w_mm = (self.drawing.sheet.width_mm.max(100.0)) as f32;
            let paper_h_mm = (self.drawing.sheet.height_mm.max(100.0)) as f32;
            let avail_w = (rect.width() - 56.0).max(240.0);
            let avail_h = (rect.height() - 104.0).clamp(300.0, 660.0);
            let px_per_mm = (avail_w / paper_w_mm).min(avail_h / paper_h_mm).clamp(1.0, 3.2);
            let sheet_w = paper_w_mm * px_per_mm;
            let sheet_h = paper_h_mm * px_per_mm;
            let sheet_rect = Rect::from_center_size(
                Pos2::new(rect.center().x, rect.center().y - 18.0),
                Vec2::new(sheet_w, sheet_h),
            );
            p.rect_filled(
                sheet_rect.translate(Vec2::new(4.0, 5.0)),
                1.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 38),
            );
            p.rect_filled(sheet_rect, 1.0, Color32::WHITE);
            p.rect_stroke(
                sheet_rect,
                1.0,
                Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                egui::StrokeKind::Middle,
            );

            let sheet_p = p.with_clip_rect(sheet_rect);
            let mm_to_rect = |r: strata_layout::sheet_composer::SheetRectMm| -> Rect {
                Rect::from_min_size(
                    Pos2::new(
                        sheet_rect.left() + (r.x_mm as f32) * px_per_mm,
                        sheet_rect.top() + (r.y_mm as f32) * px_per_mm,
                    ),
                    Vec2::new(
                        (r.width_mm as f32) * px_per_mm,
                        (r.height_mm as f32) * px_per_mm,
                    ),
                )
            };
            let map_rect = mm_to_rect(self.drawing.sheet.map_frame);

            let rs = &self.home_map.real_survey;
            let map_bg = if rs.is_layer_visible("지형 음영")
                || rs.is_layer_visible("지표모델(DSM) 30m · Coperni...")
            {
                Color32::from_rgb(241, 244, 235)
            } else {
                Color32::from_rgb(252, 251, 248)
            };
            sheet_p.rect_filled(map_rect, 0.0, map_bg);

            // 2. 실제 조사 레이어(조사구역·시굴격자·지적도·국가유산·500m 버퍼·작도 유구)를 축척(scale_denominator)에 맞춰 렌더링
            let map_p = sheet_p.with_clip_rect(map_rect);
            let mc = map_rect.center();
            let [scx, scy] = rs.survey_center;
            let scale_denom = f64::from(self.drawing.sheet.scale_denominator.max(1));
            let px_per_meter = (1000.0 / scale_denom) * f64::from(px_per_mm);
            let w2m = |pt: [f64; 2]| -> Pos2 {
                Pos2::new(
                    mc.x + ((pt[0] - scx) * px_per_meter) as f32,
                    mc.y - ((pt[1] - scy) * px_per_meter) as f32,
                )
            };
            let half_w_m = f64::from(map_rect.width()) * 0.5 / px_per_meter.max(1e-6) + 250.0;
            let half_h_m = f64::from(map_rect.height()) * 0.5 / px_per_meter.max(1e-6) + 250.0;
            let in_view_bbox = |pt: [f64; 2]| -> bool {
                (pt[0] - scx).abs() <= half_w_m && (pt[1] - scy).abs() <= half_h_m
            };

            // (a) 실제 경주시 지적도 레이어
            if rs.is_layer_visible("지적도 · 조사 주변 5km") {
                let cad_stroke =
                    Stroke::new(0.55, Color32::from_rgba_unmultiplied(68, 66, 60, 140));
                for parcel in &rs.cadastral_parcels {
                    if !in_view_bbox(parcel.center) {
                        continue;
                    }
                    let pts: Vec<Pos2> = parcel.ring.iter().map(|&pt| w2m(pt)).collect();
                    map_p.add(egui::Shape::line(pts, cad_stroke));
                    if self.drawing.sheet.scale_denominator <= 2_500
                        && rs.is_label_visible("지적도 · 조사 주변 5km")
                        && !parcel.jibun.is_empty()
                    {
                        map_p.text(
                            w2m(parcel.center),
                            egui::Align2::CENTER_CENTER,
                            &parcel.jibun,
                            egui::FontId::proportional(7.5),
                            Color32::from_rgba_unmultiplied(55, 54, 50, 180),
                        );
                    }
                }
            }

            // (b) 실제 경주시 지정유산·매장유산유존지역·문화유적분포지도 레이어
            if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
                let green_fill = Color32::from_rgba_unmultiplied(39, 174, 96, 42);
                let green_stroke = Stroke::new(1.15, Color32::from_rgb(34, 148, 52));
                let mag_fill = Color32::from_rgba_unmultiplied(142, 68, 173, 42);
                let mag_stroke = Stroke::new(1.15, Color32::from_rgb(162, 32, 118));
                let mut visible_heritage: Vec<(usize, &crate::real_survey::RealHeritagePoly)> =
                    Vec::new();

                for h in &rs.heritage_polys {
                    if !rs.is_layer_visible(&h.layer_name) || !in_view_bbox(h.center) {
                        continue;
                    }
                    let pts: Vec<Pos2> = h.ring.iter().map(|&pt| w2m(pt)).collect();
                    if pts.len() >= 3 {
                        map_p.add(egui::Shape::convex_polygon(
                            pts.clone(),
                            if h.is_green { green_fill } else { mag_fill },
                            Stroke::NONE,
                        ));
                    }
                    map_p.add(egui::Shape::line(
                        pts,
                        if h.is_green { green_stroke } else { mag_stroke },
                    ));
                    if visible_heritage.len() < 16 {
                        let next_no = visible_heritage.len() + 1;
                        visible_heritage.push((next_no, h));
                    }
                }

                if LayoutBadgePlacer::should_render_site_names(
                    self.drawing.sheet.scale_denominator,
                    self.drawing.sheet.show_site_names_under_5000,
                ) {
                    let sample_sites: Vec<(u32, String, f64, f64, f64, f64)> = visible_heritage
                        .iter()
                        .take(10)
                        .map(|&(no, hp)| {
                            let sp = w2m(hp.center);
                            (
                                no as u32,
                                hp.site_name.clone(),
                                f64::from(sp.x),
                                f64::from(sp.y),
                                68.0,
                                14.0,
                            )
                        })
                        .collect();
                    let placed = LayoutBadgePlacer::place_all_boxes(
                        &sample_sites,
                        Some((
                            f64::from(map_rect.left() + 4.0),
                            f64::from(map_rect.top() + 4.0),
                            f64::from(map_rect.right() - 4.0),
                            f64::from(map_rect.bottom() - 4.0),
                        )),
                    );
                    for nb in placed {
                        let br = Rect::from_center_size(
                            Pos2::new(nb.center_x_mm as f32, nb.center_y_mm as f32),
                            Vec2::new(nb.width_mm as f32, nb.height_mm as f32),
                        );
                        map_p.rect_filled(
                            br,
                            2.5,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 232),
                        );
                        map_p.rect_stroke(
                            br,
                            2.5,
                            Stroke::new(0.8, Color32::from_rgb(35, 34, 30)),
                            egui::StrokeKind::Middle,
                        );
                        map_p.text(
                            br.center(),
                            egui::Align2::CENTER_CENTER,
                            &nb.name,
                            egui::FontId::proportional(8.0),
                            Color32::from_rgb(17, 24, 39),
                        );
                    }
                } else {
                    for (no, hp) in visible_heritage.into_iter().take(12) {
                        let sp = w2m(hp.center);
                        if !map_rect.shrink(8.0).contains(sp) {
                            continue;
                        }
                        let badge_col = if hp.is_green {
                            Color32::from_rgb(39, 174, 96)
                        } else {
                            Color32::from_rgb(142, 68, 173)
                        };
                        map_p.circle_filled(sp, 6.5, badge_col);
                        map_p.circle_stroke(sp, 6.5, Stroke::new(1.0, Color32::WHITE));
                        map_p.text(
                            sp,
                            egui::Align2::CENTER_CENTER,
                            no.to_string(),
                            egui::FontId::proportional(8.0),
                            Color32::WHITE,
                        );
                    }
                }
            }

            // (c) 실제 주변 500m / 1000m 버퍼 및 거리 라벨
            if rs.is_layer_visible("주변 500m") && !rs.buffer_500m.is_empty() {
                let buf_pts: Vec<Pos2> = rs.buffer_500m.iter().map(|&pt| w2m(pt)).collect();
                map_p.add(egui::Shape::line(
                    buf_pts,
                    Stroke::new(1.6, Color32::from_rgb(184, 58, 24)),
                ));
            }
            if rs.show_buffer_1000m && !rs.buffer_1000m.is_empty() {
                let buf1000_pts: Vec<Pos2> = rs.buffer_1000m.iter().map(|&pt| w2m(pt)).collect();
                map_p.add(egui::Shape::line(
                    buf1000_pts,
                    Stroke::new(1.3, Color32::from_rgb(148, 42, 18)),
                ));
            }
            if rs.is_layer_visible("주변 500m 거리") && rs.is_label_visible("주변 500m 거리") {
                for (lbl, pt) in &rs.dist_labels_500m {
                    let bp = w2m(*pt);
                    if map_rect.shrink(12.0).contains(bp) {
                        let b_rect = Rect::from_center_size(bp, Vec2::new(32.0, 13.0));
                        map_p.rect_filled(
                            b_rect,
                            2.5,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 230),
                        );
                        map_p.text(
                            b_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(8.0),
                            Color32::from_rgb(184, 58, 24),
                        );
                    }
                }
            }

            // (d) 실제 조사구역 (survey_rings)
            if rs.is_layer_visible("조사구역") {
                let fill_col = Color32::from_rgba_unmultiplied(214, 92, 32, 82);
                let stroke = Stroke::new(1.9, Color32::from_rgb(214, 78, 22));
                for ring in &rs.survey_rings {
                    let pts: Vec<Pos2> = ring.iter().map(|&pt| w2m(pt)).collect();
                    if pts.len() >= 3 {
                        map_p.add(egui::Shape::convex_polygon(
                            pts.clone(),
                            fill_col,
                            Stroke::NONE,
                        ));
                    }
                    map_p.add(egui::Shape::line(pts, stroke));
                }
                if rs.is_label_visible("조사구역") {
                    let sp = w2m(rs.survey_center);
                    let lbl_r = Rect::from_center_size(sp, Vec2::new(64.0, 15.0));
                    map_p.rect_filled(
                        lbl_r,
                        2.5,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 225),
                    );
                    map_p.text(
                        lbl_r.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.0} m²", rs.survey_area_m2),
                        egui::FontId::proportional(8.5),
                        Color32::from_rgb(17, 24, 39),
                    );
                }
            }

            // (e) 실제 시굴격자 (trenches 232개 전수)
            if rs.is_layer_visible("시굴격자") {
                let tr_fill = Color32::from_rgba_unmultiplied(156, 62, 32, 65);
                let tr_stroke = Stroke::new(1.0, Color32::from_rgb(156, 62, 32));
                for tr in &rs.trenches {
                    if !in_view_bbox(tr.center) {
                        continue;
                    }
                    let pts: Vec<Pos2> = tr.ring.iter().map(|&pt| w2m(pt)).collect();
                    if pts.len() >= 3 {
                        map_p.add(egui::Shape::convex_polygon(
                            pts.clone(),
                            tr_fill,
                            Stroke::NONE,
                        ));
                    }
                    map_p.add(egui::Shape::line(pts, tr_stroke));
                }
            }

            // (f) 작도된 유구 다각형 (scene.polygons)
            for poly in &self.home_map.scene.polygons {
                let pts: Vec<Pos2> = poly.world_pts.iter().map(|&pt| w2m(pt)).collect();
                if pts.len() >= 3 {
                    map_p.add(egui::Shape::convex_polygon(
                        pts,
                        Color32::from_rgba_unmultiplied(181, 87, 58, 70),
                        Stroke::new(1.4, Color32::from_rgb(142, 56, 32)),
                    ));
                }
            }

            // (g) 도곽 좌표격자 및 TM 좌표 주기 (C++ LayoutService::applySurveyFrameGrid)
            if self.drawing.sheet.grid_enabled || self.drawing.sheet.grid_follow_map {
                let step_m = if self.drawing.sheet.grid_follow_map {
                    20.0
                } else {
                    nice_grid_interval_meters(scale_denom, self.drawing.sheet.map_frame.width_mm)
                };
                let step_px = (step_m * px_per_meter) as f32;
                if step_px >= 18.0 {
                    let min_wx = scx - f64::from(map_rect.width() * 0.5) / px_per_meter.max(1e-6);
                    let max_wx = scx + f64::from(map_rect.width() * 0.5) / px_per_meter.max(1e-6);
                    let min_wy = scy - f64::from(map_rect.height() * 0.5) / px_per_meter.max(1e-6);
                    let max_wy = scy + f64::from(map_rect.height() * 0.5) / px_per_meter.max(1e-6);
                    let first_gx = (min_wx / step_m).ceil() as i64;
                    let last_gx = (max_wx / step_m).floor() as i64;
                    let first_gy = (min_wy / step_m).ceil() as i64;
                    let last_gy = (max_wy / step_m).floor() as i64;
                    let grid_stroke =
                        Stroke::new(0.5, Color32::from_rgba_unmultiplied(17, 24, 39, 95));

                    for ix in (first_gx..=last_gx).take(24) {
                        let wx = (ix as f64) * step_m;
                        let sx = w2m([wx, scy]).x;
                        if sx > map_rect.left() + 4.0 && sx < map_rect.right() - 4.0 {
                            if self.drawing.sheet.grid_follow_map {
                                map_p.line_segment(
                                    [
                                        Pos2::new(sx, map_rect.top()),
                                        Pos2::new(sx, map_rect.bottom()),
                                    ],
                                    grid_stroke,
                                );
                            } else {
                                map_p.line_segment(
                                    [
                                        Pos2::new(sx, map_rect.top()),
                                        Pos2::new(sx, map_rect.top() + 4.0),
                                    ],
                                    grid_stroke,
                                );
                                map_p.line_segment(
                                    [
                                        Pos2::new(sx, map_rect.bottom() - 4.0),
                                        Pos2::new(sx, map_rect.bottom()),
                                    ],
                                    grid_stroke,
                                );
                            }
                            if step_px >= 42.0 {
                                sheet_p.text(
                                    Pos2::new(sx, map_rect.top() - 5.0),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{wx:.0}"),
                                    egui::FontId::proportional(6.5),
                                    Color32::from_rgb(31, 41, 55),
                                );
                            }
                        }
                    }
                    for iy in (first_gy..=last_gy).take(24) {
                        let wy = (iy as f64) * step_m;
                        let sy = w2m([scx, wy]).y;
                        if sy > map_rect.top() + 4.0 && sy < map_rect.bottom() - 4.0 {
                            if self.drawing.sheet.grid_follow_map {
                                map_p.line_segment(
                                    [
                                        Pos2::new(map_rect.left(), sy),
                                        Pos2::new(map_rect.right(), sy),
                                    ],
                                    grid_stroke,
                                );
                            } else {
                                for ix in (first_gx..=last_gx).take(24) {
                                    let sx = w2m([(ix as f64) * step_m, wy]).x;
                                    if sx > map_rect.left() + 4.0 && sx < map_rect.right() - 4.0 {
                                        map_p.line_segment(
                                            [Pos2::new(sx - 2.5, sy), Pos2::new(sx + 2.5, sy)],
                                            grid_stroke,
                                        );
                                        map_p.line_segment(
                                            [Pos2::new(sx, sy - 2.5), Pos2::new(sx, sy + 2.5)],
                                            grid_stroke,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let ink = Color32::from_rgb(17, 24, 39);
            sheet_p.rect_stroke(
                map_rect,
                0.0,
                Stroke::new(1.3, ink),
                egui::StrokeKind::Middle,
            );

            // 3. 용지 내 4요소 실물 조판: (1) 범례, (2) 표제란, (3) 축척막대·축척글·좌표계, (4) 방위표(도북 화살)

            // (1) 범례 (legend_visible 토글 연동, KaDrawingStudio::defaultItemRect(kIdLegend))
            if self.drawing.sheet.legend_visible {
                let mut legend_items: Vec<(Color32, Color32, &str)> = Vec::new();
                if rs.is_layer_visible("조사구역") {
                    legend_items.push((
                        Color32::from_rgba_unmultiplied(214, 92, 32, 95),
                        Color32::from_rgb(214, 78, 22),
                        "조사구역",
                    ));
                }
                if rs.is_layer_visible("시굴격자") {
                    legend_items.push((
                        Color32::from_rgba_unmultiplied(156, 62, 32, 80),
                        Color32::from_rgb(156, 62, 32),
                        "시굴격자",
                    ));
                }
                if rs.is_layer_visible("주변 500m") {
                    legend_items.push((
                        Color32::TRANSPARENT,
                        Color32::from_rgb(184, 58, 24),
                        "주변 500m",
                    ));
                }
                if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
                    if rs.is_layer_visible("국가지정유산") || rs.is_layer_visible("시도지정유산")
                    {
                        legend_items.push((
                            Color32::from_rgba_unmultiplied(142, 68, 173, 60),
                            Color32::from_rgb(162, 32, 118),
                            "지정유산",
                        ));
                    }
                    if rs.is_layer_visible("국가지정유산보호구역")
                        || rs.is_layer_visible("시도지정유산보호구역")
                    {
                        legend_items.push((
                            Color32::from_rgba_unmultiplied(39, 174, 96, 60),
                            Color32::from_rgb(34, 148, 52),
                            "보호·유존지역",
                        ));
                    }
                }
                if rs.is_layer_visible("지적도 · 조사 주변 5km") {
                    legend_items.push((
                        Color32::TRANSPARENT,
                        Color32::from_rgb(68, 66, 60),
                        "지적도",
                    ));
                }

                let base_leg_r = mm_to_rect(self.drawing.sheet.legend_rect);
                let row_h = 12.5_f32;
                let leg_w = base_leg_r.width().clamp(78.0, map_rect.width() * 0.42);
                let leg_h = (22.0 + (legend_items.len() as f32) * row_h)
                    .clamp(36.0, base_leg_r.height().max(72.0));
                let leg_r = Rect::from_min_size(
                    Pos2::new(
                        (map_rect.right() - leg_w - 6.0).max(map_rect.left() + 4.0),
                        base_leg_r.top(),
                    ),
                    Vec2::new(leg_w, leg_h),
                );
                sheet_p.rect_filled(
                    leg_r,
                    1.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 245),
                );
                sheet_p.rect_stroke(leg_r, 1.0, Stroke::new(0.9, ink), egui::StrokeKind::Middle);
                let title_pt = (self.drawing.sheet.legend_font_pt as f32).clamp(8.0, 11.5);
                sheet_p.text(
                    Pos2::new(leg_r.left() + 6.0, leg_r.top() + 10.0),
                    egui::Align2::LEFT_CENTER,
                    &self.drawing.sheet.legend_title,
                    egui::FontId::proportional(title_pt),
                    ink,
                );
                sheet_p.line_segment(
                    [
                        Pos2::new(leg_r.left() + 4.0, leg_r.top() + 18.0),
                        Pos2::new(leg_r.right() - 4.0, leg_r.top() + 18.0),
                    ],
                    Stroke::new(0.5, Color32::from_rgb(200, 198, 190)),
                );
                for (idx, (fill_c, stroke_c, name)) in legend_items.into_iter().enumerate() {
                    let ry = leg_r.top() + 25.5 + (idx as f32) * row_h;
                    if ry + 4.0 > leg_r.bottom() {
                        break;
                    }
                    let sw_r = Rect::from_min_size(
                        Pos2::new(leg_r.left() + 6.0, ry - 4.0),
                        Vec2::new(11.0, 7.5),
                    );
                    if fill_c != Color32::TRANSPARENT {
                        sheet_p.rect_filled(sw_r, 1.0, fill_c);
                    }
                    sheet_p.rect_stroke(
                        sw_r,
                        1.0,
                        Stroke::new(1.0, stroke_c),
                        egui::StrokeKind::Middle,
                    );
                    sheet_p.text(
                        Pos2::new(sw_r.right() + 5.0, ry),
                        egui::Align2::LEFT_CENTER,
                        name,
                        egui::FontId::proportional((title_pt - 1.5).max(7.2)),
                        ink,
                    );
                }
            }

            // (2) 표제란 (title_block_visible 토글 연동, KaTitleBlock.cpp:57-111 1:1)
            // 도면명 / 조사명 / 유적명 / 축  척 / 작성일 (빈 값은 ―, 위치는 chrome.crs 아래 1.5mm 또는 지도 우하단)
            if self.drawing.sheet.title_block_visible {
                let raw_tb_r = mm_to_rect(self.drawing.sheet.title_block_rect);
                let tb_w = raw_tb_r.width().max(118.0);
                let tb_h = raw_tb_r.height().max(48.0);
                let tb_x = raw_tb_r
                    .left()
                    .clamp(sheet_rect.left() + 4.0, (sheet_rect.right() - tb_w - 4.0).max(sheet_rect.left() + 4.0));
                let tb_y = raw_tb_r
                    .top()
                    .clamp(sheet_rect.top() + 4.0, (sheet_rect.bottom() - tb_h - 3.0).max(sheet_rect.top() + 4.0));
                let tb_r = Rect::from_min_size(Pos2::new(tb_x, tb_y), Vec2::new(tb_w, tb_h));
                sheet_p.rect_filled(tb_r, 0.0, Color32::WHITE);
                sheet_p.rect_stroke(tb_r, 0.0, Stroke::new(0.85, ink), egui::StrokeKind::Middle);
                let tb_lines = format_title_block_lines(
                    &self.drawing.title_ko,
                    &rs.survey_name,
                    &rs.site_name,
                    self.drawing.sheet.scale_denominator,
                    "2026-11-08",
                );
                let line_step = ((tb_r.height() - 6.0) / 5.0).max(8.2);
                for (li, line) in tb_lines.iter().enumerate() {
                    let ly = tb_r.top() + 3.0 + ((li as f32) + 0.5) * line_step;
                    sheet_p.text(
                        Pos2::new(tb_r.left() + 4.5, ly),
                        egui::Align2::LEFT_CENTER,
                        line,
                        egui::FontId::proportional(7.2),
                        ink,
                    );
                }
            }

            // (3) 하단 장식띠: LayoutService::standardSheetChrome 사각형 (scale_bar, scale_label, crs, north)
            let chrome_north_r = mm_to_rect(self.drawing.sheet.north_arrow_rect);
            let size_scale = (self.drawing.sheet.north_arrow_size_mm as f32 / 28.0).clamp(0.7, 1.35);
            let north_sz = (chrome_north_r.width() * size_scale)
                .clamp(22.0, (sheet_rect.bottom() - chrome_north_r.top() - 4.0).max(22.0));
            let north_rect = Rect::from_min_size(
                Pos2::new(
                    (chrome_north_r.right() - north_sz).max(sheet_rect.left() + 4.0),
                    chrome_north_r.top(),
                ),
                Vec2::new(north_sz, north_sz),
            );
            let chrome_crs_r = mm_to_rect(self.drawing.sheet.crs_label_rect);
            let chrome_bar_r = mm_to_rect(self.drawing.sheet.scale_bar_rect);
            let chrome_lbl_r = mm_to_rect(self.drawing.sheet.scale_text_rect);

            let seg_m = nice_scale_bar_segment_meters(
                self.drawing.sheet.map_frame.width_mm,
                scale_denom,
                4,
            );
            let seg_count = 4usize;
            let max_bar_w = (chrome_crs_r.left() - chrome_bar_r.left() - 8.0).max(64.0);
            let raw_bar_w = ((seg_m * (seg_count as f64) * px_per_meter) as f32)
                .clamp(64.0, max_bar_w.min(chrome_bar_r.width().max(84.0)));
            let seg_w = raw_bar_w / (seg_count as f32);
            let bar_x0 = chrome_bar_r.left();
            let bar_y0 = chrome_bar_r.top() + 6.0;
            let bar_h = 5.5_f32;

            match self.drawing.sheet.scale_bar_style {
                ScaleBarStyle::DoubleBox => {
                    let half_h = bar_h * 0.5;
                    for row in 0..2 {
                        for i in 0..seg_count {
                            let r_box = Rect::from_min_size(
                                Pos2::new(
                                    bar_x0 + (i as f32) * seg_w,
                                    bar_y0 + (row as f32) * half_h,
                                ),
                                Vec2::new(seg_w, half_h),
                            );
                            let dark = (i + row) % 2 == 0;
                            sheet_p.rect_filled(
                                r_box,
                                0.0,
                                if dark { ink } else { Color32::WHITE },
                            );
                            sheet_p.rect_stroke(
                                r_box,
                                0.0,
                                Stroke::new(0.7, ink),
                                egui::StrokeKind::Middle,
                            );
                        }
                    }
                }
                ScaleBarStyle::SingleBox => {
                    for i in 0..seg_count {
                        let r_box = Rect::from_min_size(
                            Pos2::new(bar_x0 + (i as f32) * seg_w, bar_y0),
                            Vec2::new(seg_w, bar_h),
                        );
                        sheet_p.rect_filled(
                            r_box,
                            0.0,
                            if i % 2 == 0 { ink } else { Color32::WHITE },
                        );
                        sheet_p.rect_stroke(
                            r_box,
                            0.0,
                            Stroke::new(0.8, ink),
                            egui::StrokeKind::Middle,
                        );
                    }
                }
                ScaleBarStyle::SteppedTicks => {
                    let by = bar_y0 + bar_h;
                    sheet_p.line_segment(
                        [Pos2::new(bar_x0, by), Pos2::new(bar_x0 + raw_bar_w, by)],
                        Stroke::new(1.2, ink),
                    );
                    for i in 0..=seg_count {
                        let tx = bar_x0 + (i as f32) * seg_w;
                        sheet_p.line_segment(
                            [Pos2::new(tx, by), Pos2::new(tx, bar_y0)],
                            Stroke::new(1.0, ink),
                        );
                    }
                }
            }
            for i in [0usize, 2, seg_count] {
                let tx = bar_x0 + (i as f32) * seg_w;
                let dist_val = seg_m * (i as f64);
                let dist_lbl = if i == seg_count {
                    if dist_val >= 1.0 {
                        format!("{dist_val:.0} m")
                    } else {
                        format!("{dist_val:.1} m")
                    }
                } else if dist_val >= 1.0 {
                    format!("{dist_val:.0}")
                } else {
                    format!("{dist_val:.1}")
                };
                sheet_p.text(
                    Pos2::new(tx, bar_y0 - 4.0),
                    egui::Align2::CENTER_CENTER,
                    dist_lbl,
                    egui::FontId::proportional(6.8),
                    ink,
                );
            }
            sheet_p.text(
                Pos2::new(bar_x0 + raw_bar_w * 0.5, chrome_lbl_r.center().y.max(bar_y0 + bar_h + 7.0)),
                egui::Align2::CENTER_CENTER,
                format!(
                    "축척 1 : {}",
                    format_scale_denominator(f64::from(self.drawing.sheet.scale_denominator))
                ),
                egui::FontId::proportional(8.0),
                ink,
            );

            // 좌표계 라벨 (chrome.crs)
            sheet_p.text(
                Pos2::new(chrome_crs_r.left(), chrome_crs_r.center().y),
                egui::Align2::LEFT_CENTER,
                "동부원점(GRS80) · EPSG:5187",
                egui::FontId::proportional(7.2),
                ink,
            );

            // (4) 방위표 (도북 화살, KaDrawingNorth::paintMark 1:1)
            let nc = north_rect.center();
            let nr = north_rect.width() * 0.44;
            match self.drawing.sheet.north_arrow_style {
                NorthArrowStyle::LetterOnly => {
                    let tri = vec![
                        Pos2::new(nc.x, north_rect.top() + 2.0),
                        Pos2::new(nc.x + nr * 0.55, nc.y),
                        Pos2::new(nc.x - nr * 0.55, nc.y),
                    ];
                    sheet_p.add(egui::Shape::convex_polygon(tri, ink, Stroke::NONE));
                    sheet_p.text(
                        Pos2::new(nc.x, nc.y + nr * 0.52),
                        egui::Align2::CENTER_CENTER,
                        "N",
                        egui::FontId::proportional(9.5),
                        ink,
                    );
                }
                NorthArrowStyle::SingleArrow => {
                    sheet_p.text(
                        Pos2::new(nc.x, north_rect.top() + 4.5),
                        egui::Align2::CENTER_CENTER,
                        "N",
                        egui::FontId::proportional(8.5),
                        ink,
                    );
                    let tip = Pos2::new(nc.x, north_rect.top() + 9.5);
                    let tail = Pos2::new(nc.x, north_rect.bottom() - 4.0);
                    let wing_y = north_rect.bottom() - 1.5;
                    let hw = nr * 0.42;
                    let left_half = vec![tip, Pos2::new(nc.x - hw, wing_y), tail];
                    let right_half = vec![tip, tail, Pos2::new(nc.x + hw, wing_y)];
                    sheet_p.add(egui::Shape::convex_polygon(
                        left_half,
                        ink,
                        Stroke::new(0.8, ink),
                    ));
                    sheet_p.add(egui::Shape::convex_polygon(
                        right_half,
                        Color32::WHITE,
                        Stroke::new(0.8, ink),
                    ));
                }
                NorthArrowStyle::CompassRose => {
                    let c_pos = Pos2::new(nc.x, nc.y + 2.0);
                    sheet_p.circle_stroke(c_pos, nr * 0.78, Stroke::new(1.1, ink));
                    sheet_p.circle_stroke(c_pos, nr * 0.46, Stroke::new(0.8, ink));
                    let needle = vec![
                        Pos2::new(nc.x, north_rect.top() + 1.5),
                        Pos2::new(nc.x + 3.2, c_pos.y - nr * 0.45),
                        Pos2::new(nc.x - 3.2, c_pos.y - nr * 0.45),
                    ];
                    sheet_p.add(egui::Shape::convex_polygon(needle, ink, Stroke::NONE));
                    sheet_p.text(
                        c_pos,
                        egui::Align2::CENTER_CENTER,
                        "N",
                        egui::FontId::proportional(7.5),
                        ink,
                    );
                }
                NorthArrowStyle::WindRose => {
                    let v_diamond = vec![
                        Pos2::new(nc.x, north_rect.top() + 2.0),
                        Pos2::new(nc.x + 3.0, nc.y + 1.0),
                        Pos2::new(nc.x, north_rect.bottom() - 2.0),
                        Pos2::new(nc.x - 3.0, nc.y + 1.0),
                    ];
                    let h_diamond = vec![
                        Pos2::new(north_rect.left() + 2.0, nc.y + 1.0),
                        Pos2::new(nc.x, nc.y - 1.8),
                        Pos2::new(north_rect.right() - 2.0, nc.y + 1.0),
                        Pos2::new(nc.x, nc.y + 3.8),
                    ];
                    sheet_p.add(egui::Shape::convex_polygon(
                        h_diamond,
                        Color32::from_rgb(75, 85, 99),
                        Stroke::NONE,
                    ));
                    sheet_p.add(egui::Shape::convex_polygon(v_diamond, ink, Stroke::NONE));
                    sheet_p.text(
                        Pos2::new(nc.x + 6.5, north_rect.top() + 5.0),
                        egui::Align2::CENTER_CENTER,
                        "N",
                        egui::FontId::proportional(7.5),
                        ink,
                    );
                }
            }

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
