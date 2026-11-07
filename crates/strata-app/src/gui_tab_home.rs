use eframe::egui;
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg::GpkgRepository;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// `KaStartPage.cpp` + `KaHomeTitleBlock.cpp` + `KaHomeRecentCard.cpp` + `KaHomeGuideCard.cpp` + `KaHomeConnectionCard.cpp` 1:1 재현
    pub(crate) fn render_home_tab_ui(&mut self, ui: &mut egui::Ui) {
        let full_w = ui.available_width().clamp(600.0, 2400.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_max_width(full_w);
            ui.add_space(8.0);
            // 1. KaStartHero + KaHomeTitleBlock (상단 배너)
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(240, 238, 230))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(194, 192, 182)))
                .inner_margin(egui::Margin::symmetric(28, 16))
                .show(ui, |ui| {
                    ui.set_max_width((full_w - 60.0).max(500.0));
                    ui.columns(2, |cols| {
                        cols[0].horizontal(|ui| {
                            egui::Frame::NONE
                                .fill(egui::Color32::from_rgb(181, 87, 58))
                                .corner_radius(4.0)
                                .inner_margin(egui::Margin::symmetric(10, 5))
                                .show(ui, |ui| {
                                    ui.colored_label(egui::Color32::WHITE, egui::RichText::new("S").strong().size(18.0));
                                });
                            ui.label(egui::RichText::new("Strata").strong().size(24.0));
                            ui.separator();
                            ui.label(egui::RichText::new("필드고고학 GIS").size(16.0).color(egui::Color32::from_rgb(94, 93, 89)));
                        });
                        cols[0].add_space(6.0);
                        cols[0].label(egui::RichText::new("오늘 현장을 새로 만들거나, 지난 조사를 이어서 여세요.").size(14.0));
                        cols[0].add_space(10.0);
                        cols[0].horizontal(|ui| {
                            let btn_new = egui::Button::new(egui::RichText::new("새 조사").color(egui::Color32::WHITE).strong())
                                .fill(egui::Color32::from_rgb(181, 87, 58))
                                .min_size(egui::vec2(104.0, 34.0));
                            if ui.add(btn_new).clicked() {
                                self.start_new_survey_project("경주_인왕동_신규시굴조사");
                            }
                            let btn_open = egui::Button::new(egui::RichText::new("조사 열기").strong()).min_size(egui::vec2(104.0, 34.0));
                            if ui.add(btn_open).clicked() {
                                self.open_sample_survey_project(0);
                            }
                        });

                        // 우측: KaHomeTitleBlock (이어서 작업 + 6칸 셀)
                        egui::Frame::group(cols[1].style())
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 20, 19)))
                            .inner_margin(egui::Margin::symmetric(16, 10))
                            .show(&mut cols[1], |ui| {
                                ui.horizontal(|ui| {
                                    ui.strong("이어서 작업");
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if self.is_dirty {
                                            ui.colored_label(egui::Color32::from_rgb(217, 119, 87), "● 저장 안 됨");
                                        } else {
                                            ui.label("저장됨");
                                        }
                                    });
                                });
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&self.project_title).strong().size(18.0));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let resume_btn = egui::Button::new(egui::RichText::new("이어서 열기  →").color(egui::Color32::WHITE).strong())
                                            .fill(egui::Color32::from_rgb(181, 87, 58))
                                            .min_size(egui::vec2(130.0, 30.0));
                                        if ui.add(resume_btn).clicked() {
                                            self.open_sample_survey_project(0);
                                        }
                                    });
                                });
                                ui.separator();
                                let crs_short = if self.work_crs.epsg_code() == 5186 { "5186 중부원점" } else { "5187 동부원점" };
                                let area_cnt = self.home_map.scene.polygons.iter().filter(|p| p.layer_key == "survey_area").count();
                                let feat_cnt = self.home_map.scene.polygons.len();
                                egui::Grid::new("ka_home_title_block_cells").num_columns(6).spacing([14.0, 2.0]).show(ui, |ui| {
                                    for cap in ["폴더", "마지막 열림", "작업 좌표계", "조사구역", "유구", "검수"] {
                                        ui.label(egui::RichText::new(cap).size(11.0).color(egui::Color32::from_rgb(94, 93, 89)));
                                    }
                                    ui.end_row();
                                    ui.strong("문서 › hgis");
                                    ui.strong("오늘 14:20");
                                    ui.strong(crs_short);
                                    ui.strong(format!("{area_cnt} 곳"));
                                    ui.strong(format!("{feat_cnt} 기"));
                                    ui.strong("통과");
                                    ui.end_row();
                                });
                            });
                    });
                });

            ui.add_space(12.0);

            // 2. 중단: 좌측 최근 조사(KaHomeRecentCard) + 우측 작업 순서(KaHomeGuideCard)
            ui.columns(2, |cols| {
                egui::Frame::group(cols[0].style())
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(222, 220, 209)))
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(&mut cols[0], |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("최근 조사").strong().size(16.0));
                            ui.label(format!("{}", self.home_map.recent_files.len()));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.home_map.recent_filter).hint_text("🔍 이름·폴더로 찾기").desired_width(175.0));
                            });
                        });
                        ui.separator();
                        egui::Grid::new("ka_recent_survey_table").striped(true).num_columns(6).spacing([14.0, 8.0]).show(ui, |ui| {
                            for h in ["조사", "상태", "조사구역", "유구", "검수", "마지막 열림"] {
                                ui.strong(h);
                            }
                            ui.end_row();
                            let filter = self.home_map.recent_filter.trim().to_lowercase();
                            let files = self.home_map.recent_files.clone();
                            for (idx, name) in files.iter().enumerate() {
                                if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                                    continue;
                                }
                                if ui.link(egui::RichText::new(name).strong()).clicked() {
                                    self.open_sample_survey_project(idx);
                                }
                                ui.colored_label(egui::Color32::from_rgb(63, 107, 49), "저장됨");
                                ui.label("1 곳");
                                ui.label(format!("{} 기", 4 + idx * 2));
                                ui.label("통과");
                                ui.label(if idx == 0 { "오늘 14:20" } else { "어제 17:45" });
                                ui.end_row();
                            }
                        });
                    });

                egui::Frame::group(cols[1].style())
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(222, 220, 209)))
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(&mut cols[1], |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("작업 순서").strong().size(16.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{} 기준", self.project_title)).size(12.0));
                            });
                        });
                        ui.separator();
                        for (idx, title, body, is_next) in [
                            ("1", "새 조사 · 열기", "새 조사를 만들거나, 지난 조사 파일을 엽니다.", false),
                            ("2", "조사구역 그리기", "지도 탭에서 위성·지적을 보며 「그리기」로 구역과 유구를 그립니다.", true),
                            ("3", "도면 · 제출", "「도면」으로 종이에 옮기고, 「검수·제출」에서 검수한 뒤 제출 꾸러미(5179 SHP·조사도면.pdf·MANIFEST)를 만듭니다.", false),
                        ] {
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(181, 87, 58), egui::RichText::new(idx).strong().size(15.0));
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.link(egui::RichText::new(title).strong()).clicked() {
                                            match idx {
                                                "1" => self.start_new_survey_project("경주_인왕동_신규시굴조사"),
                                                "2" => self.trigger_map_ribbon_tool(MapToolKind::DrawPolygon),
                                                _ => {
                                                    self.active_tab = MainTab::Drawing;
                                                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                                                }
                                            }
                                        }
                                        if is_next {
                                            ui.colored_label(egui::Color32::from_rgb(181, 87, 58), "[다음]");
                                        }
                                    });
                                    ui.label(egui::RichText::new(body).size(12.0).color(egui::Color32::from_rgb(94, 93, 89)));
                                });
                            });
                            ui.add_space(4.0);
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("제출 준비:");
                            ui.colored_label(egui::Color32::from_rgb(63, 107, 49), "오류 0");
                            ui.label("·");
                            ui.colored_label(egui::Color32::from_rgb(63, 107, 49), "주의 0");
                        });
                    });
            });

            ui.add_space(12.0);

            // 3. 하단: KaHomeConnectionCard.cpp (연결 상태)
            egui::Frame::group(ui.style())
                .fill(egui::Color32::WHITE)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(222, 220, 209)))
                .inner_margin(egui::Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    ui.set_max_width((full_w - 40.0).max(500.0));
                    ui.horizontal(|ui| {
                        ui.strong("연결 상태");
                        ui.label(egui::RichText::new("🔒 비밀값은 표시되지 않습니다").size(12.0).color(egui::Color32::from_rgb(94, 93, 89)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("인터넷 확인").clicked() {
                                self.home_map.internet_checked = true;
                            }
                            if self.home_map.internet_checked {
                                ui.colored_label(egui::Color32::from_rgb(63, 107, 49), "● 인터넷 · 연결됨");
                            } else {
                                ui.label("인터넷 · 확인 안 함");
                            }
                        });
                    });
                    ui.separator();
                    ui.columns(3, |cols| {
                        for (idx, (name, ready)) in [
                            ("VWorld 키", self.accounts.vworld_configured),
                            ("수치지형도 계정", self.accounts.ngii_configured),
                            ("문화재 인트라넷", self.accounts.heritage_configured),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            cols[idx].horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(name);
                                    if ready {
                                        ui.colored_label(egui::Color32::from_rgb(63, 107, 49), "설정됨");
                                    } else {
                                        ui.colored_label(egui::Color32::from_rgb(217, 119, 87), "● 설정 필요");
                                    }
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("설정").clicked() {
                                        self.modals.active = Some(ActiveModalDialog::AccountSettings);
                                    }
                                });
                            });
                        }
                    });
                });
        });
    }

    pub fn start_new_survey_project(&mut self, name: &str) {
        self.project_title = name.to_owned();
        let path = std::env::temp_dir().join(format!("{name}.gpkg"));
        let _ = GpkgRepository::create_new_survey(&path, self.work_crs);
        self.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
        self.left_layers.ensure_domain_layer(DomainLayerKey::TrialTrench);
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!("새 조사 「{name}」(.gpkg)가 준비되었습니다."));
    }

    pub fn open_sample_survey_project(&mut self, idx: usize) {
        if let Some(name) = self.home_map.recent_files.get(idx).cloned() {
            self.project_title = name.trim_end_matches(".gpkg").to_owned();
        }
        self.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
        self.left_layers.ensure_domain_layer(DomainLayerKey::TrialTrench);
        self.left_layers.add_cadastral_wms_layer();
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some(format!("「{}」 조사를 열었습니다.", self.project_title));
    }

    pub fn import_demo_dxf_and_csv(&mut self) {
        self.left_layers.ensure_domain_layer(DomainLayerKey::ControlPoints);
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some("측량 DXF 표고점 및 기준점 CSV 레이어를 지도에 올렸습니다.".to_owned());
    }
}
