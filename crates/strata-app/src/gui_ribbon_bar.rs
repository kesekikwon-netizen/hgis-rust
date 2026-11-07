use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_net::tile_fetcher::BasemapSource;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{SubToolsMode, PENINSULA_REGION_TILES};
use crate::ribbon_icons::{cpp_ribbon_tooltip, paint_ribbon_icon};

fn draw_buffer_pill(ui: &mut egui::Ui, icon_key: &str, badge_bg: bool, text: &str, width: f32) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 28.0), egui::Sense::click());
    let p = ui.painter_at(r);
    p.rect_filled(r, 6.0, Color32::WHITE);
    p.rect_stroke(r, 6.0, Stroke::new(1.0, Color32::from_rgb(160, 158, 150)), egui::StrokeKind::Middle);
    if !icon_key.is_empty() {
        let ir = Rect::from_center_size(Pos2::new(r.left() + 16.0, r.center().y), Vec2::new(20.0, 20.0));
        if badge_bg {
            p.rect_filled(ir, 4.0, Color32::from_rgb(236, 233, 224));
        }
        paint_ribbon_icon(&p, ir. expand(2.0), icon_key, Color32::from_rgb(38, 37, 34));
        p.text(Pos2::new(r.left() + 31.0, r.center().y), egui::Align2::LEFT_CENTER, text, egui::FontId::proportional(12.5), Color32::from_rgb(30, 30, 28));
    } else {
        p.text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(12.5), Color32::from_rgb(30, 30, 28));
    }
    resp.clicked()
}

