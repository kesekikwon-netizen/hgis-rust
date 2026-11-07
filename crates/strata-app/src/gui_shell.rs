use eframe::egui::{self, Color32};
use strata_canvas2d::map_tools::MapToolKind;
use strata_types::crs::WorkCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};
use crate::ribbon::SubToolsMode;

impl StrataAppState {
    /// C++ `MainWindow.cpp` + `MainWindowRibbon.cpp` + `KaShellFocus.h` 단축키 1:1 처리
    fn handle_cpp_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_kb = ctx.wants_keyboard_input();
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                self.home_map.cad.on_escape(&mut self.home_map.scene);
                self.modals.active = None;
                self.ribbon.sub_toolbar_visible = false;
                self.active_tool = MapToolKind::Pan;
            }
            if i.key_pressed(egui::Key::F9) {
                self.right_inspector.left_panel_collapsed =
                    !self.right_inspector.left_panel_collapsed;
                self.home_map.draw_guide_message = Some(
                    if self.right_inspector.left_panel_collapsed {
                        "왼쪽 패널을 접었습니다. F9 키를 누르면 다시 펼칩니다."
                    } else {
                        "왼쪽 패널을 펼쳤습니다."
                    }
                    .to_owned(),
                );
            }
            if i.key_pressed(egui::Key::F10) {
                self.right_inspector.right_panel_collapsed =
                    !self.right_inspector.right_panel_collapsed;
                self.home_map.draw_guide_message = Some(
                    if self.right_inspector.right_panel_collapsed {
                        "오른쪽 패널을 접었습니다. F10 키를 누르면 다시 펼칩니다."
                    } else {
                        "오른쪽 패널을 펼쳤습니다."
                    }
                    .to_owned(),
                );
            }
            if i.key_pressed(egui::Key::F11) {
                self.right_inspector.map_focused = !self.right_inspector.map_focused;
                self.home_map.draw_guide_message = Some(
                    if self.right_inspector.map_focused {
                        "지도 넓게 보기 — Ctrl+F11 키를 다시 누르면 리본과 왼쪽 패널이 돌아옵니다."
                    } else {
                        "지도 넓게 보기를 끝냈습니다."
                    }
                    .to_owned(),
                );
            }
            if !wants_kb && i.modifiers.is_none() && i.key_pressed(egui::Key::A) {
                self.active_tab = MainTab::Map;
                self.active_tool = MapToolKind::Select;
            }
            if !wants_kb
                && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            {
                self.home_map.cad.on_backspace(&mut self.home_map.scene);
            }
            if !wants_kb && self.active_tab == MainTab::Map {
                let arrow = if i.key_pressed(egui::Key::ArrowLeft) {
                    Some(strata_geo::trench_grid::TrenchArrowKey::Left)
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    Some(strata_geo::trench_grid::TrenchArrowKey::Right)
                } else if i.key_pressed(egui::Key::ArrowUp) {
                    Some(strata_geo::trench_grid::TrenchArrowKey::Up)
                } else if i.key_pressed(egui::Key::ArrowDown) {
                    Some(strata_geo::trench_grid::TrenchArrowKey::Down)
                } else {
                    None
                };
                if let Some(key) = arrow {
                    if let Ok(next_origin) =
                        strata_geo::trench_grid::translate_trench_center_with_key(
                            [0.0, 0.0],
                            key,
                            i.modifiers.shift,
                            true,
                        )
                    {
                        self.home_map.real_survey.translate_all_trenches(
                            next_origin[0],
                            next_origin[1],
                            None,
                        );
                        self.is_dirty = true;
                        let step_m = if i.modifiers.shift { 1.0 } else { 0.1 };
                        self.home_map.draw_guide_message = Some(format!(
                            "시굴격자를 화살표 방향으로 {step_m:.1} m 이동했습니다."
                        ));
                    }
                }
            }
            if i.modifiers.ctrl {
                if i.key_pressed(egui::Key::N) {
                    self.start_new_survey_project("경주시");
                } else if i.key_pressed(egui::Key::O) {
                    self.open_sample_survey_project(0);
                } else if i.key_pressed(egui::Key::S) {
                    self.is_dirty = false;
                    self.home_map.draw_guide_message =
                        Some(format!("「{}」 조사를 저장했습니다.", self.project_title));
                } else if i.key_pressed(egui::Key::Num1) {
                    self.active_tab = MainTab::Map;
                    self.active_tool = MapToolKind::Select;
                } else if i.key_pressed(egui::Key::D) {
                    self.active_tab = MainTab::Map;
                    self.ribbon.toggle_sub_mode(SubToolsMode::Draw);
                    self.active_tool = MapToolKind::DrawPolygon;
                } else if i.key_pressed(egui::Key::L) {
                    self.modals.active = Some(ActiveModalDialog::PaperSetup);
                } else if i.key_pressed(egui::Key::P) {
                    self.open_studio_tab(MainTab::Drawing);
                    self.modals.active = Some(ActiveModalDialog::DrawingPrint);
                } else if i.key_pressed(egui::Key::E) {
                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                } else if i.key_pressed(egui::Key::Z) {
                    self.home_map.draw_guide_message =
                        Some("마지막 편집을 되돌렸습니다 (Ctrl+Z).".to_owned());
                } else if i.key_pressed(egui::Key::Y) {
                    self.home_map.draw_guide_message =
                        Some("되돌린 편집을 다시 적용했습니다 (Ctrl+Y).".to_owned());
                }
            }
        });
    }

    /// C++ `MainWindow.cpp` + `MainWindowChrome.cpp` + `KaStatusBar.cpp` 1:1 재현
    pub fn render_workspace_egui(&mut self, ctx: &egui::Context) -> egui::Rect {
        if (ctx.pixels_per_point() - 1.0).abs() > 0.01 {
            ctx.set_pixels_per_point(1.0);
        }
        let native_ppp = ctx.native_pixels_per_point().unwrap_or(1.0);
        if native_ppp > 1.05 {
            let full = ctx.screen_rect();
            let vis_w = (full.width() / native_ppp - 8.0).max(320.0);
            let vis_h = (full.height() / native_ppp - 16.0).max(240.0);
            let pad_b = (full.height() - vis_h).max(0.0);
            let pad_r = (full.width() - vis_w).max(0.0);
            if pad_b > 0.5 {
                egui::TopBottomPanel::bottom("ka_dpi_capture_pad_bottom")
                    .exact_height(pad_b)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |_ui| {});
            }
            if pad_r > 0.5 {
                egui::SidePanel::right("ka_dpi_capture_pad_right")
                    .exact_width(pad_r)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |_ui| {});
            }
        }
        self.poll_debug_remote_commands(ctx);
        self.handle_cpp_keyboard_shortcuts(ctx);

        if !self.right_inspector.map_focused {
            egui::TopBottomPanel::top("ka_cpp_top_chrome").show(ctx, |ui| {
                self.render_cpp_top_bars(ctx, ui);
            });
        }

        egui::TopBottomPanel::bottom("ka_status_bar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.active_tab == MainTab::Home {
                        ui.label(
                            egui::RichText::new("먼저 「새 조사」로 오늘 현장을 만드세요.")
                                .size(11.5)
                                .color(Color32::from_rgb(75, 74, 70)),
                        );
                    } else {
                        let msg = if self.active_tab == MainTab::Drawing {
                            "지도 칸을 만들었습니다. 나침반·좌표계·축척자·축척이 들어갔습니다."
                        } else {
                            self.home_map
                                .draw_guide_message
                                .as_deref()
                                .unwrap_or("위성과 지적도를 올려 두었습니다.")
                        };
                        ui.label(
                            egui::RichText::new(msg)
                                .size(11.5)
                                .color(Color32::from_rgb(75, 74, 70)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.active_tab == MainTab::Map {
                                ui.label(
                                    egui::RichText::new("지도갱신 켜짐")
                                        .size(11.0)
                                        .color(Color32::from_rgb(95, 94, 88)),
                                );
                                ui.separator();
                            }
                            if ui.small_button("→ 제출 5179").clicked() {
                                self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                            }
                            if ui
                                .small_button(match self.work_crs {
                                    WorkCrs::Epsg5187 => "작업 5187 · 동부원점 ▼",
                                    WorkCrs::Epsg5186 => "작업 5186 · 중부원점 ▼",
                                })
                                .clicked()
                            {
                                self.work_crs = match self.work_crs {
                                    WorkCrs::Epsg5187 => WorkCrs::Epsg5186,
                                    WorkCrs::Epsg5186 => WorkCrs::Epsg5187,
                                };
                            }
                            let live_scale = if self.active_tab == MainTab::Drawing {
                                self.drawing.sheet.scale_denominator.max(100)
                            } else {
                                ((self.home_map.viewport.meters_per_pixel * 3780.0).round() as u32)
                                    .max(100)
                            };
                            let scale_btn = ui.small_button(format!("1:{live_scale} ▼"));
                            if scale_btn.clicked() {
                                self.scale_menu_open = !self.scale_menu_open;
                            }
                            if self.scale_menu_open {
                                let popup_pos = egui::Pos2::new(
                                    scale_btn.rect.left(),
                                    (scale_btn.rect.top() - 165.0).max(40.0),
                                );
                                egui::Area::new(ui.id().with("ka_status_scale_popup"))
                                    .fixed_pos(popup_pos)
                                    .order(egui::Order::Foreground)
                                    .show(ctx, |ui| {
                                        egui::Frame::popup(ui.style())
                                            .fill(Color32::from_rgb(252, 251, 247))
                                            .stroke(egui::Stroke::new(
                                                1.0,
                                                Color32::from_rgb(185, 182, 172),
                                            ))
                                            .inner_margin(egui::Margin::symmetric(8, 6))
                                            .show(ui, |ui| {
                                                for (preset_lbl, denom) in [
                                                    ("1:500", 500u32),
                                                    ("1:1,000", 1_000),
                                                    ("1:2,500", 2_500),
                                                    ("1:5,000", 5_000),
                                                    ("1:10,000", 10_000),
                                                    ("1:25,000", 25_000),
                                                ] {
                                                    if ui
                                                        .selectable_label(
                                                            live_scale == denom,
                                                            preset_lbl,
                                                        )
                                                        .clicked()
                                                    {
                                                        if self.active_tab == MainTab::Drawing {
                                                            self.drawing.sheet.scale_denominator =
                                                                denom;
                                                        } else {
                                                            self.home_map
                                                                .viewport
                                                                .meters_per_pixel =
                                                                f64::from(denom) / 3780.0;
                                                        }
                                                        self.scale_menu_open = false;
                                                    }
                                                }
                                            });
                                    });
                            }
                            ui.label(egui::RichText::new("축척").size(11.0));
                            if self.active_tab == MainTab::Map {
                                ui.separator();
                                let [cx, cy] = self.home_map.cursor_world_xy;
                                ui.label(
                                    egui::RichText::new(format!("X {cx:11.3}    Y {cy:11.3}"))
                                        .size(11.0),
                                );
                            }
                            if self.is_dirty {
                                ui.colored_label(
                                    Color32::from_rgb(181, 87, 58),
                                    egui::RichText::new("● 저장 안 됨").size(11.0),
                                );
                            } else {
                                ui.colored_label(
                                    Color32::from_rgb(21, 128, 61),
                                    egui::RichText::new("● 저장됨").size(11.0),
                                );
                            }
                            if self.active_tab == MainTab::Map {
                                let snap_lbl = if self.home_map.cad.snap_enabled {
                                    "자석 켬"
                                } else {
                                    "자석 끔"
                                };
                                if ui.small_button(snap_lbl).clicked() {
                                    self.home_map.cad.snap_enabled =
                                        !self.home_map.cad.snap_enabled;
                                }
                                ui.label(
                                    egui::RichText::new(self.active_tool.label_ko())
                                        .size(11.0)
                                        .strong(),
                                );
                            }
                        });
                    }
                });
            });

        let mut central_rect = egui::Rect::NOTHING;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(243, 241, 232)))
            .show(ctx, |ui| {
                central_rect = ui.available_rect_before_wrap();
                match self.active_tab {
                    MainTab::Home => self.render_home_tab_ui(ui),
                    MainTab::Map => self.render_map_tab_ui(ctx, ui),
                    MainTab::Paleo => self.render_paleo_tab_ui(ctx, ui),
                    MainTab::Drawing => self.render_drawing_tab_ui(ui),
                    MainTab::Terrain3d => self.render_terrain3d_tab_ui(ctx, ui),
                }
            });

        render_active_modal_dialogs(self, ctx);
        central_rect
    }
}

impl eframe::App for StrataAppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let _ = self.render_workspace_egui(ctx);
    }
}
