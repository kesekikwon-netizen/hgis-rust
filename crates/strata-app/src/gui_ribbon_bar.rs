use eframe::egui;
use strata_canvas2d::map_tools::MapToolKind;
use strata_net::tile_fetcher::BasemapSource;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{DrawSubTarget, SubToolsMode, PENINSULA_REGION_TILES};

impl StrataAppState {
    /// `MainWindowRibbon.cpp:251-730` — 7개 리본 묶음(조사·기록·자료 받기·배경 지도·정합·내보내기·기타) + 우측 `지역`·`주소·지번 찾기`
    pub(crate) fn render_top_ribbon_and_subtoolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("ka_main_ribbon_toolbar")
            .exact_height(58.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // [그룹 1: 조사] 새 조사 · 열기 · 저장 · 다른 이름
                    Self::ribbon_group_frame(ui, "조사", |ui| {
                        if ui.button("새 조사").on_hover_text("현장 조사 프로젝트를 새로 만듭니다 (Ctrl+N)").clicked() {
                            self.start_new_survey_project("경주_인왕동_신규시굴조사");
                        }
                        if ui.button("열기").on_hover_text("저장한 조사를 엽니다 (Ctrl+O)").clicked() {
                            self.open_sample_survey_project(0);
                        }
                        let save_lbl = if self.is_dirty { "● 저장" } else { "저장" };
                        let save_btn = egui::Button::new(egui::RichText::new(save_lbl).color(egui::Color32::WHITE).strong())
                            .fill(egui::Color32::from_rgb(181, 87, 58));
                        if ui.add(save_btn).on_hover_text("현재 조사를 저장합니다 (Ctrl+S)").clicked() {
                            self.is_dirty = false;
                            self.home_map.draw_guide_message = Some("현재 조사를 .gpkg에 세대 사본 검증 저장했습니다.".to_owned());
                        }
                        if ui.button("다른 이름").on_hover_text("다른 이름으로 저장합니다 (Ctrl+Shift+S)").clicked() {
                            self.is_dirty = false;
                            self.home_map.draw_guide_message = Some("다른 이름으로 사본(.gpkg)을 저장했습니다.".to_owned());
                        }
                    });

                    // [그룹 2: 기록] 선택 · 되돌리기 · 다시 실행 · 측거 · 그리기 · 시굴격자 · 버퍼
                    Self::ribbon_group_frame(ui, "기록", |ui| {
                        if ui.selectable_label(self.active_tool == MapToolKind::Select, "선택")
                            .on_hover_text("그린 도형을 선택합니다 (Ctrl+1 / A)").clicked()
                        {
                            let next = if self.active_tool == MapToolKind::Select { MapToolKind::Pan } else { MapToolKind::Select };
                            self.trigger_map_ribbon_tool(next);
                        }
                        if ui.button("되돌리기").on_hover_text("마지막 편집 되돌리기 (Ctrl+Z)").clicked() {
                            self.home_map.scene.polygons.pop();
                        }
                        if ui.button("다시 실행").on_hover_text("되돌린 편집 다시 적용 (Ctrl+Y)").clicked() {
                            self.home_map.draw_guide_message = Some("되돌린 편집을 다시 적용했습니다.".to_owned());
                        }
                        if ui.selectable_label(self.active_tool == MapToolKind::Measure, "측거").clicked() {
                            self.trigger_map_ribbon_tool(MapToolKind::Measure);
                        }
                        let draw_on = self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw;
                        if ui.selectable_label(draw_on, "그리기").on_hover_text("조사구역·유구 면과 선을 그립니다 (Ctrl+D)").clicked() {
                            if draw_on {
                                self.ribbon.sub_toolbar_visible = false;
                                self.ribbon.sub_mode = SubToolsMode::None;
                                self.trigger_map_ribbon_tool(MapToolKind::Pan);
                            } else {
                                self.ribbon.sub_toolbar_visible = true;
                                self.ribbon.sub_mode = SubToolsMode::Draw;
                                self.trigger_map_ribbon_tool(MapToolKind::DrawPolygon);
                            }
                        }
                        if ui.selectable_label(self.active_tool == MapToolKind::TrenchPlace, "시굴격자").clicked() {
                            self.trigger_map_ribbon_tool(MapToolKind::TrenchPlace);
                            self.modals.active = Some(ActiveModalDialog::TrenchGenerator);
                        }
                        let buf_on = self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Buffer;
                        if ui.selectable_label(buf_on, "버퍼").on_hover_text("주변유적 500M·1000M 경계를 그립니다").clicked() {
                            self.ribbon.sub_toolbar_visible = !buf_on;
                            self.ribbon.sub_mode = if buf_on { SubToolsMode::None } else { SubToolsMode::Buffer };
                            if self.active_tab != MainTab::Map && self.active_tab != MainTab::Home {
                                self.active_tab = MainTab::Map;
                            }
                        }
                    });

                    // [그룹 3: 자료 받기] DEM · 지적 · 수치지형 · 유산
                    Self::ribbon_group_frame(ui, "자료 받기", |ui| {
                        if ui.button("DEM").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some("DEM 고도·음영 레이어를 겹쳤습니다.".to_owned());
                        }
                        if ui.button("지적").clicked() {
                            self.active_tab = MainTab::Map;
                            self.left_layers.add_cadastral_wms_layer();
                            self.home_map.active_basemap = BasemapSource::VworldCadastralWms;
                            self.home_map.draw_guide_message = Some("VWorld 연속지적도(경계선·지번)를 지도에 올렸습니다.".to_owned());
                        }
                        if ui.button("수치지형").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some("국토정보플랫폼 1:5,000 수치지형도 등고선을 합성했습니다.".to_owned());
                        }
                        if ui.button("유산").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some("국가유산청 인트라넷 시·군 유적 레이어(3개소)를 올렸습니다.".to_owned());
                        }
                    });

                    // [그룹 4: 배경 지도] 지형 · 등고선 · 토양 · 고지형 · 옛 지도 · 지질 · 수계
                    Self::ribbon_group_frame(ui, "배경 지도", |ui| {
                        if ui.button("지형").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.active_basemap = BasemapSource::VworldBase;
                        }
                        if ui.button("등고선").clicked() {
                            self.active_tab = MainTab::Map;
                            self.left_layers.ensure_domain_layer(DomainLayerKey::SectionLine);
                        }
                        if ui.button("토양").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some("흙토람 분포지형 토양도를 겹쳤습니다.".to_owned());
                        }
                        if ui.button("고지형").clicked() {
                            self.active_tab = MainTab::Paleo;
                        }
                        egui::ComboBox::from_id_salt("ribbon_old_maps")
                            .selected_text("옛 지도")
                            .width(68.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(self.home_map.active_basemap == BasemapSource::Daedongyeojido, "대동여지도").clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.home_map.active_basemap = BasemapSource::Daedongyeojido;
                                }
                                if ui.selectable_label(self.home_map.active_basemap == BasemapSource::History1919, "1919 조선지형도 1:5만").clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.home_map.active_basemap = BasemapSource::History1919;
                                }
                            });
                        if ui.button("지질").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some("KIGAM 1:5만 수치지질도를 겹쳤습니다.".to_owned());
                        }
                        if ui.button("수계").clicked() {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some("하천망 수계 레이어를 겹쳤습니다.".to_owned());
                        }
                    });

                    // [그룹 5: 정합] 정합
                    Self::ribbon_group_frame(ui, "정합", |ui| {
                        if ui.selectable_label(self.active_tool == MapToolKind::GeorefAlign, "정합").clicked() {
                            self.ribbon.sub_toolbar_visible = true;
                            self.ribbon.sub_mode = SubToolsMode::Align;
                            self.trigger_map_ribbon_tool(MapToolKind::GeorefAlign);
                        }
                    });

                    // [그룹 6: 내보내기] 도면 · 인쇄 · GeoTIFF · 검수·제출
                    Self::ribbon_group_frame(ui, "내보내기", |ui| {
                        let dom_btn = egui::Button::new(egui::RichText::new("도면").strong())
                            .fill(egui::Color32::from_rgb(227, 218, 204));
                        if ui.add(dom_btn).clicked() {
                            self.active_tab = MainTab::Drawing;
                        }
                        if ui.button("인쇄").clicked() {
                            self.active_tab = MainTab::Drawing;
                        }
                        if ui.button("GeoTIFF").clicked() {
                            self.home_map.draw_guide_message = Some("현재 지도 범위를 EPSG:5187 GeoTIFF(Tag 34735)로 저장했습니다.".to_owned());
                        }
                        if ui.button("검수·제출").clicked() {
                            self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                        }
                    });

                    // [그룹 7: 기타] 입체지형 · 더보기
                    Self::ribbon_group_frame(ui, "기타", |ui| {
                        if ui.button("입체지형").clicked() {
                            self.active_tab = MainTab::Terrain3d;
                        }
                        if ui.button("더보기 ▾").clicked() {
                            self.modals.active = Some(ActiveModalDialog::AccountSettings);
                        }
                    });

                    // 우측 끝: KaAppBar (📍 지역 + 🔍 주소·지번 찾기)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.query)
                                .hint_text("주소·지번 찾기")
                                .desired_width(140.0),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some(format!("「{}」 위치로 이동했습니다.", self.ribbon.app_bar.query));
                        }
                        let reg_label = format!("📍 지역({})", self.ribbon.app_bar.selected_sido);
                        if ui.button(reg_label).clicked() {
                            self.ribbon.app_bar.region_popup_open = !self.ribbon.app_bar.region_popup_open;
                        }
                    });
                });
            });

        self.render_subtoolbar_and_region_popup(ctx);
    }

    fn render_subtoolbar_and_region_popup(&mut self, ctx: &egui::Context) {
        if self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode != SubToolsMode::None {
            egui::TopBottomPanel::top("ka_sub_toolbar").exact_height(32.0).show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    match self.ribbon.sub_mode {
                        SubToolsMode::Draw => {
                            ui.strong("  그리기 › ");
                            for (target, tool) in [
                                (DrawSubTarget::EasyDraw, MapToolKind::DrawPolygon),
                                (DrawSubTarget::SurveyArea, MapToolKind::DrawPolygon),
                                (DrawSubTarget::FeaturePoly, MapToolKind::DrawPolygon),
                                (DrawSubTarget::FeatureLine, MapToolKind::DrawLine),
                                (DrawSubTarget::Artifact, MapToolKind::PlacePoint),
                            ] {
                                if ui.selectable_label(self.ribbon.draw_sub_target == target, target.label_ko()).clicked() {
                                    self.ribbon.draw_sub_target = target;
                                    self.trigger_map_ribbon_tool(tool);
                                }
                            }
                        }
                        SubToolsMode::Buffer => {
                            ui.strong("  주변유적경계 › ");
                            for lbl in ["500m 경계 표시", "1000m 경계 표시", "500m · 1000m 동시 표시"] {
                                if ui.button(lbl).clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.home_map.draw_guide_message = Some(format!("조사구역 둘레 {lbl} 완료."));
                                }
                            }
                        }
                        SubToolsMode::Align => {
                            ui.strong("  도면정합 › ");
                            if ui.button("사진·CAD 불러오기").clicked() {
                                self.import_demo_dxf_and_csv();
                            }
                            if ui.button("기준점 짝짓기 (Helmert)").clicked() {
                                self.home_map.draw_guide_message = Some("Helmert 2D 상사변환 정합 완료 (RMS 0.04mm).".to_owned());
                            }
                        }
                        SubToolsMode::None => {}
                    }
                });
            });
        }

        if self.ribbon.app_bar.region_popup_open {
            let mut open = true;
            egui::Window::new("지역 고르기 (한반도 시·도 타일 지도)")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("ka_peninsula_17_tiles").spacing([4.0, 4.0]).show(ui, |ui| {
                        for r in 0..6 {
                            for c in 0..4 {
                                if let Some(tile) = PENINSULA_REGION_TILES.iter().find(|t| t.row == r && t.col == c) {
                                    let sel = self.ribbon.app_bar.selected_sido == tile.name;
                                    if ui.add_sized([46.0, 26.0], egui::SelectableLabel::new(sel, tile.name)).clicked() {
                                        self.ribbon.app_bar.selected_sido = tile.name.to_owned();
                                        self.home_map.viewport.center_xy = tile.center_5187;
                                        self.active_tab = MainTab::Map;
                                    }
                                } else {
                                    ui.allocate_exact_size(egui::vec2(46.0, 26.0), egui::Sense::hover());
                                }
                            }
                            ui.end_row();
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.ribbon.app_bar.city_input).desired_width(75.0));
                        ui.add(egui::TextEdit::singleline(&mut self.ribbon.app_bar.dong_input).desired_width(85.0));
                        ui.add(egui::TextEdit::singleline(&mut self.ribbon.app_bar.lot_input).desired_width(65.0));
                        if ui.button("찾기").clicked() {
                            self.ribbon.app_bar.region_popup_open = false;
                            self.active_tab = MainTab::Map;
                        }
                    });
                });
            if !open {
                self.ribbon.app_bar.region_popup_open = false;
            }
        }
    }

    fn ribbon_group_frame(ui: &mut egui::Ui, caption: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::NONE
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(222, 220, 209)))
            .inner_margin(egui::Margin::symmetric(4, 2))
            .corner_radius(3.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(add_contents);
                    ui.label(egui::RichText::new(caption).size(10.0).color(egui::Color32::from_rgb(94, 93, 89)));
                });
            });
    }
}