impl StrataAppState {
    /// C++ `MainWindowRibbon.cpp` + `KaBeginnerRibbon.cpp` 1:1 (Images 1, 2, 4, 9, 10, 11, 12, 15)
    pub(crate) fn render_cpp_top_bars(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let is_home = self.active_tab == MainTab::Home;
        let dirty_save = !is_home && self.is_dirty;

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
                                        || (label == "그리기" && (self.active_tool == MapToolKind::Measure || (self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw)))
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
                        ui.spacing_mut().item_spacing.x = 5.0;
                        match self.ribbon.sub_mode {
                            SubToolsMode::Draw => {
                                let wbtn = |ui: &mut egui::Ui, txt: &str| -> egui::Response {
                                    ui.add(egui::Button::new(egui::RichText::new(txt).size(12.0)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172))).corner_radius(5.0).min_size(Vec2::new(0.0, 26.0)))
                                };
                                ui.label(egui::RichText::new("그리기 ›").size(12.5).strong());
                                if wbtn(ui, "▶ 도형선택").clicked() {
                                    self.active_tab = MainTab::Map;
                                    self.active_tool = MapToolKind::Select;
                                }
                                ui.separator();
                                let mut shared_on = true;
                                ui.checkbox(&mut self.home_map.cad.snap_enabled, "자석");
                                ui.checkbox(&mut shared_on, "공유 경계 허용치");
                                let _ = wbtn(ui, "12 px   ↕");
                                let _ = wbtn(ui, "모든 조사 레이어 ▼");
                                let assist_resp = ui
                                    .add(
                                        egui::Button::new(egui::RichText::new("▼").size(11.5))
                                            .fill(Color32::WHITE)
                                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                            .corner_radius(5.0)
                                            .min_size(Vec2::new(31.0, 26.0)),
                                    )
                                    .on_hover_text(
                                        "자석 보조 — 선 따라 자동채움 · 근접 자석점 표시 · 가까운 선에 붙이기",
                                    );
                                if assist_resp.clicked() {
                                    self.ribbon.snap_assist_menu_open = !self.ribbon.snap_assist_menu_open;
                                }
                                if self.ribbon.snap_assist_menu_open {
                                    let popup_pos = Pos2::new(assist_resp.rect.left(), assist_resp.rect.bottom() + 4.0);
                                    egui::Area::new(ui.id().with("ka_snap_assist_menu"))
                                        .fixed_pos(popup_pos)
                                        .order(egui::Order::Foreground)
                                        .show(ctx, |ui| {
                                            egui::Frame::popup(ui.style())
                                                .fill(Color32::from_rgb(252, 251, 247))
                                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                                .inner_margin(egui::Margin::symmetric(8, 6))
                                                .show(ui, |ui| {
                                                    ui.set_min_width(188.0);
                                                    let snap_now = ui
                                                        .button("가까운 선에 붙이기")
                                                        .on_hover_text(
                                                            "지적도·사업자 도면 위에 그린 도형(방금 그린 도형 또는 선택한 도형)을\n가장 가까운 지적선·도면선에 자동으로 붙이고 빠진 사이 꺾임점도 채웁니다 (Ctrl+Z 되돌리기 가능).",
                                                        );
                                                    if snap_now.clicked() {
                                                        let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                                        if changed > 0 {
                                                            self.is_dirty = true;
                                                        }
                                                        self.ribbon.snap_assist_menu_open = false;
                                                    }
                                                    ui.separator();
                                                    let prev_trace = self.home_map.cad.snap_trace_enabled;
                                                    if ui
                                                        .checkbox(&mut self.home_map.cad.snap_trace_enabled, "선 따라 자동채움")
                                                        .on_hover_text(
                                                            "1번 기능(선 따라 빠진 사이 자석점 자동 채우기)\n· 그릴 때 자석점 한두 칸을 건너뛰어도 사이 꼭짓점을 선을 따라 자동으로 채우며,\n  이미 그린 도형이 있을 때 누르면 가장 가까운 선에 바로 붙입니다.",
                                                        )
                                                        .changed()
                                                    {
                                                        let on = self.home_map.cad.snap_trace_enabled;
                                                        if on && !self.home_map.cad.snap_enabled {
                                                            self.home_map.cad.snap_enabled = true;
                                                        }
                                                        if on && !prev_trace {
                                                            let (changed, _, _) = self.home_map.snap_drawn_shapes_to_nearest_lines();
                                                            if changed == 0 {
                                                                self.home_map.draw_guide_message = Some(
                                                                    "선 따라 자동채움 켜짐 — 자석점을 한두 칸 건너뛰어 찍어도 사이 꼭짓점을 자동으로 채웁니다."
                                                                        .to_owned(),
                                                                );
                                                            } else {
                                                                self.is_dirty = true;
                                                            }
                                                        } else if !on {
                                                            self.home_map.draw_guide_message = Some("선 따라 자동채움 꺼짐".to_owned());
                                                        }
                                                    }
                                                    if ui
                                                        .checkbox(&mut self.home_map.cad.snap_preview_enabled, "근접 자석점 표시")
                                                        .on_hover_text(
                                                            "3번 기능(마우스가 가까이 갔을 때만 주변 자석점 미리 표시)\n· 마우스가 선·꼭짓점에 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다.",
                                                        )
                                                        .changed()
                                                    {
                                                        let on = self.home_map.cad.snap_preview_enabled;
                                                        if on && !self.home_map.cad.snap_enabled {
                                                            self.home_map.cad.snap_enabled = true;
                                                        }
                                                        self.home_map.draw_guide_message = Some(if on {
                                                            "근접 자석점 표시 켜짐 — 마우스가 가까이 갔을 때만 주변 자석점을 미리 보여 줍니다."
                                                                .to_owned()
                                                        } else {
                                                            "근접 자석점 표시 꺼짐".to_owned()
                                                        });
                                                    }
                                                });
                                        });
                                }
                                ui.separator();
                                for (b_lbl, tool) in [("／ 쉽게그리기", MapToolKind::DrawPolygon), ("□ 조사구역 그리기", MapToolKind::DrawPolygon), ("◇ 유구 그리기", MapToolKind::DrawPolygon), ("━ 유구 선", MapToolKind::DrawLine), ("● 유물위치표시", MapToolKind::PlacePoint)] {
                                    if wbtn(ui, b_lbl).clicked() {
                                        self.active_tab = MainTab::Map;
                                        self.active_tool = tool;
                                        self.home_map.update_guide_for_tool(tool);
                                    }
                                }
                                ui.separator();
                                let _ = wbtn(ui, "연속 그리기");
                                ui.separator();
                                for op in ["폴리곤 묶기", "폴리곤 나누기", "구간 분리", "겹친 곳 지우기"] {
                                    if wbtn(ui, op).clicked() {
                                        self.home_map.draw_guide_message = Some(format!("「{op}」 도구를 실행할 도형을 지도에서 선택하세요."));
                                    }
                                }
                                if wbtn(ui, "닫기").clicked() { self.ribbon.sub_toolbar_visible = false; }
                            }
                            SubToolsMode::Buffer => {
                                ui.label(egui::RichText::new("주변유적경계 ›").size(12.5).strong());
                                if draw_buffer_pill(ui, "버퍼", true, "500m", 78.0) {
                                    self.home_map.real_survey.toggle_layer_visible("주변 500m");
                                    self.home_map.draw_guide_message = Some("조사구역 주변 500m 경계를 갱신했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "버퍼", true, "1000m", 84.0) {
                                    self.home_map.real_survey.show_buffer_1000m = !self.home_map.real_survey.show_buffer_1000m;
                                    self.home_map.draw_guide_message = Some("조사구역 주변 1,000m 경계를 지도에 표시했습니다.".to_owned());
                                }
                                if draw_buffer_pill(ui, "수치지형", false, "엑셀 내보내기", 118.0) {
                                    if let Some(out) = self.home_map.real_survey.export_nearby_heritage_csv() {
                                        self.home_map.draw_guide_message = Some(format!("주변유적 목록을 내보냈습니다: {}", out.display()));
                                    }
                                }
                                if draw_buffer_pill(ui, "", false, "닫기", 48.0) { self.ribbon.sub_toolbar_visible = false; }
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
            .inner_margin(egui::Margin::symmetric(12, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for (tab, ico, txt) in [(MainTab::Home, "유산", "홈"), (MainTab::Map, "옛 지도", "지도")] {
                        let sel = self.active_tab == tab;
                        let (tr, resp) = ui.allocate_exact_size(Vec2::new(56.0, 24.0), egui::Sense::click());
                        let tp = ui.painter_at(tr);
                        paint_ribbon_icon(&tp, Rect::from_center_size(Pos2::new(tr.left() + 10.0, tr.center().y), Vec2::new(20.0, 20.0)), ico, Color32::from_rgb(45, 44, 40));
                        tp.text(Pos2::new(tr.left() + 24.0, tr.center().y), egui::Align2::LEFT_CENTER, txt, egui::FontId::proportional(12.5), Color32::from_rgb(30, 30, 28));
                        if sel {
                            tp.line_segment([Pos2::new(tr.left(), tr.bottom()), Pos2::new(tr.right(), tr.bottom())], Stroke::new(2.0, Color32::from_rgb(35, 35, 32)));
                        }
                        if resp.clicked() { self.active_tab = tab; }
                    }
                    let open_tabs = self.open_studio_tabs.clone();
                    for (extra, ico, lbl) in [
                        (MainTab::Drawing, "도면", "도면"),
                        (MainTab::Paleo, "고지형", "고지형"),
                        (MainTab::Terrain3d, "검수·제출", "입체지형"),
                    ] {
                        if open_tabs.contains(&extra) || self.active_tab == extra {
                            let sel = self.active_tab == extra;
                            let (tr, resp) = ui.allocate_exact_size(Vec2::new(86.0, 24.0), egui::Sense::click());
                            let tp = ui.painter_at(tr);
                            paint_ribbon_icon(
                                &tp,
                                Rect::from_center_size(Pos2::new(tr.left() + 10.0, tr.center().y), Vec2::new(20.0, 20.0)),
                                ico,
                                Color32::from_rgb(35, 35, 32),
                            );
                            tp.text(
                                Pos2::new(tr.left() + 24.0, tr.center().y),
                                egui::Align2::LEFT_CENTER,
                                lbl,
                                egui::FontId::proportional(12.5),
                                Color32::from_rgb(30, 30, 28),
                            );
                            let close_rect = Rect::from_min_max(Pos2::new(tr.right() - 18.0, tr.top()), tr.max);
                            tp.text(
                                close_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "×",
                                egui::FontId::proportional(13.0),
                                Color32::from_rgb(90, 89, 84),
                            );
                            if sel {
                                tp.line_segment(
                                    [Pos2::new(tr.left(), tr.bottom()), Pos2::new(tr.right(), tr.bottom())],
                                    Stroke::new(2.0, Color32::from_rgb(35, 35, 32)),
                                );
                            }
                            if resp.clicked() {
                                if resp.interact_pointer_pos().is_some_and(|p| close_rect.contains(p)) {
                                    self.close_studio_tab(extra);
                                } else {
                                    self.active_tab = extra;
                                }
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let theme_resp = ui.button(egui::RichText::new("테마 ▼").size(11.5));
                        if theme_resp.clicked() {
                            self.theme_menu_open = !self.theme_menu_open;
                        }
                        if self.theme_menu_open {
                            let popup_pos = Pos2::new((theme_resp.rect.right() - 180.0).max(16.0), theme_resp.rect.bottom() + 4.0);
                            egui::Area::new(ui.id().with("ka_theme_menu_popup"))
                                .fixed_pos(popup_pos)
                                .order(egui::Order::Foreground)
                                .show(ctx, |ui| {
                                    egui::Frame::popup(ui.style())
                                        .fill(Color32::from_rgb(252, 251, 247))
                                        .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                        .inner_margin(egui::Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.set_min_width(172.0);
                                            if ui.checkbox(&mut self.paper_look, "새 모양 (조용한 도판)").changed() {
                                                self.theme = strata_types::theme_tokens::ThemeTokens::paper_tokens(!self.paper_look);
                                            }
                                            if ui.checkbox(&mut self.large_font_mode, "큰 글씨 모드").changed() {
                                                ctx.set_pixels_per_point(if self.large_font_mode { 1.15 } else { 1.0 });
                                            }
                                        });
                                });
                        }
                        if !is_home {
                            if self.is_dirty {
                                ui.label(egui::RichText::new("● 저장 안 됨").size(12.0).color(Color32::from_rgb(181, 87, 58)));
                            } else {
                                ui.label(egui::RichText::new("● 저장됨").size(12.0).color(Color32::from_rgb(21, 128, 61)));
                            }
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
        resp.on_hover_text(cpp_ribbon_tooltip(label)).clicked()
    }

    fn handle_ribbon_click(&mut self, label: &str) {
        match label {
            "새 조사" => self.start_new_survey_project("경주시"),
            "열기" => self.open_sample_survey_project(0),
            "저장" | "다른 이름" => {
                self.is_dirty = false;
                self.home_map.draw_guide_message = Some(format!("「{}」 조사를 안전하게 저장했습니다.", self.project_title));
            }
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
            "고지형" => self.open_studio_tab(MainTab::Paleo),
            "도면" => self.modals.active = Some(ActiveModalDialog::PaperSetup),
            "인쇄" => { self.open_studio_tab(MainTab::Drawing); self.modals.active = Some(ActiveModalDialog::DrawingPrint); }
            "GeoTIFF" => self.open_studio_tab(MainTab::Drawing),
            "검수·제출" => self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage),
            "더보기" => self.modals.active = Some(ActiveModalDialog::AccountSettings),
            _ => self.active_tab = MainTab::Map,
        }
    }

    fn render_region_locator_popup(&mut self, ctx: &egui::Context) {
        if !self.ribbon.app_bar.region_popup_open {
            return;
        }
        let mut open = self.ribbon.app_bar.region_popup_open;
        egui::Window::new("한반도 17개 광역 시·도 선택")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::Grid::new("ka_peninsula_17_grid")
                    .spacing([6.0, 6.0])
                    .show(ui, |ui| {
                        for r in 0..6 {
                            for c in 0..4 {
                                if let Some(tile) = PENINSULA_REGION_TILES.iter().find(|t| t.row == r && t.col == c) {
                                    let is_sel = self.ribbon.app_bar.selected_sido == tile.name;
                                    let btn = egui::Button::new(
                                        egui::RichText::new(tile.name)
                                            .size(12.5)
                                            .strong()
                                            .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(30, 30, 28) }),
                                    )
                                    .fill(if is_sel { Color32::from_rgb(44, 43, 41) } else { Color32::WHITE })
                                    .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                    .corner_radius(5.0);
                                    if ui.add_sized([66.0, 28.0], btn).clicked() {
                                        self.ribbon.app_bar.selected_sido = tile.name.to_owned();
                                        self.home_map.viewport.center_xy = tile.center_5187;
                                        self.active_tab = MainTab::Map;
                                    }
                                } else {
                                    ui.allocate_space(Vec2::new(66.0, 28.0));
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                egui::Grid::new("ka_region_detail_inputs")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("시·군·구").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.city_input)
                                .hint_text("예: 경주시")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                        ui.label(egui::RichText::new("읍·면·동").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.dong_input)
                                .hint_text("예: 인왕동")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                        ui.label(egui::RichText::new("지번").size(12.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ribbon.app_bar.lot_input)
                                .hint_text("예: 76 또는 산 12-1")
                                .desired_width(210.0),
                        );
                        ui.end_row();
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let move_btn = egui::Button::new(
                        egui::RichText::new("이동").color(Color32::WHITE).strong().size(13.0),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(5.0)
                    .min_size(Vec2::new(140.0, 30.0));
                    if ui.add(move_btn).clicked() {
                        let full_addr = format!(
                            "{} {} {} {}",
                            self.ribbon.app_bar.selected_sido,
                            self.ribbon.app_bar.city_input.trim(),
                            self.ribbon.app_bar.dong_input.trim(),
                            self.ribbon.app_bar.lot_input.trim()
                        );
                        self.ribbon.app_bar.query = full_addr.trim().to_owned();
                        if let Some(parcel) = self
                            .home_map
                            .real_survey
                            .cadastral_parcels
                            .iter()
                            .find(|p| !self.ribbon.app_bar.lot_input.trim().is_empty() && p.jibun.contains(self.ribbon.app_bar.lot_input.trim()))
                        {
                            self.home_map.viewport.center_xy = parcel.center;
                            self.home_map.viewport.meters_per_pixel = 1.25;
                        } else if let Some(tile) = PENINSULA_REGION_TILES
                            .iter()
                            .find(|t| t.name == self.ribbon.app_bar.selected_sido)
                        {
                            self.home_map.viewport.center_xy = tile.center_5187;
                        }
                        self.active_tab = MainTab::Map;
                        self.home_map.draw_guide_message = Some(format!(
                            "지역·지번 「{}」 위치로 이동했습니다.",
                            self.ribbon.app_bar.query
                        ));
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                    if ui.button("닫기").clicked() {
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                });
            });
        self.ribbon.app_bar.region_popup_open = open;
    }
}
