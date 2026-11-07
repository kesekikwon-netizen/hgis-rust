use eframe::egui;
use strata_net::tile_fetcher::{AsyncTileDispatcher, BasemapSource};
use strata_types::schema::DomainLayerKey;

use crate::app_state::StrataAppState;
use crate::right_inspector::InspectorCardTab;

impl StrataAppState {
    /// `MainWindow.cpp:790-1050` + `MainWindowChrome.cpp` — `지도` 탭 전용 3단 분할(`leftSplit` | `mapCard` | `KaInspectorPanel`)
    pub(crate) fn render_map_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. 우측 KaInspectorPanel (오직 지도 탭 내부에서만 표시)
        egui::SidePanel::right("ka_map_inspector_panel")
            .resizable(true)
            .default_width(276.0)
            .min_width(240.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("선택한 유구");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(94, 93, 89),
                            format!("{}개 선택", self.home_map.scene.polygons.len()),
                        );
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.right_inspector.active_card,
                        InspectorCardTab::FeatureAttributeCard,
                        "속성",
                    );
                    ui.selectable_value(
                        &mut self.right_inspector.active_card,
                        InspectorCardTab::LayerStyleCard,
                        "스타일",
                    );
                });
                ui.separator();
                match self.right_inspector.active_card {
                    InspectorCardTab::FeatureAttributeCard | InspectorCardTab::BasemapQuickCard => {
                        ui.label(
                            egui::RichText::new("그리는 중에도 여기서 바로 고칠 수 있습니다. 지도를 누르면 이어서 그립니다.")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(94, 93, 89)),
                        );
                        ui.add_space(6.0);
                        egui::Frame::group(ui.style())
                            .fill(egui::Color32::WHITE)
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.strong("유구·조사구역 조사카드 (KaFeatureCard)");
                                ui.separator();
                                ui.label(format!("조사명: {}", self.project_title));
                                ui.label(format!("작업 좌표계: {}", self.work_crs.label_ko()));
                                ui.label("지목·지번: 경주시 인왕동 76-1전 외 4필지");
                                ui.label("주변 국가유산: 반경 500m 내 3개소 감지");
                            });
                    }
                    InspectorCardTab::LayerStyleCard => {
                        ui.strong("선택 레이어 스타일 (KaInspectorStyle)");
                        ui.add(egui::Slider::new(&mut self.right_inspector.stroke_width_mm, 0.1..=2.5).text("선 두께(mm)"));
                        ui.add(egui::Slider::new(&mut self.right_inspector.fill_opacity, 0.0..=1.0).text("면 불투명도"));
                    }
                }
                ui.add_space(10.0);
                ui.separator();
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(245, 244, 237))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.strong("배경 지도");
                        ui.horizontal_wrapped(|ui| {
                            for (src, label) in [
                                (BasemapSource::VworldSatellite, "위성"),
                                (BasemapSource::VworldBase, "지형(기본도)"),
                                (BasemapSource::VworldCadastralWms, "연속지적도"),
                                (BasemapSource::Daedongyeojido, "대동여지도"),
                                (BasemapSource::History1919, "1919 지형도"),
                            ] {
                                if ui.selectable_label(self.home_map.active_basemap == src, label).clicked() {
                                    self.home_map.active_basemap = src;
                                }
                            }
                        });
                    });
            });

        // 2. 좌측 leftSplit: 상단 레이어(m_layersCard) + 하단 파일함(m_filesPanel)
        egui::SidePanel::left("ka_map_left_split_panel")
            .resizable(true)
            .default_width(268.0)
            .min_width(210.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("레이어");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("전체 끄기/켜기").clicked() {
                            let any_on = self.left_layers.entries.iter().any(|e| e.visible);
                            for e in &mut self.left_layers.entries {
                                e.visible = !any_on;
                            }
                        }
                        ui.toggle_value(&mut self.home_map.files_open, "파일함");
                    });
                });
                ui.add(egui::TextEdit::singleline(&mut self.home_map.layer_filter).hint_text("🔍 레이어 찾기 · 편집 중 ✎").desired_width(f32::INFINITY));
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("+ 조사구역").clicked() {
                        self.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
                    }
                    if ui.small_button("+ 시굴격자").clicked() {
                        self.left_layers.ensure_domain_layer(DomainLayerKey::TrialTrench);
                    }
                    if ui.small_button("+ 연속지적도").clicked() {
                        self.left_layers.add_cadastral_wms_layer();
                    }
                });
                ui.separator();
                if self.left_layers.entries.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(14.0);
                        ui.label(
                            egui::RichText::new("레이어가 없습니다.\n파일함에서 SHP·DXF·DWG를 끌어 넣거나\n위성·지적 배경을 올리세요.")
                                .color(egui::Color32::from_rgb(94, 93, 89)),
                        );
                        ui.add_space(14.0);
                    });
                } else {
                    let mut remove_key: Option<String> = None;
                    let filter = self.home_map.layer_filter.trim().to_lowercase();
                    for entry in &mut self.left_layers.entries {
                        if !filter.is_empty() && !entry.label_ko.to_lowercase().contains(&filter) {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut entry.visible, &entry.label_ko);
                            ui.add(egui::Slider::new(&mut entry.opacity, 0.1..=1.0).show_value(false));
                            if ui.small_button("✕").clicked() {
                                remove_key = Some(entry.layer_key.clone());
                            }
                        });
                    }
                    if let Some(k) = remove_key {
                        self.left_layers.remove_by_key(&k);
                    }
                }

                if self.home_map.files_open {
                    ui.separator();
                    ui.strong("파일함 (문서 › hgis)");
                    for (fname, key) in [
                        ("📐 경주_인왕동_측량등고.dxf", DomainLayerKey::SectionLine),
                        ("🗺 경주_월성북편_조사구역.shp", DomainLayerKey::SurveyArea),
                        ("📏 경주_시굴트렌치_계획.shp", DomainLayerKey::TrialTrench),
                        ("📍 경주_인왕동_기준점.csv", DomainLayerKey::ControlPoints),
                        ("🏛 경주시_국가유산_분포.gpkg", DomainLayerKey::FeaturePoly),
                    ] {
                        if ui.selectable_label(false, fname).clicked() {
                            self.left_layers.ensure_domain_layer(key);
                            self.home_map.draw_guide_message = Some(format!("파일함에서 「{fname}」 레이어를 지도에 올렸습니다."));
                        }
                    }
                }
            });

        // 3. 중앙 mapCard
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            let painter = ui.painter_at(rect);
            self.paint_map_canvas_full(ctx, &painter, rect, &response);
        });
    }

    fn paint_map_canvas_full(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: egui::Rect,
        response: &egui::Response,
    ) {
        self.home_map.viewport.screen_size_px = [f64::from(rect.width()), f64::from(rect.height())];
        let bg_rgba = AsyncTileDispatcher::synthesize_basemap_backdrop_rgba(self.home_map.active_basemap, 128, 128);
        let img = egui::ColorImage::from_rgba_unmultiplied([128, 128], &bg_rgba);
        let tex = ctx.load_texture("map_basemap_bg", img, egui::TextureOptions::NEAREST);
        painter.image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);

        if response.dragged() {
            let delta = response.drag_delta();
            self.home_map.viewport.pan_by_pixels(f64::from(delta.x), f64::from(delta.y));
        }
        if let Some(pos) = response.hover_pos() {
            let local = [f64::from(pos.x - rect.left()), f64::from(pos.y - rect.top())];
            self.home_map.cursor_world_xy = self.home_map.viewport.screen_to_world(local);
            let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
            self.home_map.cad.on_wheel_zoom(scroll_y, local, &mut self.home_map.viewport);
        }
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = [f64::from(pos.x - rect.left()), f64::from(pos.y - rect.top())];
                let w = self.home_map.viewport.screen_to_world(local);
                self.home_map.cad.on_left_click(w, self.active_tool, &mut self.home_map.scene);
                self.is_dirty = true;
            }
        }
        if response.secondary_clicked() && self.home_map.cad.on_right_click(self.active_tool, &mut self.home_map.scene) {
            self.is_dirty = true;
        }

        if self.home_map.grid_enabled {
            let step_px = 120.0_f32;
            let mut gx = rect.left() + step_px;
            while gx < rect.right() {
                painter.line_segment([egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())], egui::Stroke::new(0.8, egui::Color32::from_rgba_unmultiplied(80, 78, 72, 45)));
                gx += step_px;
            }
            let mut gy = rect.top() + step_px;
            while gy < rect.bottom() {
                painter.line_segment([egui::pos2(rect.left(), gy), egui::pos2(rect.right(), gy)], egui::Stroke::new(0.8, egui::Color32::from_rgba_unmultiplied(80, 78, 72, 45)));
                gy += step_px;
            }
        }

        for prim in self.home_map.scene.project_polygons(&self.home_map.viewport) {
            let pts: Vec<egui::Pos2> = prim.screen_pts.iter().map(|p| egui::pos2(rect.left() + p[0], rect.top() + p[1])).collect();
            let sc = egui::Color32::from_rgba_unmultiplied(prim.stroke_rgba[0], prim.stroke_rgba[1], prim.stroke_rgba[2], prim.stroke_rgba[3]);
            painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(prim.stroke_width_px, sc)));
        }

        if self.home_map.scene.active_rubberband.len() >= 2 {
            let rb_pts: Vec<egui::Pos2> = self.home_map.scene.active_rubberband.iter().map(|w| {
                let s = self.home_map.viewport.world_to_screen(*w);
                egui::pos2(rect.left() + s[0] as f32, rect.top() + s[1] as f32)
            }).collect();
            painter.add(egui::Shape::line(rb_pts, egui::Stroke::new(2.2, egui::Color32::from_rgb(181, 87, 58))));
        }

        for (num, world_xy, label) in &self.home_map.scene.heritage_badges_world {
            let s = self.home_map.viewport.world_to_screen(*world_xy);
            let pos = egui::pos2(rect.left() + s[0] as f32, rect.top() + s[1] as f32);
            if rect.contains(pos) {
                painter.circle_filled(pos, 11.0, egui::Color32::from_rgb(181, 87, 58));
                painter.text(pos, egui::Align2::CENTER_CENTER, num.to_string(), egui::FontId::proportional(11.5), egui::Color32::WHITE);
                painter.text(pos + egui::vec2(15.0, 0.0), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(12.5), egui::Color32::from_rgb(20, 20, 19));
            }
        }

        let band_rect = egui::Rect::from_min_size(rect.min + egui::vec2(14.0, 10.0), egui::vec2(430.0, 28.0));
        painter.rect_filled(band_rect, 4.0, egui::Color32::from_rgba_unmultiplied(250, 249, 245, 235));
        let hint = self.home_map.draw_guide_message.as_deref().unwrap_or("클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)");
        painter.text(band_rect.min + egui::vec2(10.0, 6.0), egui::Align2::LEFT_TOP, hint, egui::FontId::proportional(12.0), egui::Color32::from_rgb(20, 20, 19));
    }
}
