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
        let _perf_frame = crate::perf_recorder::begin_frame();
        self.poll_debug_remote_commands(ctx);
        self.handle_cpp_keyboard_shortcuts(ctx);
        self.drop_sketch_when_tool_changed();
        self.resync_card_on_tool_change();
        self.home_facts_on_visit(Some(ctx));

        // C++ refreshWindowTitle(`MainWindowSession.cpp:1327-1335`) — 탭과 관계없이 조사 파일 이름.
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));

        if !self.right_inspector.map_focused {
            egui::TopBottomPanel::top("ka_cpp_top_chrome")
                .frame(egui::Frame::NONE)
                .show_separator_line(false)
                .show(ctx, |ui| {
                    let _perf_rib =
                        crate::perf_recorder::begin_part(crate::perf_recorder::PerfPart::Ribbon);
                    self.render_cpp_top_bars(ctx, ui);
                });
        }

        let now = ctx.input(|i| i.time);
        self.download_jobs.tick(now);

        egui::TopBottomPanel::bottom("ka_status_bar")
            .exact_height(UX_STATUS_BAR_HEIGHT)
            .show(ctx, |ui| {
                let _perf_sb =
                    crate::perf_recorder::begin_part(crate::perf_recorder::PerfPart::StatusBar);
                if self.active_tab == MainTab::Home {
                    ui.horizontal_centered(|ui| {
                        ui.label(
                            egui::RichText::new("먼저 「새 조사」로 오늘 현장을 만드세요.")
                                .size(11.5)
                                .color(Color32::from_rgb(75, 74, 70)),
                        );
                        let budget = crate::download_gauge::gauge_title_budget(ui.available_width());
                        self.render_download_gauge(ctx, ui, now, budget);
                    });
                } else if self.download_jobs.is_empty() {
                    ui.horizontal_centered(|ui| {
                        // 다른 알림이 없으면 C++ updateNextActionStatus 의 「다음에 할 일」.
                        let msg = self.status_left_message().to_owned();
                        ui.label(
                            egui::RichText::new(msg)
                                .size(11.5)
                                .color(Color32::from_rgb(75, 74, 70)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.render_status_right_cluster(ctx, ui);
                        });
                    });
                } else {
                    // Paint right-hand cluster in its exact spot, then fit message + gauge to its left (`docs/RUST_PLAN.md` §2.8).
                    let bar_rect = ui.available_rect_before_wrap();
                    let mut right_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(bar_rect)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    self.render_status_right_cluster(ctx, &mut right_ui);
                    let left_max_x = (right_ui.min_rect().left() - 6.0).max(bar_rect.left());
                    let left_rect = egui::Rect::from_min_max(bar_rect.min, egui::Pos2::new(left_max_x, bar_rect.max.y));
                    let mut left_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(left_rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    let budget = crate::download_gauge::gauge_title_budget(left_rect.width());
                    let gauge_w = self.measure_download_gauge_width(&left_ui, now, budget);
                    let msg_cap_w = (left_rect.width() - gauge_w - 6.0).max(36.0);
                    let msg = self.status_left_message().to_owned();
                    let msg_full_w = left_ui.fonts(|f| {
                        f.layout_no_wrap(msg.clone(), egui::FontId::proportional(11.5), Color32::BLACK).size().x
                    });
                    let cap_rect = egui::Rect::from_min_size(left_ui.cursor().min, Vec2::new(msg_cap_w, left_rect.height()));
                    let msg_resp = left_ui
                        .scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(cap_rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            |msg_ui| {
                                msg_ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&msg).size(11.5).color(Color32::from_rgb(75, 74, 70)),
                                    )
                                    .truncate(),
                                )
                            },
                        )
                        .inner;
                    if msg_full_w > msg_cap_w {
                        let _ = msg_resp.on_hover_text(&msg);
                    }
                    self.render_download_gauge(ctx, &mut left_ui, now, budget);
                    ui.advance_cursor_after_rect(bar_rect);
                }
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
        self.render_survey_area_setup(ctx);
        self.render_feature_form(ctx);
        self.render_feature_record(ctx);
        self.render_rename_layer(ctx);
        central_rect
    }

    fn render_status_right_cluster(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 6.0;
        if self.active_tab == MainTab::Map {
            // KaStatusBar.cpp:121-134 renderToggle — 지도 다시 그리기를 잠시 멈춘다
            let render_lbl = if self.map_rendering_enabled { "지도갱신 켜짐" } else { "지도갱신 멈춤" };
            if ui
                .add(egui::Button::new(egui::RichText::new(render_lbl).size(11.5).color(Color32::from_rgb(75, 74, 70))).frame(false))
                .on_hover_text("지도 다시 그리기를 잠시 멈춥니다. 레이어를 여러 개 켜고 끌 때 편합니다.")
                .clicked()
            {
                self.map_rendering_enabled = !self.map_rendering_enabled;
            }
            ui.separator();
        }
        if ui
            .add(egui::Button::new(egui::RichText::new("→ 제출 5179").size(11.5).color(Color32::from_rgb(94, 93, 89))).frame(false))
            .on_hover_text("제출 꾸러미는 EPSG:5179(UTM-K)로 자동 변환됩니다.")
            .clicked()
        {
            self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
        }
        let crs_lbl = match self.work_crs {
            WorkCrs::Epsg5187 => "작업 5187 · 동부원점 ▼",
            WorkCrs::Epsg5186 => "작업 5186 · 중부원점 ▼",
        };
        let crs_btn = egui::Button::new(egui::RichText::new(crs_lbl).size(11.5).strong().color(Color32::from_rgb(156, 74, 47)))
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
            // QGIS 축척(96 dpi) 그대로 — C++ 은 1:100 아래로 자르지 않는다.
            u32::try_from(self.map_scale_denominator()).unwrap_or(u32::MAX)
        };
        let scale_btn_widget = egui::Button::new(
            egui::RichText::new(format!("1:{live_scale}   ▼")).size(11.5).color(Color32::from_rgb(30, 30, 28)),
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
            let popup_pos = egui::Pos2::new(scale_btn.rect.left(), (scale_btn.rect.top() - 165.0).max(40.0));
            egui::Area::new(ui.id().with("ka_status_scale_popup"))
                .fixed_pos(popup_pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(Color32::from_rgb(252, 251, 247))
                        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            let edit = ui.add(egui::TextEdit::singleline(&mut self.scale_input_buf).hint_text("예: 1000").desired_width(96.0));
                            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && self.apply_scale_input() {
                                self.scale_menu_open = false;
                            }
                            if self.scale_menu_open {
                                ui.separator();
                                for denom in KA_STATUS_SCALE_PRESETS {
                                    if ui.selectable_label(live_scale == denom, format!("1:{denom}")).clicked() {
                                        if self.active_tab == MainTab::Drawing {
                                            self.drawing.sheet.scale_denominator = denom;
                                        } else {
                                            self.home_map.viewport.meters_per_pixel =
                                                f64::from(denom) / crate::status_bar_text::POINTS_PER_METER;
                                        }
                                        self.scale_menu_open = false;
                                    }
                                }
                            }
                        });
                });
        }
        ui.label(egui::RichText::new("축척").size(11.5).color(Color32::from_rgb(94, 93, 89)));
        if self.active_tab == MainTab::Map {
            ui.separator();
            // KaStatusBar.cpp:58-64 xyReadout — 폭 240, 오른쪽 맞춤.
            let xy = self.xy_readout();
            ui.allocate_ui_with_layout(Vec2::new(240.0, 22.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(xy).size(11.5).color(Color32::from_rgb(75, 74, 70)))
                    .on_hover_text("지도 위 커서의 작업 좌표입니다.");
            });
            // KaDrawSketchTools.cpp:239-262 toolChip — 지금 도구, 굵게, 좌우 여백 8.
            let tool = self.tool_chip_label();
            if !tool.is_empty() {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(tool).size(11.5).strong().color(Color32::from_rgb(35, 35, 32)))
                    .on_hover_text("지금 켜져 있는 지도 도구입니다.");
                ui.add_space(8.0);
            }
        }
        // KaStatusBar::setUnsavedCount — 「저장 안 됨 n건」(편집한 도형 수) 또는 「저장 안 됨」.
        if let Some(unsaved) = self.unsaved_chip_text() {
            ui.separator();
            ui.colored_label(Color32::from_rgb(181, 87, 58), egui::RichText::new(unsaved).size(11.5))
                .on_hover_text("아직 파일에 쓰지 않은 편집입니다. 「저장」(Ctrl+S)으로 씁니다.");
        }
    }
}
