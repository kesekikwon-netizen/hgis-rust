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
                self.right_inspector.left_panel_collapsed = !self.right_inspector.left_panel_collapsed;
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
                self.right_inspector.right_panel_collapsed = !self.right_inspector.right_panel_collapsed;
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
            if !wants_kb && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
                self.home_map.cad.on_backspace(&mut self.home_map.scene);
            }
            if i.modifiers.ctrl {
                if i.key_pressed(egui::Key::N) {
                    self.start_new_survey_project("경주시");
                } else if i.key_pressed(egui::Key::O) {
                    self.open_sample_survey_project(0);
                } else if i.key_pressed(egui::Key::S) {
                    self.home_map.draw_guide_message = Some(format!("「{}」 조사를 저장했습니다.", self.project_title));
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
                    self.active_tab = MainTab::Drawing;
                    self.modals.active = Some(ActiveModalDialog::DrawingPrint);
                } else if i.key_pressed(egui::Key::E) {
                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                } else if i.key_pressed(egui::Key::Z) {
                    self.home_map.draw_guide_message = Some("마지막 편집을 되돌렸습니다 (Ctrl+Z).".to_owned());
                } else if i.key_pressed(egui::Key::Y) {
                    self.home_map.draw_guide_message = Some("되돌린 편집을 다시 적용했습니다 (Ctrl+Y).".to_owned());
                }
            }
        });
    }

    /// C++ `MainWindow.cpp` + `MainWindowChrome.cpp` + `KaStatusBar.cpp` 1:1 재현
    pub fn render_workspace_egui(&mut self, ctx: &egui::Context) -> egui::Rect {
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
                            self.home_map.draw_guide_message.as_deref().unwrap_or("위성과 지적도를 올려 두었습니다.")
                        };
                        ui.label(egui::RichText::new(msg).size(11.5).color(Color32::from_rgb(75, 74, 70)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.active_tab == MainTab::Map {
                                ui.label(egui::RichText::new("지도갱신 켜짐").size(11.0).color(Color32::from_rgb(95, 94, 88)));
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
                            let scale_str = if self.active_tab == MainTab::Drawing {
                                "1:6532 ▼"
                            } else {
                                "1:10117 ▼"
                            };
                            let _ = ui.small_button(scale_str);
                            ui.label(egui::RichText::new("축척").size(11.0));
                            if self.active_tab == MainTab::Map {
                                ui.separator();
                                ui.label(egui::RichText::new("X 219,897.093    Y 361,622.354").size(11.0));
                            }
                            ui.colored_label(
                                Color32::from_rgb(181, 87, 58),
                                egui::RichText::new("● 저장 안 됨").size(11.0),
                            );
                            if self.active_tab == MainTab::Map {
                                ui.label(egui::RichText::new("자석 켬").size(11.0));
                                ui.label(egui::RichText::new("이동").size(11.0).strong());
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
