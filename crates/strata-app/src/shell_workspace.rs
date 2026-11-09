//! 워크스페이스 렌더 진입점(`render_workspace_egui`).
//!
//! `gui_shell.rs` 가 300줄을 넘어 옮겼다. 본문은 그대로다.
use eframe::egui::{self, Color32, Vec2};
use strata_types::crs::WorkCrs;
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};
use crate::gui_shell::{UX_STATUS_BAR_HEIGHT, KA_STATUS_SCALE_PRESETS};

impl StrataAppState {
    /// C++ `MainWindow.cpp` + `MainWindowChrome.cpp` + `KaStatusBar.cpp` 1:1 재현
    pub fn render_workspace_egui(&mut self, ctx: &egui::Context) -> egui::Rect {
        self.poll_debug_remote_commands(ctx);
        self.handle_cpp_keyboard_shortcuts(ctx);

        let win_title = if self.active_tab == MainTab::Home {
            "Strata".to_owned()
        } else if self.is_dirty {
            format!("{} * - Strata", self.project_title)
        } else {
            format!("{} - Strata", self.project_title)
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(win_title));

        if !self.right_inspector.map_focused {
            egui::TopBottomPanel::top("ka_cpp_top_chrome")
                .frame(egui::Frame::NONE)
                .show_separator_line(false)
                .show(ctx, |ui| {
                    self.render_cpp_top_bars(ctx, ui);
                });
        }

        egui::TopBottomPanel::bottom("ka_status_bar")
            .exact_height(UX_STATUS_BAR_HEIGHT)
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
                            // MainWindowExport.cpp:252 — 도면 탭 안내는 C++ 원문 그대로.
                            StrataAppState::DRAWING_STUDIO_WELCOME
                        } else if !self.status_bar_hint.is_empty() {
                            self.status_bar_hint.as_str()
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
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if self.active_tab == MainTab::Map {
                                // KaStatusBar.cpp:121-134 renderToggle — 지도 다시 그리기를 잠시 멈춘다
                                let render_lbl = if self.map_rendering_enabled {
                                    "지도갱신 켜짐"
                                } else {
                                    "지도갱신 멈춤"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(render_lbl)
                                                .size(11.5)
                                                .color(Color32::from_rgb(75, 74, 70)),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text(
                                        "지도 다시 그리기를 잠시 멈춥니다. 레이어를 여러 개 켜고 끌 때 편합니다.",
                                    )
                                    .clicked()
                                {
                                    self.map_rendering_enabled = !self.map_rendering_enabled;
                                }
                                ui.separator();
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("→ 제출 5179")
                                            .size(11.5)
                                            .color(Color32::from_rgb(94, 93, 89)),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("제출 꾸러미는 EPSG:5179(UTM-K)로 자동 변환됩니다.")
                                .clicked()
                            {
                                self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                            }
                            let crs_lbl = match self.work_crs {
                                WorkCrs::Epsg5187 => "작업 5187 · 동부원점 ▼",
                                WorkCrs::Epsg5186 => "작업 5186 · 중부원점 ▼",
                            };
                            let crs_btn = egui::Button::new(
                                egui::RichText::new(crs_lbl)
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::from_rgb(156, 74, 47)),
                            )
                            .fill(Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                            .corner_radius(4.0)
                            .min_size(Vec2::new(124.0, 22.0));
                            if ui.add(crs_btn).clicked() {
                                self.work_crs = match self.work_crs {
                                    WorkCrs::Epsg5187 => WorkCrs::Epsg5186,
                                    WorkCrs::Epsg5186 => WorkCrs::Epsg5187,
                                };
                            }
                            ui.separator();
                            let live_scale = if self.active_tab == MainTab::Drawing {
                                self.drawing.sheet.scale_denominator.max(100)
                            } else {
                                ((self.home_map.viewport.meters_per_pixel * 3780.0).round() as u32)
                                    .max(100)
                            };
                            let scale_btn_widget = egui::Button::new(
                                egui::RichText::new(format!("1:{live_scale}   ▼"))
                                    .size(11.5)
                                    .color(Color32::from_rgb(30, 30, 28)),
                            )
                            .fill(Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                            .corner_radius(4.0)
                            .min_size(Vec2::new(82.0, 22.0));
                            let scale_btn = ui.add(scale_btn_widget);
                            if scale_btn.clicked() {
                                self.scale_menu_open = !self.scale_menu_open;
                                if self.scale_menu_open {
                                    self.scale_input_buf = format!("1:{live_scale}");
                                }
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
                                                let edit = ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.scale_input_buf,
                                                    )
                                                    .hint_text("예: 1000")
                                                    .desired_width(96.0),
                                                );
                                                if edit.lost_focus()
                                                    && ui.input(|i| {
                                                        i.key_pressed(egui::Key::Enter)
                                                    })
                                                    && self.apply_scale_input()
                                                {
                                                    self.scale_menu_open = false;
                                                }
                                                if self.scale_menu_open {
                                                    ui.separator();
                                                    for denom in KA_STATUS_SCALE_PRESETS {
                                                        let preset_lbl =
                                                            format!("1:{denom}");
                                                        if ui
                                                            .selectable_label(
                                                                live_scale == denom,
                                                                preset_lbl,
                                                            )
                                                            .clicked()
                                                        {
                                                            if self.active_tab
                                                                == MainTab::Drawing
                                                            {
                                                                self.drawing
                                                                    .sheet
                                                                .scale_denominator = denom;
                                                            } else {
                                                                self.home_map
                                                                    .viewport
                                                                    .meters_per_pixel =
                                                                    f64::from(denom) / 3780.0;
                                                            }
                                                            self.scale_menu_open = false;
                                                        }
                                                    }
                                                }
                                            });
                                    });
                            }
                            ui.label(
                                egui::RichText::new("축척")
                                    .size(11.5)
                                    .color(Color32::from_rgb(94, 93, 89)),
                            );
                            if self.active_tab == MainTab::Map {
                                ui.separator();
                                let [cx, cy] = self.home_map.cursor_world_xy;
                                ui.label(
                                    egui::RichText::new(format!("X {cx:10.3}   Y {cy:10.3}"))
                                        .size(11.5)
                                        .color(Color32::from_rgb(75, 74, 70)),
                                );
                            }
                            if self.is_dirty {
                                ui.separator();
                                ui.colored_label(
                                    Color32::from_rgb(181, 87, 58),
                                    egui::RichText::new("● 저장 안 됨").size(11.5),
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

        self.render_ribbon_context_menus(ctx);
        self.render_map_context_menu(ctx);
        self.render_layer_context_menu(ctx);
        self.handle_dropped_files(ctx);
        self.render_close_prompt(ctx);
        render_active_modal_dialogs(self, ctx);
        self.render_borehole_dialog_frame(ctx);
        central_rect
    }
}
