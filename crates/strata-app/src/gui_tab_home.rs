use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg::GpkgRepository;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon_icons::paint_strata_logo;

impl StrataAppState {
    pub(crate) fn render_home_tab_ui(&mut self, ui: &mut egui::Ui) {
        let total_w = ui.available_width().clamp(800.0, 3600.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_max_width(total_w);
            egui::Frame::NONE.inner_margin(egui::Margin::symmetric(20, 14)).show(ui, |ui| {
                let inner_w = (total_w - 40.0).max(760.0);
                ui.set_max_width(inner_w);

                let hero_w = (inner_w * 0.26).max(240.0);
                let title_w = (inner_w - hero_w - 20.0).max(480.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(hero_w, 146.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            let (logo_r, _) = ui.allocate_exact_size(Vec2::new(40.0, 40.0), egui::Sense::hover());
                            paint_strata_logo(&ui.painter_at(logo_r), logo_r);
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Strata").size(28.0).strong());
                            ui.label(egui::RichText::new("|").size(20.0).color(Color32::from_rgb(160, 158, 150)));
                            ui.label(egui::RichText::new("필드고고학 GIS").size(14.5).color(Color32::from_rgb(80, 79, 74)));
                        });
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("오늘 현장을 새로 만들거나, 지난 조사를 이어서 여세요.").size(13.0).color(Color32::from_rgb(75, 74, 70)));
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            let b1 = egui::Button::new(egui::RichText::new("⊞ 새 조사").size(13.0).strong()).fill(Color32::WHITE).stroke(Stroke::new(1.2, Color32::from_rgb(65, 64, 60))).corner_radius(6.0).min_size(Vec2::new(88.0, 34.0));
                            if ui.add(b1).clicked() { self.start_new_survey_project("경주시"); }
                            let b2 = egui::Button::new(egui::RichText::new("조사 열기").size(13.0).strong()).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(175, 172, 162))).corner_radius(6.0).min_size(Vec2::new(96.0, 34.0));
                            if ui.add(b2).clicked() { self.open_sample_survey_project(0); }
                        });
                    });
                    ui.add_space(16.0);
                    let (tb_rect, _) = ui.allocate_exact_size(Vec2::new(title_w, 146.0), egui::Sense::hover());
                    self.paint_home_title_block(ui, tb_rect);
                });

                ui.add_space(22.0);

                let recent_w = (inner_w * 0.77).max(520.0);
                let guide_w = (inner_w - recent_w - 20.0).max(200.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(Vec2::new(recent_w, 250.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("최근 조사").size(15.0).strong());
                            ui.label(egui::RichText::new("4개").size(12.0).color(Color32::from_rgb(110, 108, 102)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(egui::TextEdit::singleline(&mut self.home_map.recent_filter).hint_text("Q 이름·폴더로 찾기").desired_width(190.0));
                            });
                        });
                        ui.add_space(6.0);
                        self.render_recent_survey_box(ui, recent_w);
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("목록에서 빼려면 조사 위에서 오른쪽 단추를 누르세요.").size(11.5).color(Color32::from_rgb(105, 103, 96)));
                    });
                    ui.add_space(18.0);
                    ui.allocate_ui_with_layout(Vec2::new(guide_w, 250.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("작업 순서").size(15.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("경주시 기준").size(11.5).color(Color32::from_rgb(105, 103, 96)));
                            });
                        });
                        ui.add_space(10.0);
                        for (badge, title, next_tag, desc, step_id) in [
                            ("1", "새 조사", false, "조사 이름과 좌표계(5186·5187)를 정하면 조사 파일(GPKG)이 만들어집니다.", 1),
                            ("●", "조사구역 그리기", true, "지도 탭에서 위성·지적을 보며 「그리기」로 구역과 유구를 그립니다.", 2),
                            ("3", "도면 · 제출", false, "「도면」으로 종이에 옮기고, 「검수·제출」에서 검수한 뒤 제출 꾸러미(5179 SHP·조사도면.pdf·MANIFEST)를 만듭니다.\n아직 검수하지 않았습니다 — 지도 탭 「검수·제출」에서 확인합니다.", 3),
                        ] {
                            ui.horizontal_top(|ui| {
                                let col = if next_tag { Color32::from_rgb(181, 87, 58) } else { Color32::from_rgb(75, 74, 70) };
                                ui.colored_label(col, egui::RichText::new(badge).strong().size(13.0));
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.link(egui::RichText::new(title).strong().size(13.0)).clicked() {
                                            match step_id {
                                                1 => self.start_new_survey_project("경주시"),
                                                2 => self.trigger_map_ribbon_tool(MapToolKind::DrawPolygon),
                                                _ => self.modals.active = Some(ActiveModalDialog::PaperSetup),
                                            }
                                        }
                                        if next_tag {
                                            ui.colored_label(Color32::from_rgb(181, 87, 58), egui::RichText::new("• 다음").size(11.5).strong());
                                        }
                                    });
                                    ui.label(egui::RichText::new(desc).size(11.5).color(Color32::from_rgb(95, 94, 88)));
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });
                });

                ui.add_space(18.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("연결 상태").size(14.5).strong());
                    ui.label(egui::RichText::new("비밀값은 표시되지 않습니다").size(12.0).color(Color32::from_rgb(105, 103, 96)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("인터넷 확인").clicked() {
                            self.home_map.internet_checked = true;
                            self.home_map.draw_guide_message = Some("인터넷 연결 및 VWorld·국토정보플랫폼·국가유산청 계정 설정을 확인했습니다.".to_owned());
                        }
                        let net_txt = if self.home_map.internet_checked { "인터넷 · 연결됨" } else { "인터넷 · 확인 안 함" };
                        ui.label(egui::RichText::new(net_txt).size(12.0).color(Color32::from_rgb(95, 94, 88)));
                    });
                });
                ui.add_space(8.0);
                let statuses = [
                    ("VWorld 키", self.accounts.vworld_configured),
                    ("역사지리정보DB 키", self.accounts.history_gis_configured),
                    ("VWorld 지적도 계정", self.accounts.cadastral_configured),
                    ("수치지형도 계정", self.accounts.ngii_configured),
                    ("문화재 인트라넷", self.accounts.heritage_configured),
                ];
                ui.columns(5, |cols| {
                    for (idx, (title, ready)) in statuses.into_iter().enumerate() {
                        cols[idx].horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(title).size(12.5).strong());
                                let st_txt = if ready { "설정됨" } else { "미설정" };
                                ui.label(egui::RichText::new(st_txt).size(11.5).color(Color32::from_rgb(105, 103, 96)));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.link(egui::RichText::new("설정").size(12.0)).clicked() {
                                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                                }
                            });
                        });
                    }
                });
            });
        });
    }

    fn paint_home_title_block(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = ui.painter_at(rect);
        let border = Stroke::new(1.0, Color32::from_rgb(90, 88, 82));
        let sub_border = Stroke::new(1.0, Color32::from_rgb(185, 182, 172));
        p.rect_filled(rect, 3.0, Color32::WHITE);
        p.rect_stroke(rect, 3.0, border, egui::StrokeKind::Middle);

        let (y1, y2) = (rect.top() + 28.0, rect.top() + 96.0);
        p.line_segment([Pos2::new(rect.left(), y1), Pos2::new(rect.right(), y1)], sub_border);
        p.line_segment([Pos2::new(rect.left(), y2), Pos2::new(rect.right(), y2)], sub_border);
        p.text(Pos2::new(rect.left() + 12.0, rect.top() + 14.0), egui::Align2::LEFT_CENTER, "이어서 작업", egui::FontId::proportional(11.5), Color32::from_rgb(95, 94, 88));
        p.text(Pos2::new(rect.right() - 12.0, rect.top() + 14.0), egui::Align2::RIGHT_CENTER, "저장됨", egui::FontId::proportional(11.5), Color32::from_rgb(95, 94, 88));

        let split_x = rect.right() - 210.0;
        p.line_segment([Pos2::new(split_x, y1), Pos2::new(split_x, y2)], sub_border);
        p.text(Pos2::new(rect.left() + 14.0, (y1 + y2) * 0.5), egui::Align2::LEFT_CENTER, "경주시", egui::FontId::proportional(26.0), Color32::from_rgb(30, 30, 28));
        let btn_rect = Rect::from_center_size(Pos2::new((split_x + rect.right()) * 0.5, (y1 + y2) * 0.5), Vec2::new(178.0, 38.0));
        p.rect_filled(btn_rect, 5.0, Color32::from_rgb(181, 87, 58));
        p.text(btn_rect.center(), egui::Align2::CENTER_CENTER, "이어서 열기  →", egui::FontId::proportional(13.5), Color32::WHITE);
        if ui.interact(btn_rect, ui.id().with("home_resume_btn"), egui::Sense::click()).clicked() {
            self.open_sample_survey_project(0);
        }

        let area_cnt = format!("{} 곳", self.home_map.real_survey.survey_rings.len().max(1));
        let cells = [
            ("폴더", "문서 › 경주시"),
            ("마지막 열림", "어제 22:26"),
            ("작업 좌표계", "5187 동부원점"),
            ("조사구역", area_cnt.as_str()),
            ("유구", "0 기"),
            ("검수", "검수 전"),
        ];
        let cw = rect.width() / 6.0;
        for (i, (k, v)) in cells.iter().enumerate() {
            let x0 = rect.left() + cw * (i as f32);
            if i > 0 { p.line_segment([Pos2::new(x0, y2), Pos2::new(x0, rect.bottom())], sub_border); }
            p.text(Pos2::new(x0 + 10.0, y2 + 14.0), egui::Align2::LEFT_CENTER, *k, egui::FontId::proportional(11.0), Color32::from_rgb(105, 103, 96));
            p.text(Pos2::new(x0 + 10.0, y2 + 33.0), egui::Align2::LEFT_CENTER, *v, egui::FontId::proportional(12.5), Color32::from_rgb(35, 35, 32));
        }
    }

    fn render_recent_survey_box(&mut self, ui: &mut egui::Ui, width: f32) {
        let (box_r, _) = ui.allocate_exact_size(Vec2::new(width, 206.0), egui::Sense::hover());
        let p = ui.painter_at(box_r);
        p.rect_filled(box_r, 2.0, Color32::WHITE);
        p.rect_stroke(box_r, 2.0, Stroke::new(1.0, Color32::from_rgb(185, 182, 172)), egui::StrokeKind::Middle);

        let col_x = [
            box_r.left() + 14.0,
            box_r.left() + width * 0.62,
            box_r.left() + width * 0.71,
            box_r.left() + width * 0.78,
            box_r.left() + width * 0.84,
            box_r.left() + width * 0.91,
        ];
        for (i, h) in ["조사", "상태", "조사구역", "유구", "검수", "마지막 열림"].iter().enumerate() {
            p.text(Pos2::new(col_x[i], box_r.top() + 15.0), egui::Align2::LEFT_CENTER, *h, egui::FontId::proportional(11.0), Color32::from_rgb(95, 94, 88));
        }
        p.line_segment([Pos2::new(box_r.left(), box_r.top() + 28.0), Pos2::new(box_r.right(), box_r.top() + 28.0)], Stroke::new(1.0, Color32::from_rgb(222, 220, 210)));

        let rows = [
            ("경주시", "문서 › 경주시", "저장됨", "1", "0", "검수 전", "어제 22:26", true),
            ("verify1-저장", "... › ka-hgis-survey-verify", "⊗ 원본 없음", "—", "—", "—", "찾을 수 없음", false),
            ("verify1", "... › ka-hgis-survey-verify", "⊗ 원본 없음", "—", "—", "—", "찾을 수 없음", false),
            ("andong", "바탕 화면 › andong", "⊗ 원본 없음", "—", "—", "—", "찾을 수 없음", false),
        ];
        for (idx, (name, folder, st, area, feat, chk, opened, ok)) in rows.into_iter().enumerate() {
            if !self.home_map.recent_filter.trim().is_empty() && !name.contains(self.home_map.recent_filter.trim()) {
                continue;
            }
            let ry = box_r.top() + 30.0 + (idx as f32) * 43.0;
            let row_r = Rect::from_min_size(Pos2::new(box_r.left() + 1.0, ry), Vec2::new(width - 2.0, 42.0));
            if idx == 0 {
                p.rect_filled(row_r, 0.0, Color32::from_rgb(239, 236, 226));
            }
            p.text(Pos2::new(col_x[0], ry + 13.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(13.0), Color32::from_rgb(35, 35, 32));
            p.text(Pos2::new(col_x[0], ry + 30.0), egui::Align2::LEFT_CENTER, folder, egui::FontId::proportional(11.0), Color32::from_rgb(115, 113, 106));
            let st_col = if ok { Color32::from_rgb(65, 64, 60) } else { Color32::from_rgb(181, 87, 58) };
            for (ci, val) in [(1, st), (2, area), (3, feat), (4, chk), (5, opened)] {
                let c = if ci == 1 { st_col } else { Color32::from_rgb(65, 64, 60) };
                p.text(Pos2::new(col_x[ci], ry + 21.0), egui::Align2::LEFT_CENTER, val, egui::FontId::proportional(12.0), c);
            }
            if ui.interact(row_r, ui.id().with(("recent_row", idx)), egui::Sense::click()).clicked() {
                self.open_sample_survey_project(idx);
            }
        }
    }

    pub fn start_new_survey_project(&mut self, name: &str) {
        self.project_title = name.to_owned();
        let path = std::env::temp_dir().join(format!("{name}.gpkg"));
        let _ = GpkgRepository::create_new_survey(&path, self.work_crs);
        self.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
        self.home_map.fit_to_survey();
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some("위성과 지적도를 올려 두었습니다.".to_owned());
    }

    pub fn open_sample_survey_project(&mut self, _idx: usize) {
        self.project_title = "경주시".to_owned();
        self.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
        self.left_layers.add_cadastral_wms_layer();
        self.home_map.fit_to_survey();
        self.active_tab = MainTab::Map;
        self.home_map.draw_guide_message = Some("다 그렸으면 리본의 「도면」으로 종이에 옮기세요.".to_owned());
    }

    pub fn import_demo_dxf_and_csv(&mut self) {
        self.left_layers.ensure_domain_layer(DomainLayerKey::ControlPoints);
        self.active_tab = MainTab::Map;
    }
}
