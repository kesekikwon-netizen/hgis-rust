//! 「고지형」 탭의 우측 스크롤 패널·좌측 레이어 패널.
//!
//! `gui_tab_paleo.rs` 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 원래 `render_paleo_tab_ui` 안의 1·2번 절을 그대로 옮겼고, 순서·수치·문구는 바꾸지 않았다.

use eframe::egui::{self, Color32, Stroke, Vec2};
use strata_paleo::pipeline::PaleoClass;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 1. 우측 스크롤 패널 (`KaPaleoStudio.cpp:123-204`)
    pub(crate) fn render_paleo_right_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
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

    }

    /// 2. 좌측 레이어 패널 (`KaPaleoStudio.cpp:51-99`)
    pub(crate) fn render_paleo_layer_panel(&mut self, ui: &mut egui::Ui) {
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
    }
}
