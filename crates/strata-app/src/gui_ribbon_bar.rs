use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_net::tile_fetcher::BasemapSource;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{SubToolsMode, PENINSULA_REGION_TILES};
use crate::ribbon_icons::paint_ribbon_icon;

impl StrataAppState {
    /// C++ `MainWindowRibbon.cpp` + `KaBeginnerRibbon.cpp` 1:1 (Images 1, 2, 4, 9, 10, 11, 12)
    pub(crate) fn render_cpp_top_bars(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let is_home = self.active_tab == MainTab::Home;
        let dirty_save = !is_home;

        egui::Frame::NONE
            .fill(Color32::from_rgb(245, 243, 236))
            .inner_margin(egui::Margin::symmetric(8, 5))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let groups: &[(&str, &[&str])] = &[
                        ("조사", &["새 조사", "열기", "저장", "다른 이름"]),
                        ("기록", &["선택", "측거", "그리기", "시굴격자", "버퍼"]),
                        ("자료 받기", &["지적", "수치지형", "유산"]),
                        ("배경 지도", &["지형", "DEM", "등고선", "토양", "고지형", "지질", "수계", "옛 지도"]),
                        ("정합", &["정합"]),
                        ("내보내기", &["도면", "인쇄", "GeoTIFF", "검수·제출"]),
                        ("기타", &["더보기"]),
                    ];

                    for (g_idx, (g_title, items)) in groups.iter().enumerate() {
                        if g_idx > 0 {
                            let (_, sep_r) = ui.allocate_space(Vec2::new(2.0, 66.0));
                            ui.painter().line_segment(
                                [Pos2::new(sep_r.center().x, sep_r.top() + 2.0), Pos2::new(sep_r.center().x, sep_r.bottom() - 2.0)],
                                Stroke::new(1.0, Color32::from_rgb(214, 211, 200)),
                            );
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(egui::RichText::new(*g_title).size(10.5).strong().color(Color32::from_rgb(65, 64, 60)));
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 3.0;
                                for &label in *items {
                                    let muted = is_home && *g_title == "기록";
                                    let highlight_save = label == "저장" && dirty_save;
                                    let dark_active = (label == "측거" && self.active_tool == MapToolKind::Measure)
                                        || (label == "그리기" && self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw)
                                        || (label == "버퍼" && self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Buffer);
                                    let warm_active = label == "DEM" && !is_home;
                                    if self.draw_ribbon_chip(ui, label, muted, highlight_save, dark_active, warm_active) {
                                        self.handle_ribbon_click(label);
                                    }
                                }
                            });
                        });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.query)
                                .hint_text("Q 주소·지번 찾기")
                                .desired_width(142.0),
                        );
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.active_tab = MainTab::Map;
                            self.home_map.draw_guide_message = Some(format!("주소·지번 「{}」 위치로 이동했습니다.", self.ribbon.app_bar.query));
                        }
                        let reg_btn = egui::Button::new(egui::RichText::new("● 지역").size(12.0).strong())
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(60.0, 30.0));
                        if ui.add(reg_btn).clicked() {
                            self.ribbon.app_bar.region_popup_open = !self.ribbon.app_bar.region_popup_open;
                        }
                    });
                });
            });

        if self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode != SubToolsMode::None {
            egui::Frame::NONE
                .fill(Color32::from_rgb(245, 243, 236))
                .stroke(Stroke::new(1.0, Color32::from_rgb(218, 215, 205)))
                .inner_margin(egui::Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        match self.ribbon.sub_mode {
                            SubToolsMode::Draw => {
                                ui.label(egui::RichText::new("그리기 ›").size(12.5).strong());
                                let _ = ui.button("▶ 도형선택");
                                ui.separator();
                                let mut snap_on = true;
                                let mut shared_on = true;
                                ui.checkbox(&mut snap_on, "자석");
                                ui.checkbox(&mut shared_on, "공유 경계 허용치");
                                let _ = ui.button("12 px   ↕");
                                let _ = ui.button("모든 조사 레이어 ▼");
                                let _ = ui.button("▼");
                                ui.separator();
                                for b_lbl in ["／ 쉽게그리기", "□ 조사구역 그리기", "◇ 유구 그리기", "━ 유구 선", "● 유물위치표시"] {
                                    if ui.button(b_lbl).clicked() {
                                        self.active_tab = MainTab::Map;
                                        self.active_tool = MapToolKind::DrawPolygon;
                                    }
                                }
                                ui.separator();
                                let _ = ui.button("연속 그리기");
                                ui.separator();
                                for op in ["폴리곤 묶기", "폴리곤 나누기", "구간 분리", "겹친 곳 지우기"] {
                                    let _ = ui.button(op);
                                }
                                if ui.button("닫기").clicked() {
                                    self.ribbon.sub_toolbar_visible = false;
                                }
                            }
                            SubToolsMode::Buffer => {
                                ui.label(egui::RichText::new("주변유적경계 ›").size(13.0).strong());
                                for b_lbl in ["◎ 500m", "◎ 1000m", "⬇ 엑셀 내보내기"] {
                                    let btn = egui::Button::new(egui::RichText::new(b_lbl).size(12.5)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(175, 172, 162))).corner_radius(6.0).min_size(Vec2::new(82.0, 28.0));
                                    let _ = ui.add(btn);
                                }
                                if ui.add(egui::Button::new("닫기").fill(Color32::WHITE).corner_radius(6.0).min_size(Vec2::new(52.0, 28.0))).clicked() {
                                    self.ribbon.sub_toolbar_visible = false;
                                }
                            }
                            SubToolsMode::Align => {
                                ui.strong("도면정합 ›");
                                if ui.button("기준점 CSV 가져오기").clicked() {
                                    self.import_demo_dxf_and_csv();
                                }
                            }
                            SubToolsMode::None => {}
                        }
                    });
                });
        }

        egui::Frame::NONE
            .fill(Color32::from_rgb(239, 237, 228))
            .inner_margin(egui::Margin::symmetric(12, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for (tab, icon_txt) in [(MainTab::Home, "홈"), (MainTab::Map, "지도")] {
                        if ui.selectable_label(self.active_tab == tab, egui::RichText::new(icon_txt).size(13.0).strong()).clicked() {
                            self.active_tab = tab;
                        }
                    }
                    for extra in [MainTab::Drawing, MainTab::Paleo, MainTab::Terrain3d] {
                        if self.active_tab == extra {
                            let lbl = match extra {
                                MainTab::Drawing => "도면   ×",
                                MainTab::Paleo => "고지형   ×",
                                _ => "입체지형   ×",
                            };
                            let _ = ui.selectable_label(true, egui::RichText::new(lbl).size(13.0).strong());
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.button(egui::RichText::new("테마 ▼").size(11.5));
                        if !is_home {
                            ui.label(egui::RichText::new("● 저장 안 됨").size(12.0).color(Color32::from_rgb(181, 87, 58)));
                            ui.label(egui::RichText::new(&self.project_title).size(12.0).color(Color32::from_rgb(90, 89, 84)));
                        }
                    });
                });
            });

        self.render_region_locator_popup(ctx);
    }

    fn draw_ribbon_chip(&self, ui: &mut egui::Ui, label: &str, muted: bool, highlight_save: bool, dark_active: bool, warm_active: bool) -> bool {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(43.0, 54.0), egui::Sense::click());
        let p = ui.painter_at(rect);
        let icon_box = Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 18.0), Vec2::new(35.0, 33.0));
        if highlight_save {
            p.rect_filled(icon_box, 7.0, Color32::from_rgb(181, 87, 58));
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
            p.circle_filled(Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5), 4.0, Color32::WHITE);
            p.circle_filled(Pos2::new(icon_box.right() - 5.5, icon_box.top() + 5.5), 2.6, Color32::from_rgb(235, 152, 34));
        } else if dark_active {
            p.rect_filled(icon_box, 7.0, Color32::from_rgb(44, 43, 41));
            paint_ribbon_icon(&p, icon_box, label, Color32::WHITE);
        } else {
            let bg = if warm_active || resp.hovered() { Color32::from_rgb(220, 215, 203) } else { Color32::from_rgb(236, 233, 224) };
            let border = if warm_active { Stroke::new(1.5, Color32::from_rgb(142, 138, 128)) } else { Stroke::new(1.0, Color32::from_rgb(212, 209, 199)) };
            p.rect_filled(icon_box, 7.0, bg);
            p.rect_stroke(icon_box, 7.0, border, egui::StrokeKind::Middle);
            let stroke_col = if muted { Color32::from_rgb(155, 153, 146) } else { Color32::from_rgb(38, 37, 34) };
            paint_ribbon_icon(&p, icon_box, label, stroke_col);
        }
        let txt_col = if muted { Color32::from_rgb(150, 148, 140) } else { Color32::from_rgb(30, 30, 28) };
        p.text(Pos2::new(rect.center().x, rect.bottom() - 6.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(11.0), txt_col);
        resp.clicked()
    }

    fn handle_ribbon_click(&mut self, label: &str) {
        match label {
            "새 조사" => self.start_new_survey_project("경주시"),
            "열기" => self.open_sample_survey_project(0),
            "저장" | "다른 이름" => self.home_map.draw_guide_message = Some(format!("「{}」 조사를 안전하게 저장했습니다.", self.project_title)),
            "선택" => { self.active_tab = MainTab::Map; self.active_tool = MapToolKind::Select; }
            "측거" => self.trigger_map_ribbon_tool(MapToolKind::Measure),
            "그리기" => { self.active_tab = MainTab::Map; self.ribbon.toggle_sub_mode(SubToolsMode::Draw); self.active_tool = MapToolKind::DrawPolygon; }
            "시굴격자" => self.modals.active = Some(ActiveModalDialog::TrenchGenerator),
            "버퍼" => { self.active_tab = MainTab::Map; self.ribbon.toggle_sub_mode(SubToolsMode::Buffer); }
            "DEM" => { self.active_tab = MainTab::Map; self.modals.active = Some(ActiveModalDialog::DemDownload); }
            "지적" => { self.active_tab = MainTab::Map; self.left_layers.add_cadastral_wms_layer(); self.modals.active = Some(ActiveModalDialog::CadastralDownload); }
            "수치지형" => { self.active_tab = MainTab::Map; self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload); }
            "유산" => { self.active_tab = MainTab::Map; self.modals.active = Some(ActiveModalDialog::HeritageRegionSelect); }
            "지형" => { self.active_tab = MainTab::Map; self.home_map.active_basemap = BasemapSource::VworldBase; }
            "등고선" => { self.active_tab = MainTab::Map; self.left_layers.ensure_domain_layer(DomainLayerKey::SectionLine); }
            "고지형" => self.active_tab = MainTab::Paleo,
            "도면" => self.modals.active = Some(ActiveModalDialog::PaperSetup),
            "인쇄" => { self.active_tab = MainTab::Drawing; self.modals.active = Some(ActiveModalDialog::DrawingPrint); }
            "GeoTIFF" => self.active_tab = MainTab::Drawing,
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            "더보기" => self.active_tab = MainTab::Terrain3d,
            _ => self.active_tab = MainTab::Map,
        }
    }

    fn render_region_locator_popup(&mut self, ctx: &egui::Context) {
        if !self.ribbon.app_bar.region_popup_open { return; }
        let mut open = self.ribbon.app_bar.region_popup_open;
        egui::Window::new("한반도 17개 광역 시·도 선택").open(&mut open).collapsible(false).resizable(false).default_width(300.0).show(ctx, |ui| {
            egui::Grid::new("ka_peninsula_17_grid").spacing([6.0, 6.0]).show(ui, |ui| {
                for r in 0..6 {
                    for c in 0..4 {
                        if let Some(tile) = PENINSULA_REGION_TILES.iter().find(|t| t.row == r && t.col == c) {
                            if ui.add_sized([62.0, 28.0], egui::Button::new(tile.name)).clicked() {
                                self.ribbon.app_bar.selected_sido = tile.name.to_owned();
                                self.home_map.viewport.center_xy = tile.center_5187;
                                self.active_tab = MainTab::Map;
                                self.ribbon.app_bar.region_popup_open = false;
                            }
                        } else {
                            ui.allocate_space(Vec2::new(62.0, 28.0));
                        }
                    }
                    ui.end_row();
                }
            });
        });
        self.ribbon.app_bar.region_popup_open = open;
    }
}
