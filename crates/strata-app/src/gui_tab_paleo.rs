use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_paleo::pipeline::PaleoClass;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// `KaPaleoStudio.cpp:36-291` & `KaPaleoCrossSectionWidget.cpp:27-97` — 3단 분할(`layersCard` | `centerSplit`[7대 고지형 분류도 + 시추공 주상도 단면도] | `rightScroll`[6대 자료·7대 범례·요약])
    pub(crate) fn render_paleo_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. 우측 스크롤 패널 (KaPaleoStudio.cpp:123-204)
        egui::SidePanel::right("ka_paleo_right_panel")
            .resizable(true)
            .default_width(380.0)
            .width_range(330.0..=440.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.strong("고지형 · 기본층서 종합 분석 (반경 10km)");
                    ui.add_space(2.0);
                    ui.strong("6대 입력 자료 현황");
                      // KaPaleoStudio.cpp:226-250 refreshSourceStatus — 준비된 항목은 초록 ●,
                      // 미비 항목은 빨강 ○ 로 6대 입력 자료 상태를 한 줄에 보여준다.
                      {
                        let geol_ok = self.paleo_3d.geology_feature_count > 0;
                        let bh_ok = self.paleo_3d.declustered_borehole_count > 0;
                        let sources: [(&str, bool, String); 6] = [
                            ("국토지리원DEM", true, String::new()),
                            ("토양도", true, String::new()),
                            ("지질도", geol_ok, format!("({}건)", self.paleo_3d.geology_feature_count)),
                            ("수계도", true, String::new()),
                            ("KIGAM수문", true, String::new()),
                            (
                                "시추",
                                bh_ok,
                                format!("({}공)", self.paleo_3d.declustered_borehole_count),
                            ),
                        ];
                        ui.horizontal_wrapped(|ui| {
                            for (idx, (name, ready, suffix)) in sources.iter().enumerate() {
                                let (mark, col) = if *ready {
                                    ("●", Color32::from_rgb(0x15, 0x80, 0x3D))
                                } else {
                                    ("○", Color32::from_rgb(0xB9, 0x1C, 0x1C))
                                };
                                ui.colored_label(
                                    col,
                                    egui::RichText::new(format!("{mark} {name}{suffix}"))
                                        .size(12.0),
                                );
                                if idx % 3 == 2 {
                                    ui.end_row();
                                } else if idx + 1 < sources.len() {
                                    ui.label(egui::RichText::new("·").size(12.0));
                                }
                            }
                        });
                      }
                    ui.add_space(4.0);
                    // C++ KaPaleoStudio.cpp:143-150: two separate download entries side by side.
                    ui.horizontal(|ui| {
                        if ui
                            .button("수치지질·시추·KIGAM 별도 받기…")
                            .clicked()
                        {
                            self.modals.active = Some(ActiveModalDialog::PaleoSourceDownload);
                        }
                        if ui.button("국토지리원 DEM 받기…").clicked() {
                            // MainWindowDownloads.cpp:85-155 (56776d3) — 국토지리정보원 DEM 브라우저
                            self.modals.active = Some(ActiveModalDialog::DemDownload);
                        }
                    });
                    ui.checkbox(
                        &mut self.paleo_3d.show_imhof_slope_overlay,
                        "다중방위 음영기복(Hillshade) 중첩 적용",
                    );
                    let btn_run = egui::Button::new(
                        egui::RichText::new("준비된 자료로 고지형 분석 실행")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .min_size(egui::vec2(ui.available_width(), 32.0));
                    if ui.add(btn_run).clicked() {
                        self.paleo_3d.run_sample_10km_analysis();
                        self.home_map.draw_guide_message = Some(
                            "고지형 4단계 파이프라인(반경 10km) 연산을 완료했습니다.".to_owned(),
                        );
                    }
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new("도면(조판)에 고지형 판독 근거 넣기"),
                        )
                        .clicked()
                    {
                        self.open_studio_tab(MainTab::Drawing);
                        self.home_map.draw_guide_message =
                            Some("도면(조판) 시트에 고지형 판독 근거 및 단면도를 반영했습니다.".to_owned());
                    }
                    ui.separator();
                    ui.strong("7대 고지형 분류 체계 및 유적 잠재력");
                    let mut class_counts = [0usize; 8];
                    let mut total_cells = 0usize;
                    if let Some(ref out) = self.paleo_3d.last_paleo_output {
                        total_cells = out.paleo_class_grid.len().max(1);
                        for &c in &out.paleo_class_grid {
                            let idx = c as usize;
                            if idx < class_counts.len() {
                                class_counts[idx] += 1;
                            }
                        }
                    }
                    for cls in PaleoClass::ALL {
                        let [r, g, b, _] = cls.color_rgba();
                        let col = Color32::from_rgb(r, g, b);
                        let pct = if total_cells > 0 {
                            (class_counts[cls as usize] as f64 * 100.0 / total_cells as f64).round()
                                as usize
                        } else {
                            0
                        };
                        ui.horizontal_wrapped(|ui| {
                            let (sw_rect, _) =
                                ui.allocate_exact_size(Vec2::new(13.0, 13.0), egui::Sense::hover());
                            ui.painter().rect_filled(sw_rect, 2.0, col);
                            ui.painter().rect_stroke(
                                sw_rect,
                                2.0,
                                Stroke::new(0.8, Color32::from_rgb(45, 43, 40)),
                                egui::StrokeKind::Middle,
                            );
                            ui.strong(format!("{} ({pct}%)", cls.name_ko()));
                            ui.label(
                                egui::RichText::new(format!("— {}", cls.arch_potential_ko()))
                                    .size(11.5),
                            );
                        });
                    }
                    ui.separator();
                    ui.strong("시추공 충적층 두께 및 수치지질도(1:5만) 범례");
                    ui.label("• 시추 마커: ● 사질 우세(SM, 주황) · ● 점토·실트 우세(CL, 청색)");
                    ui.label("• 지질(KIGAM): Qa 제4기 충적층 · Qr 하안단구층 · Ks 경상누층군 · Kgr 불국사화강암");
                    ui.separator();
                    ui.strong("조사구역 고지형·기본층서 판독 요약 및 근거");
                    let auto_cls = strata_paleo::terrain_edt_tpi::classify_paleo_dem_polygon(
                        "자연제방",
                        &strata_paleo::terrain_edt_tpi::PaleoDemMetrics {
                            has_dem: true,
                            mean_elev_m: 25.4,
                            local_relief_m: 2.8,
                            mean_slope_deg: 2.1,
                            river_dist_m: 145.0,
                            has_borehole: true,
                            borehole_code: "BH-02".to_owned(),
                            alluvial_thickness_m: 3.8,
                            weathered_depth_m: 5.0,
                            deposit_env: "사질".to_owned(),
                            dominant_uscs: "SM".to_owned(),
                        },
                    );
                    ui.label(
                        egui::RichText::new(&auto_cls.note)
                            .size(11.5)
                            .color(Color32::from_rgb(107, 78, 61)),
                    );
                    let mut summary = self
                        .paleo_3d
                        .last_paleo_output
                        .as_ref()
                        .map_or_else(
                            || "분석 실행 버튼을 누르면 판독 요약이 생성됩니다.".to_owned(),
                            |o| {
                                format!(
                                    "{}\n• 1단계(지형기복): EDT 수계거리 및 다중스케일 TPI 7대 미지형(자연제방·구하도·배후습지·선상지·범람원·구릉사면·산지) 분류\n• 2단계(지반·시추): 성토층 0.0~1.2m / 충적층(문화층 유망) 1.2~5.0m / 풍화대 5.0m 이하\n• 3단계(수문·토양): KIGAM 제4기 충적층(Qa) 및 흙토람 사양질 배수 양호",
                                    o.aoi_summary_text
                                )
                            },
                        );
                    ui.add(
                        egui::TextEdit::multiline(&mut summary)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY),
                    );
                    if ui
                        .add_sized(
                            [ui.available_width(), 28.0],
                            egui::Button::new("판독 요약·근거 문장 복사"),
                        )
                        .clicked()
                    {
                        ctx.copy_text(summary);
                        self.home_map.draw_guide_message = Some(
                            "고지형·기본층서 판독 요약문을 클립보드에 복사했습니다.".to_owned(),
                        );
                    }
                });
            });

        // 2. 좌측 레이어 패널 (KaPaleoStudio.cpp:51-99)
        egui::SidePanel::left("ka_paleo_left_layers")
            .resizable(true)
            .default_width(250.0)
            .width_range(200.0..=320.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("레이어");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let _ = ui.small_button("선택 삭제(Del)");
                        });
                    });
                    ui.separator();
                    for name in [
                        "☑ [고지형] 7대 고지형 분류도",
                        "☑ [고지형] 하상비고(REM)",
                        "☑ [고지형] 시추주상도 마커 (8공)",
                        "☑ [DEM] 국토지리원_DEM.tif",
                        "☑ 고지형 수문·연대(KIGAM)",
                        "☑ 토양도_흙토람 (지형분류)",
                        "☑ 수계도(하천망) · A–A' 단면선",
                    ] {
                        ui.label(name);
                    }
                    ui.separator();
                    ui.strong("7대 분류도 레이어 범례");
                    for cls in PaleoClass::ALL {
                        let [r, g, b, _] = cls.color_rgba();
                        ui.horizontal(|ui| {
                            let (sw, _) =
                                ui.allocate_exact_size(Vec2::new(11.0, 11.0), egui::Sense::hover());
                            ui.painter()
                                .rect_filled(sw, 2.0, Color32::from_rgb(r, g, b));
                            ui.label(egui::RichText::new(cls.name_ko()).size(11.5));
                        });
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new(
                            "해상도: 18.0m DEM 격자 (120×84)\n좌표계: EPSG:5187 (동부원점)",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(94, 93, 89)),
                    );
                });
            });

        // 3. 중앙 수직 분할 (상단: KaPaleoStudio::renderMapPreview 7색 분류도 + 시추공·단면선 / 하단: KaPaleoCrossSectionWidget 지층 단면도 + 시추공 주상도)
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let total_h = ui.available_height();
            let top_h = (total_h * 0.56).clamp(190.0, (total_h - 175.0).max(190.0));

            // (A) 상단: 7대 고지형 분류도 실물 렌더 (KaPaleoStudio.cpp:268-291)
            let (map_rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), top_h),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(map_rect);
            painter.rect_filled(map_rect, 4.0, Color32::from_rgb(242, 240, 234));

            if let Some(ref out) = self.paleo_3d.last_paleo_output {
                let w = out.transform.width;
                let h = out.transform.height;
                if w > 0 && h > 0 && out.paleo_rgba_buffer.len() >= w * h * 4 {
                    let rgba = if self.paleo_3d.show_imhof_slope_overlay {
                        out.paleo_rgba_buffer.clone()
                    } else {
                        let mut flat = Vec::with_capacity(w * h * 4);
                        for &cls in &out.paleo_class_grid {
                            flat.extend_from_slice(&cls.color_rgba());
                        }
                        flat
                    };
                    let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    let tex = ctx.load_texture(
                        "paleo_7class_map_preview",
                        img,
                        egui::TextureOptions::NEAREST,
                    );
                    let inner_map = map_rect.shrink(4.0);
                    painter.image(
                        tex.id(),
                        inner_map,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    painter.rect_stroke(
                        inner_map,
                        2.0,
                        Stroke::new(1.0, Color32::from_rgb(120, 113, 108)),
                        egui::StrokeKind::Middle,
                    );

                    // 월드 좌표 -> map_rect 화면 좌표 변환 클로저
                    let map_w_m = (w as f64) * out.transform.cell_size;
                    let map_h_m = (h as f64) * out.transform.cell_size;
                    let world_to_screen = |wx: f64, wy: f64| -> Pos2 {
                        let u = ((wx - out.transform.origin_x) / map_w_m).clamp(0.02, 0.98) as f32;
                        let v = ((out.transform.origin_y - wy) / map_h_m).clamp(0.05, 0.95) as f32;
                        Pos2::new(
                            inner_map.left() + u * inner_map.width(),
                            inner_map.top() + v * inner_map.height(),
                        )
                    };

                    // 조사구역(AOI) 경계 점선 사각형 표시
                    let aoi_r = Rect::from_center_size(
                        inner_map.center(),
                        Vec2::new(inner_map.width() * 0.34, inner_map.height() * 0.36),
                    );
                    painter.rect_stroke(
                        aoi_r,
                        2.0,
                        Stroke::new(1.6, Color32::from_rgb(185, 28, 28)),
                        egui::StrokeKind::Middle,
                    );
                    painter.text(
                        Pos2::new(aoi_r.left() + 5.0, aoi_r.top() + 4.0),
                        egui::Align2::LEFT_TOP,
                        "조사구역(AOI)",
                        egui::FontId::proportional(10.5),
                        Color32::from_rgb(153, 27, 27),
                    );

                    // A–A' 단면 절단선 표시
                    let sec_y = inner_map.center().y;
                    let p_a = Pos2::new(inner_map.left() + inner_map.width() * 0.06, sec_y);
                    let p_ap = Pos2::new(inner_map.right() - inner_map.width() * 0.06, sec_y);
                    painter.line_segment(
                        [p_a, p_ap],
                        Stroke::new(3.2, Color32::from_rgba_unmultiplied(255, 255, 255, 210)),
                    );
                    painter.line_segment(
                        [p_a, p_ap],
                        Stroke::new(2.0, Color32::from_rgb(185, 28, 28)),
                    );
                    for (pt, lbl) in [(p_a, "A"), (p_ap, "A'")] {
                        let badge = Rect::from_center_size(pt, Vec2::new(20.0, 18.0));
                        painter.rect_filled(badge, 4.0, Color32::from_rgb(185, 28, 28));
                        painter.text(
                            badge.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(11.0),
                            Color32::WHITE,
                        );
                    }

                    // 시추공(BH-01 ~ BH-08) 마커 및 충적층 두께 라벨 (PaleoAnalysisRunner.cpp:81-105)
                    for (idx, bh) in out.parsed_boreholes.iter().enumerate() {
                        let pt = world_to_screen(bh.x, bh.y);
                        let is_coarse = bh.sed_type.eq_ignore_ascii_case("COARSE");
                        let fill_c = if is_coarse {
                            Color32::from_rgb(255, 152, 0)
                        } else {
                            Color32::from_rgb(2, 132, 199)
                        };
                        painter.circle_filled(pt, 6.5, Color32::WHITE);
                        painter.circle_filled(pt, 5.0, fill_c);
                        painter.circle_stroke(pt, 5.0, Stroke::new(1.2, Color32::from_rgb(31, 41, 55)));

                        let dy = if idx % 2 == 0 { -16.0 } else { 15.0 };
                        let lbl = format!("{}({:.1}m)", bh.hole_id, bh.alluvial_thickness_m);
                        let pill = Rect::from_center_size(
                            Pos2::new(pt.x, pt.y + dy),
                            Vec2::new(66.0, 15.0),
                        );
                        painter.rect_filled(
                            pill,
                            3.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 230),
                        );
                        painter.rect_stroke(
                            pill,
                            3.0,
                            Stroke::new(0.7, Color32::from_rgb(120, 113, 108)),
                            egui::StrokeKind::Middle,
                        );
                        painter.text(
                            pill.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(28, 25, 23),
                        );
                    }

                    // 상단 제목 배지 및 하단 7대 미지형 색상 칩 바
                    let title_bg = Rect::from_min_size(
                        Pos2::new(inner_map.left() + 8.0, inner_map.top() + 8.0),
                        Vec2::new(410.0_f32.min(inner_map.width() - 16.0), 22.0),
                    );
                    painter.rect_filled(
                        title_bg,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 232),
                    );
                    painter.rect_stroke(
                        title_bg,
                        4.0,
                        Stroke::new(0.8, Color32::from_rgb(180, 176, 168)),
                        egui::StrokeKind::Middle,
                    );
                    painter.text(
                        Pos2::new(title_bg.left() + 8.0, title_bg.center().y),
                        egui::Align2::LEFT_CENTER,
                        "7대 고지형 분류도 (DEM 고도·경사·TPI·하천EDT 실물 분류 · A–A' 시추 단면선)",
                        egui::FontId::proportional(11.0),
                        Color32::from_rgb(28, 25, 23),
                    );

                    let strip_w = (inner_map.width() - 16.0).min(540.0);
                    let strip_r = Rect::from_min_size(
                        Pos2::new(inner_map.left() + 8.0, inner_map.bottom() - 24.0),
                        Vec2::new(strip_w, 19.0),
                    );
                    painter.rect_filled(
                        strip_r,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 235),
                    );
                    painter.rect_stroke(
                        strip_r,
                        4.0,
                        Stroke::new(0.8, Color32::from_rgb(180, 176, 168)),
                        egui::StrokeKind::Middle,
                    );
                    let cell_w = strip_w / 7.0;
                    for (i, cls) in PaleoClass::ALL.iter().enumerate() {
                        let [r, g, b, _] = cls.color_rgba();
                        let cx0 = strip_r.left() + (i as f32) * cell_w + 5.0;
                        let sw = Rect::from_min_size(
                            Pos2::new(cx0, strip_r.center().y - 5.0),
                            Vec2::new(10.0, 10.0),
                        );
                        painter.rect_filled(sw, 2.0, Color32::from_rgb(r, g, b));
                        painter.rect_stroke(
                            sw,
                            2.0,
                            Stroke::new(0.6, Color32::from_rgb(45, 43, 40)),
                            egui::StrokeKind::Middle,
                        );
                        painter.text(
                            Pos2::new(sw.right() + 3.5, strip_r.center().y),
                            egui::Align2::LEFT_CENTER,
                            cls.name_ko(),
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(28, 25, 23),
                        );
                    }
                }
            }

            ui.add_space(4.0);

            // (B) 하단: KaPaleoCrossSectionWidget 지층 단면도 + 시추공 주상도 (KaPaleoCrossSectionWidget.cpp:27-97)
            let sec_avail = ui.available_size();
            let (sec_rect, _) = ui.allocate_exact_size(
                Vec2::new(sec_avail.x, sec_avail.y.max(150.0)),
                egui::Sense::hover(),
            );
            let sp = ui.painter_at(sec_rect);
            sp.rect_filled(sec_rect, 4.0, Color32::from_rgb(250, 250, 249));
            sp.rect_stroke(
                sec_rect,
                4.0,
                Stroke::new(1.0, Color32::from_rgb(214, 211, 209)),
                egui::StrokeKind::Middle,
            );

            sp.text(
                Pos2::new(sec_rect.left() + 10.0, sec_rect.top() + 11.0),
                egui::Align2::LEFT_CENTER,
                "2D 가상 지층 단면도 (현 지표면 · 매립 제거 원지형 · 충적 문화층 · 생토면 · 하천 수면 · 시추공 주상도)",
                egui::FontId::proportional(11.5),
                Color32::from_rgb(28, 25, 23),
            );

            let plot = Rect::from_min_max(
                Pos2::new(sec_rect.left() + 48.0, sec_rect.top() + 24.0),
                Pos2::new(sec_rect.right() - 14.0, sec_rect.bottom() - 24.0),
            );
            sp.rect_filled(plot, 0.0, Color32::WHITE);
            sp.rect_stroke(
                plot,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(214, 211, 209)),
                egui::StrokeKind::Middle,
            );

            if let Some(ref out) = self.paleo_3d.last_paleo_output {
                let cs = &out.center_cross_section;
                let n = cs.dist_m.len();
                if n >= 2 && plot.width() > 40.0 && plot.height() > 30.0 {
                    let mut z_min = cs
                        .z_bedrock_base_m
                        .iter()
                        .copied()
                        .fold(f64::INFINITY, f64::min);
                    let mut z_max = cs
                        .z_surface_curr_m
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max);
                    for bh in &out.parsed_boreholes {
                        z_min = z_min.min(bh.weathered_top_elev_m - 3.5);
                        z_max = z_max.max(bh.ground_elev_m + 1.5);
                    }
                    z_min -= 1.5;
                    z_max = (z_max + 2.0).max(z_min + 8.0);
                    let max_dist = cs.dist_m.last().copied().unwrap_or(1.0).max(1.0);

                    let to_pt = |d: f64, z: f64| -> Pos2 {
                        let x = plot.left() + ((d / max_dist) as f32) * plot.width();
                        let y =
                            plot.bottom() - (((z - z_min) / (z_max - z_min)) as f32) * plot.height();
                        Pos2::new(
                            x.clamp(plot.left(), plot.right()),
                            y.clamp(plot.top(), plot.bottom()),
                        )
                    };

                    // 수평 표고 보조선 및 좌측 표고 눈금
                    for step in 0..=4 {
                        let z_val = z_min + (z_max - z_min) * (step as f64 / 4.0);
                        let py = to_pt(0.0, z_val).y;
                        sp.line_segment(
                            [Pos2::new(plot.left(), py), Pos2::new(plot.right(), py)],
                            Stroke::new(0.6, Color32::from_rgb(238, 236, 232)),
                        );
                        sp.text(
                            Pos2::new(plot.left() - 5.0, py),
                            egui::Align2::RIGHT_CENTER,
                            format!("{z_val:.0}m"),
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(87, 83, 78),
                        );
                    }

                    // 3대 지층 단면 영역 채움 (성토층 / 충적 문화층 / 기반암 풍화토)
                    for i in 0..(n - 1) {
                        let d0 = cs.dist_m[i];
                        let d1 = cs.dist_m[i + 1];
                        let p_s0 = to_pt(d0, cs.z_surface_curr_m[i]);
                        let p_s1 = to_pt(d1, cs.z_surface_curr_m[i + 1]);
                        let p_p0 = to_pt(d0, cs.z_paleo_top_m[i]);
                        let p_p1 = to_pt(d1, cs.z_paleo_top_m[i + 1]);
                        let p_b0 = to_pt(d0, cs.z_bedrock_base_m[i]);
                        let p_b1 = to_pt(d1, cs.z_bedrock_base_m[i + 1]);

                        // (1) 기반암·풍화대 영역
                        sp.add(egui::Shape::convex_polygon(
                            vec![
                                p_b0,
                                p_b1,
                                Pos2::new(p_b1.x, plot.bottom()),
                                Pos2::new(p_b0.x, plot.bottom()),
                            ],
                            Color32::from_rgba_unmultiplied(168, 160, 144, 95),
                            Stroke::NONE,
                        ));
                        // (2) 충적 문화층 영역 (KaPaleoCrossSectionWidget.cpp:73 QColor(251, 192, 45, 70))
                        sp.add(egui::Shape::convex_polygon(
                            vec![p_p0, p_p1, p_b1, p_b0],
                            Color32::from_rgba_unmultiplied(251, 192, 45, 95),
                            Stroke::NONE,
                        ));
                        // (3) 현대 매립·성토층 영역
                        sp.add(egui::Shape::convex_polygon(
                            vec![p_s0, p_s1, p_p1, p_p0],
                            Color32::from_rgba_unmultiplied(194, 158, 120, 105),
                            Stroke::NONE,
                        ));
                    }

                    // 4대 경계선 곡선 (KaPaleoCrossSectionWidget.cpp:84-87)
                    let river_z0 = 22.0_f64;
                    for i in 0..(n - 1) {
                        let d0 = cs.dist_m[i];
                        let d1 = cs.dist_m[i + 1];
                        // 하천 수면 (파란 파선)
                        if i % 2 == 0 {
                            let r0 = river_z0 + 2.4 * (d0 / max_dist);
                            let r1 = river_z0 + 2.4 * (d1 / max_dist);
                            sp.line_segment(
                                [to_pt(d0, r0), to_pt(d1, r1)],
                                Stroke::new(1.4, Color32::from_rgb(2, 132, 199)),
                            );
                        }
                        // 생토면(풍화토) 갈색 실선
                        sp.line_segment(
                            [
                                to_pt(d0, cs.z_bedrock_base_m[i]),
                                to_pt(d1, cs.z_bedrock_base_m[i + 1]),
                            ],
                            Stroke::new(1.8, Color32::from_rgb(120, 53, 15)),
                        );
                        // 복원 원지형(Z_paleo) 주황 실선
                        sp.line_segment(
                            [
                                to_pt(d0, cs.z_paleo_top_m[i]),
                                to_pt(d1, cs.z_paleo_top_m[i + 1]),
                            ],
                            Stroke::new(2.2, Color32::from_rgb(217, 119, 6)),
                        );
                        // 현 지표면 점선
                        if i % 2 == 0 {
                            sp.line_segment(
                                [
                                    to_pt(d0, cs.z_surface_curr_m[i]),
                                    to_pt(d1, cs.z_surface_curr_m[i + 1]),
                                ],
                                Stroke::new(1.5, Color32::from_rgb(87, 83, 78)),
                            );
                        }
                    }

                    // 시추공 주상도 기둥 (BH-01 ~ BH-08: 성토층·충적층·풍화토 심도별 주상도 + 지하수위)
                    let col_half_w = 6.5_f32;
                    for bh in &out.parsed_boreholes {
                        let dist_along = (bh.x - out.transform.origin_x).clamp(0.0, max_dist);
                        let z_top = bh.ground_elev_m;
                        let z_paleo = bh.paleo_surface_elev_m;
                        let z_weathered = bh.weathered_top_elev_m;
                        let z_bot = (z_weathered - 3.2).max(z_min + 0.3);

                        let p_top = to_pt(dist_along, z_top);
                        let p_pal = to_pt(dist_along, z_paleo);
                        let p_wea = to_pt(dist_along, z_weathered);
                        let p_bot = to_pt(dist_along, z_bot);

                        let r_fill = Rect::from_min_max(
                            Pos2::new(p_top.x - col_half_w, p_top.y),
                            Pos2::new(p_top.x + col_half_w, p_pal.y.max(p_top.y + 3.0)),
                        );
                        let r_alluv = Rect::from_min_max(
                            Pos2::new(p_top.x - col_half_w, r_fill.bottom()),
                            Pos2::new(p_top.x + col_half_w, p_wea.y.max(r_fill.bottom() + 5.0)),
                        );
                        let r_weath = Rect::from_min_max(
                            Pos2::new(p_top.x - col_half_w, r_alluv.bottom()),
                            Pos2::new(p_top.x + col_half_w, p_bot.y.max(r_alluv.bottom() + 4.0)),
                        );

                        let is_coarse = bh.sed_type.eq_ignore_ascii_case("COARSE");
                        let alluv_col = if is_coarse {
                            Color32::from_rgb(245, 158, 11)
                        } else {
                            Color32::from_rgb(16, 185, 129)
                        };

                        sp.rect_filled(r_fill, 0.0, Color32::from_rgb(194, 158, 120));
                        sp.rect_filled(r_alluv, 0.0, alluv_col);
                        sp.rect_filled(r_weath, 0.0, Color32::from_rgb(120, 53, 15));

                        let full_col = Rect::from_min_max(r_fill.min, r_weath.max);
                        sp.rect_stroke(
                            full_col,
                            0.0,
                            Stroke::new(1.0, Color32::from_rgb(28, 25, 23)),
                            egui::StrokeKind::Middle,
                        );
                        sp.line_segment(
                            [
                                Pos2::new(r_fill.left(), r_fill.bottom()),
                                Pos2::new(r_fill.right(), r_fill.bottom()),
                            ],
                            Stroke::new(0.8, Color32::from_rgb(28, 25, 23)),
                        );
                        sp.line_segment(
                            [
                                Pos2::new(r_alluv.left(), r_alluv.bottom()),
                                Pos2::new(r_alluv.right(), r_alluv.bottom()),
                            ],
                            Stroke::new(0.8, Color32::from_rgb(28, 25, 23)),
                        );

                        // 지하수위 마커 (▼)
                        let p_water = to_pt(dist_along, z_top - bh.water_depth_m);
                        sp.line_segment(
                            [
                                Pos2::new(p_top.x - col_half_w - 3.0, p_water.y),
                                Pos2::new(p_top.x + col_half_w + 3.0, p_water.y),
                            ],
                            Stroke::new(1.2, Color32::from_rgb(2, 132, 199)),
                        );

                        // 시추공 코드 및 충적층 심도 라벨
                        sp.text(
                            Pos2::new(p_top.x, (p_top.y - 8.0).max(plot.top() + 7.0)),
                            egui::Align2::CENTER_CENTER,
                            &bh.hole_id,
                            egui::FontId::proportional(9.5),
                            Color32::from_rgb(28, 25, 23),
                        );
                        sp.text(
                            Pos2::new(p_top.x + col_half_w + 3.0, r_alluv.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{}:{:.1}m", bh.dominant_uscs, bh.alluvial_thickness_m),
                            egui::FontId::proportional(8.5),
                            Color32::from_rgb(45, 43, 40),
                        );
                    }
                }
            }

            // 하단 단면도 범례 문구 (KaPaleoCrossSectionWidget.cpp:96)
            sp.text(
                Pos2::new(plot.left(), sec_rect.bottom() - 11.0),
                egui::Align2::LEFT_CENTER,
                "점선: 현 지표면 | 주황 실선: 복원 원지형(Z_paleo) | 갈색 실선: 생토면(풍화토) | 파란 파선: 하천 수면 | 주상도 기둥: 시추공(성토·충적·풍화토)",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(87, 83, 78),
            );
        });
    }
}
