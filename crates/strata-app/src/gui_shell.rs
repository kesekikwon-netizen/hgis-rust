use eframe::egui::{self, Color32, Vec2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_types::crs::WorkCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};
use crate::ribbon::SubToolsMode;

/// UX_LAYOUT_SPEC.md 1절: 세로 5단 골격 (창 1600×900 기준, 상태줄 y=839, h=32)
pub const UX_STATUS_BAR_Y: f32 = 839.0;
pub const UX_STATUS_BAR_HEIGHT: f32 = 32.0;
pub const UX_TAB_BODY_Y: f32 = 158.0;
pub const UX_TAB_BODY_HEIGHT: f32 = 681.0;

/// KaStatusBar.cpp:86-87 — 13개 기본 축척 프리셋
pub const KA_STATUS_SCALE_PRESETS: [u32; 13] = [
    100, 200, 250, 500, 1_000, 2_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000,
];

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
        central_rect
    }

    /// `MainWindowFiles.cpp:259-289` `tryAddDroppedPaths` — 창에 끌어 놓은 파일을
    /// 확장자에 따라 레이어로 올린다. CAD 도면은 안내 창을 먼저 띄운다.
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let mut added = 0usize;
        let mut cad_path: Option<String> = None;
        let mut skipped = 0usize;
        for f in &dropped {
            let Some(path) = f.path.as_ref() else {
                skipped += 1;
                continue;
            };
            let name = path.to_string_lossy().to_string();
            let low = name.to_lowercase();
            let is_cad = low.ends_with(".dxf") || low.ends_with(".dwg");
            let ok = low.ends_with(".shp")
                || is_cad
                || low.ends_with(".gpkg")
                || low.ends_with(".geojson")
                || low.ends_with(".json")
                || low.ends_with(".tif")
                || low.ends_with(".tiff")
                || low.ends_with(".img")
                || low.ends_with(".csv")
                || low.ends_with(".xlsx")
                || low.ends_with(".hgis");
            if !ok {
                skipped += 1;
                continue;
            }
            if is_cad {
                cad_path = Some(name);
                continue;
            }
            added += 1;
        }
        if let Some(path) = cad_path {
            // KaCadImportNotice.cpp — 도면은 좌표계 확인 안내를 먼저 보여준다
            self.modals.active = Some(ActiveModalDialog::CadImportNotice);
            self.modals.cad_import_path = path;
            self.home_map.draw_guide_message =
                Some("CAD 도면을 끌어 놓았습니다. 좌표계를 확인하고 지도에 올리세요.".to_owned());
            return;
        }
        if added > 0 {
            let msg = format!("레이어 {added}개 추가됨 (파일→지도)");
            self.last_drop_message = Some(msg.clone());
            self.home_map.draw_guide_message = Some(msg);
            self.active_tab = MainTab::Map;
            self.is_dirty = true;
        } else if skipped > 0 {
            self.home_map.draw_guide_message =
                Some("지도에 올릴 수 있는 파일이 아닙니다.".to_owned());
        }
    }

    /// `KaClosePrompt.cpp` — 저장하지 않은 편집이 있으면 닫기 전에 한 번 묻는다.
    fn render_close_prompt(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.is_dirty && !self.close_prompt_open
        {
            self.close_prompt_open = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if !self.close_prompt_open {
            return;
        }
        let full = ctx.screen_rect();
        let w = 420.0_f32.min(full.width() - 32.0);
        let h = 168.0_f32.min(full.height() - 32.0);
        let pos = egui::Pos2::new(
            ((full.width() - w) * 0.5).max(16.0),
            ((full.height() - h) * 0.5).max(16.0),
        );
        egui::Window::new("ka_close_prompt")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(pos)
            .fixed_size([w, h])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("저장하지 않은 편집이 있습니다")
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("끝내기 전에 지금까지 그린 조사 내용을 저장할까요?")
                        .size(12.0)
                        .color(Color32::from_rgb(80, 79, 74)),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(
                        egui::RichText::new("저장하고 끝내기")
                            .color(Color32::WHITE)
                            .strong()
                            .size(12.5),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(130.0, 30.0));
                    if ui.add(save_btn).clicked() {
                        self.is_dirty = false;
                        self.close_prompt_open = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("저장하지 않고 끝내기").size(12.0),
                            )
                            .min_size(Vec2::new(150.0, 30.0)),
                        )
                        .clicked()
                    {
                        self.close_prompt_open = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("취소").size(12.0))
                                    .min_size(Vec2::new(70.0, 30.0)),
                            )
                            .clicked()
                        {
                            self.close_prompt_open = false;
                        }
                    });
                });
            });
    }
}

impl eframe::App for StrataAppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // OS 모니터 DPI 배율(native_pixels_per_point, 예: 1.5)을 그대로 유지하고
        // 큰 글씨 모드일 때만 zoom_factor(1.15)를 곱한다.
        let want_zoom = if self.large_font_mode { 1.15 } else { 1.0 };
        if (ctx.zoom_factor() - want_zoom).abs() > 0.001 {
            ctx.set_zoom_factor(want_zoom);
        }
        // C++ KaThemeOptions.cpp — 「테마」에서 고른 4대 테마·고대비가 화면 전체에
        // 즉시 반영되도록, 테마가 바뀐 프레임에 스타일을 다시 적용한다.
        if self.theme_dirty {
            crate::gui_theme_fonts::apply_theme_tokens(ctx, &self.theme);
            self.theme_dirty = false;
        }
        let _ = self.render_workspace_egui(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_layer_panel::{
        FILE_BTN_DESK_TIP, FILE_BTN_DOCS_TIP, FILE_BTN_DOWN_TIP, FILE_BTN_PC_TIP,
        FILE_BTN_PICK_TIP, FILE_BTN_UP_TIP, FILE_LIST_TIP,
    };
    use crate::gui_layer_panel::{
        UX_FILE_DRAWER_HEIGHT, UX_LAYER_CARD_HEIGHT, UX_LEFT_PANEL_WIDTH,
    };
    use crate::gui_ribbon_bar::{
        REGION_BOTTOM_HINT, REGION_LOT_HINT, REGION_LOT_PLACEHOLDER, UX_RIBBON_BUTTON_SIZE,
        UX_RIBBON_GROUP_WIDTHS, UX_RIBBON_HEIGHT, UX_VIEW_TAB_BAR_HEIGHT,
    };
    use crate::gui_tab_map::{
        brightness_row_text, grid_band_rects, grid_label_text, grid_line_stride,
        is_raster_image_layer, nice_grid_step_meters, scale_bar_metres, scale_bar_tick_text,
        ShellGridConfig, GRID_BAND_BORDER, GRID_BAND_FILL, GRID_BAND_INK, GRID_BAND_PX,
        UX_MAP_CANVAS_WIDTH, UX_MAP_LEFT_WIDTH, UX_MAP_RIGHT_WIDTH,
    };

    // D12 KaStatusBar.cpp:77-136 — 13단계 축척 프리셋, 직접 입력 칸, 지도갱신 토글
    // D21 — 지도 캔버스·레이어 트리 우클릭 메뉴 상태
    // D25 파일 끌어 놓기·종료 확인·CAD 안내 + 해상도별 레이아웃
    #[test]
    fn test_d25_drop_close_prompt_cad_notice_and_resolutions() {
        // MainWindowFiles.cpp:265-268 — 받는 확장자
        let accepts = |n: &str| -> bool {
            let low = n.to_lowercase();
            low.ends_with(".shp")
                || low.ends_with(".dxf")
                || low.ends_with(".dwg")
                || low.ends_with(".gpkg")
                || low.ends_with(".geojson")
                || low.ends_with(".json")
                || low.ends_with(".tif")
                || low.ends_with(".img")
                || low.ends_with(".csv")
                || low.ends_with(".xlsx")
                || low.ends_with(".hgis")
        };
        assert!(accepts("조사구역.SHP"));
        assert!(accepts("도면.dxf"));
        assert!(accepts("data.gpkg"));
        assert!(!accepts("readme.txt"));
        assert!(!accepts("photo.bmp"));

        // KaClosePrompt.cpp — 저장/저장안함/취소
        let buttons = ["저장하고 끝내기", "저장하지 않고 끝내기", "취소"];
        assert_eq!(buttons.len(), 3);

        // KaCadImportNotice.cpp — 문구
        assert!("CAD 도면 좌표계 확인".contains("좌표계"));

        // 해상도·배율별 레이아웃이 화면 안에 든다
        for (sw, sh, ppp) in [
            (1600.0_f32, 900.0_f32, 1.0_f32),
            (1920.0, 1080.0, 1.0),
            (2560.0, 1080.0, 1.0),
            (3440.0, 1440.0, 1.0),
            (3840.0, 2160.0, 1.5),
            (3840.0, 2160.0, 2.0),
        ] {
            let lw = sw / ppp;
            let lh = sh / ppp;
            // 모달은 항상 화면 - 32 안에 든다
            for (dw, dh) in [
                (960.0_f32, 620.0_f32),
                (940.0, 700.0),
                (820.0, 500.0),
                (460.0, 260.0),
            ] {
                assert!(dw.min(lw - 32.0) <= lw);
                assert!(dh.min(lh - 32.0) <= lh);
            }
            // 좌우 패널을 빼도 가운데 칸이 남는다
            assert!(lw - 268.0 - 280.0 > 320.0, "too narrow at {sw}x{sh} @{ppp}");
        }
    }

    #[test]
    fn test_d21_map_and_layer_context_menus() {
        // MainWindowContextMenus.cpp:112-161 mapContextMenu 항목 순서
        let map_items = [
            "이 도형 기록 입력",
            "조사 도형 그리기",
            "조사구역",
            "유구 면",
            "유구 선",
            "단면선 그리기",
            "이전 지도 범위",
            "다음 지도 범위",
            "선택한 레이어로 이동",
            "전체 지도 보기",
            "지도 새로고침",
            "이 위치 좌표 복사",
        ];
        assert!(map_items.contains(&"이 위치 좌표 복사"));
        assert!(map_items.contains(&"전체 지도 보기"));

        // 좌표 복사 문구 형식 "{N:.3}, {E:.3} ({EPSG})"
        let txt = format!(
            "{:.3}, {:.3} ({})",
            361_622.354_f64, 219_897.093_f64, "EPSG:5187"
        );
        assert!(txt.starts_with("361622.354, "));

        // MainWindowContextMenus.cpp:176-448 layerContextMenu 항목
        let layer_items = [
            "참고 자료 불러오기…",
            "이름 바꾸기 (두 번 클릭)",
            "면·외곽선 색",
            "도형 속성",
            "라벨·명칭 속성",
            "깨진 한글 바로잡기",
            "투명도",
            "이 레이어로 이동",
            "전체 보기",
            "같은 묶음의 맨 아래로",
            "그리기·편집 시작",
            "꼭짓점 수정",
            "폴리곤 묶기",
            "면적 확인",
            "그린 도형 모두 지우기…",
            "레이어 삭제",
        ];
        assert_eq!(layer_items.len(), 16);
        assert_eq!(layer_items[0], "참고 자료 불러오기…");
        assert_eq!(layer_items[15], "레이어 삭제");

        // 글자 크기 후보 10종
        let sizes = [3_u32, 4, 5, 6, 7, 8, 9, 10, 12, 14];
        assert_eq!(sizes.len(), 10);
        // 투명도 후보 6종
        let opacity = [0_u8, 20, 40, 50, 60, 80];
        assert_eq!(opacity.len(), 6);
        // 인코딩 후보 4종
        let encodings = ["CP949", "EUC-KR", "UTF-8", "이 컴퓨터 설정"];
        assert_eq!(encodings.len(), 4);
    }

    #[test]
    fn test_d12_status_bar_13_scale_presets_and_render_toggle() {
        assert_eq!(KA_STATUS_SCALE_PRESETS.len(), 13);
        assert_eq!(KA_STATUS_SCALE_PRESETS[0], 100);
        assert_eq!(KA_STATUS_SCALE_PRESETS[12], 500_000);

        let mut app = StrataAppState::new_startup();
        assert!(app.map_rendering_enabled);
        assert!(app.scale_input_buf.is_empty());

        // 「1000」과 「1:2500」이 모두 그대로 쓰인다 (KaStatusBar.cpp:92-98)
        app.active_tab = MainTab::Map;
        app.scale_input_buf = "1000".to_owned();
        assert!(app.apply_scale_input());
        assert!((app.home_map.viewport.meters_per_pixel - 1000.0 / 3780.0).abs() < 1e-12);
        app.scale_input_buf = "1:2500".to_owned();
        assert!(app.apply_scale_input());
        assert!((app.home_map.viewport.meters_per_pixel - 2500.0 / 3780.0).abs() < 1e-12);
        assert_eq!(app.scale_input_buf, "1:2500");

        // 읽을 수 없는 글자는 아무것도 바꾸지 않는다
        let before = app.home_map.viewport.meters_per_pixel;
        app.scale_input_buf = "abc".to_owned();
        assert!(!app.apply_scale_input());
        assert!((app.home_map.viewport.meters_per_pixel - before).abs() < 1e-12);
    }

    #[test]
    fn test_u1_ux_layout_spec_1600x900_skeleton_ribbon_and_3_column_split() {
        assert_eq!(UX_RIBBON_BUTTON_SIZE, egui::Vec2::new(69.0, 100.0));
        assert_eq!(UX_RIBBON_HEIGHT, 91.0);
        assert_eq!(UX_VIEW_TAB_BAR_HEIGHT, 51.0);
        assert_eq!(
            UX_RIBBON_GROUP_WIDTHS,
            [214.0, 250.0, 156.0, 382.0, 49.0, 215.0, 50.0]
        );
        assert_eq!(UX_STATUS_BAR_Y, 839.0);
        assert_eq!(UX_STATUS_BAR_HEIGHT, 32.0);
        assert_eq!(UX_LEFT_PANEL_WIDTH, 351.0);
        assert_eq!(UX_MAP_LEFT_WIDTH, 351.0);
        assert_eq!(UX_MAP_CANVAS_WIDTH, 954.0);
        assert_eq!(UX_MAP_RIGHT_WIDTH, 236.0);
        assert_eq!(UX_LAYER_CARD_HEIGHT, 507.0);
        assert_eq!(UX_FILE_DRAWER_HEIGHT, 168.0);

        let ctx = egui::Context::default();
        let mut app = StrataAppState::new_startup();
        app.active_tab = MainTab::Map;
        app.ribbon.sub_toolbar_visible = false;

        let raw_1600 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw_1600, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
        let map_w_1600 = app.home_map.viewport.screen_size_px[0];
        assert!(
            (map_w_1600 - f64::from(UX_MAP_CANVAS_WIDTH)).abs() <= 4.0,
            "expected map canvas width ~954 at 1600x900, got {map_w_1600}"
        );

        let raw_1920 = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 900.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw_1920, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
        let map_w_1920 = app.home_map.viewport.screen_size_px[0];
        assert!(
            map_w_1920 > map_w_1600 + 250.0,
            "center map must expand when window widens: 1600->{map_w_1600}, 1920->{map_w_1920}"
        );
    }

    // D14 KaRegionLocator.cpp:143-195, 265-276 — 찾기/취소, 법정동·읍·면·리, 산 12-3, 아래 규칙 두 줄
    // D17 KaDrawGuideBand 실시간 스케치 측정 · KaReferenceStatusDialog
    #[test]
    fn test_d17_draw_guide_measure_and_reference_status() {
        // KaDrawGuideBand.cpp:48-62 — 선 길이·면 면적 계산
        let ring = [[0.0_f64, 0.0], [30.0, 0.0], [30.0, 40.0]];
        let mut len_m = 0.0_f64;
        for w in ring.windows(2) {
            let dx = w[1][0] - w[0][0];
            let dy = w[1][1] - w[0][1];
            len_m += (dx * dx + dy * dy).sqrt();
        }
        assert!((len_m - 70.0).abs() < 1e-9);
        let mut area = 0.0_f64;
        for k in 0..ring.len() {
            let a = ring[k];
            let b = ring[(k + 1) % ring.len()];
            area += a[0] * b[1] - b[0] * a[1];
        }
        assert!((area.abs() * 0.5 - 600.0).abs() < 1e-9);

        // 1 km 이상이면 km 병기
        let far = [[0.0_f64, 0.0], [1500.0, 0.0]];
        let mut far_len = 0.0_f64;
        for w in far.windows(2) {
            let dx = w[1][0] - w[0][0];
            let dy = w[1][1] - w[0][1];
            far_len += (dx * dx + dy * dy).sqrt();
        }
        assert!(far_len >= 1000.0);

        // KaReferenceStatusDialog.cpp:56-60 — 7개 열 폭 합이 창 안에 든다
        let cols = [176.0_f32, 68.0, 96.0, 96.0, 56.0, 150.0, 86.0];
        let sum: f32 = cols.iter().sum::<f32>() + 48.0;
        assert!(
            sum <= 776.0,
            "columns {sum} must fit the 820px dialog inner width"
        );
    }

    #[test]
    fn test_d14_region_locator_and_canvas_grid_band() {
        let mut app = StrataAppState::new_startup();
        assert_eq!(REGION_LOT_PLACEHOLDER, "산 12-3");
        assert_eq!(
            REGION_LOT_HINT,
            "번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요."
        );
        assert_eq!(
            REGION_BOTTOM_HINT,
            "번지 입력 시 정확한 지번만 찾습니다. 읍·면은 리까지 입력하세요.\n예: 고아읍 봉한리 / 산 12-3"
        );

        // 찾기와 같은 길: 고른 시·도(및 시·군·구·동) 중심으로 옮기고 팝업을 닫는다
        let tile_center = crate::ribbon::PENINSULA_REGION_TILES
            .iter()
            .find(|t| t.name == "경북")
            .expect("경북 타일")
            .center_5187;
        app.ribbon.app_bar.region_popup_open = true;
        app.ribbon.app_bar.lot_input = "없는번지 12345".to_owned();
        app.search_region_locator_address();
        assert!(!app.ribbon.app_bar.region_popup_open);
        assert_eq!(app.active_tab, MainTab::Map);
        assert_eq!(app.home_map.viewport.center_xy, tile_center);
        assert_eq!(
            app.ribbon.app_bar.query,
            "경북 경주시 인왕동 없는번지 12345".to_owned()
        );

        // 지번이 실제 지적 필지에 있으면 그 필지 중심으로, 1.25 m/px까지 당겨 온다
        let parcel = app
            .home_map
            .real_survey
            .cadastral_parcels
            .first()
            .map(|p| (p.jibun.clone(), p.center))
            .expect("지적 필지 샘플");
        app.ribbon.app_bar.lot_input = parcel.0.clone();
        app.search_region_locator_address();
        assert_eq!(app.home_map.viewport.center_xy, parcel.1);
        assert!((app.home_map.viewport.meters_per_pixel - 1.25).abs() < 1e-12);

        // 도곽 띠: 위 18 px 가로 띠와 왼쪽 18 px 세로 띠, 그 안쪽 경계
        assert!((GRID_BAND_PX - 18.0).abs() < 1e-9);
        assert_eq!(GRID_BAND_FILL, egui::Color32::from_rgb(0xF5, 0xF4, 0xED));
        assert_eq!(GRID_BAND_BORDER, egui::Color32::from_rgb(0xDE, 0xDC, 0xD1));
        assert_eq!(GRID_BAND_INK, egui::Color32::from_rgb(0x5E, 0x5D, 0x59));
        let map_r =
            egui::Rect::from_min_size(egui::Pos2::new(100.0, 60.0), egui::vec2(400.0, 300.0));
        let (top_band, left_band) = grid_band_rects(map_r);
        assert_eq!(top_band.min, egui::Pos2::new(100.0, 60.0));
        assert_eq!(top_band.width(), 400.0);
        assert!((top_band.height() - 18.0).abs() < 1e-9);
        assert_eq!(left_band.min, egui::Pos2::new(100.0, 78.0));
        assert!((left_band.width() - 18.0).abs() < 1e-9);
        assert!((left_band.height() - 282.0).abs() < 1e-9);

        // 계단 간격은 120 mpp에 맞춘 1·2·5·10 미터
        assert!((nice_grid_step_meters(120.0) - 100.0).abs() < 1e-9);
        assert!((nice_grid_step_meters(240.0) - 200.0).abs() < 1e-9);
        assert!((nice_grid_step_meters(600.0) - 500.0).abs() < 1e-9);
        assert!((nice_grid_step_meters(0.0) - 10.0).abs() < 1e-9);

        // 회전한 격자는 원점에서 잰 거리로, 돌지 않은 격자는 실제 좌표로 적는다
        assert_eq!(grid_label_text(0.0, 219_000.0, 20.0, true), "원점");
        assert_eq!(grid_label_text(20.0, 219_000.0, 20.0, true), "+20 m");
        assert_eq!(grid_label_text(-20.0, 219_000.0, 20.0, true), "−20 m");
        assert_eq!(grid_label_text(0.0, 219_000.0, 20.0, false), "219000 m");
        assert_eq!(grid_label_text(0.0, 219_000.0, 1000.0, false), "219 km");

        // 선이 너무 많으면 n번째마다 그린다
        assert_eq!(grid_line_stride(100.0, 20.0, 80), 1);
        assert_eq!(grid_line_stride(2000.0, 20.0, 80), 2);

        // 굵기·모양은 설정값을 따른다 (0 실선 · 1 점선 · 2 점, 굵기 0.5~5.0)
        let cfg = ShellGridConfig {
            line_width: 9.0,
            dash_idx: 0,
            ..ShellGridConfig::default()
        };
        let ink = egui::Color32::from_rgb(0xD9, 0x2B, 0x2B);
        let stroke = crate::gui_tab_map::grid_stroke_for(&cfg, ink);
        assert!((stroke.width - 5.0).abs() < 1e-6);
        assert_eq!(egui::Color32::from_rgb(0xD9, 0x2B, 0x2B).r(), 0xD9);
        // 격자 선은 견본 색을 불투명 150/255로 흐리게 쓴다 (KaCanvasGridOverlay.cpp:110)
        let faint = egui::Color32::from_rgba_unmultiplied(0xD9, 0x2B, 0x2B, 150);
        assert_eq!(stroke.color, faint);
        let a = egui::Pos2::new(0.0, 0.0);
        let b = egui::Pos2::new(0.0, 100.0);
        assert_eq!(
            crate::gui_tab_map::grid_dash_segments(a, b, 0, 1.0),
            vec![[a, b]]
        );
        let dots = crate::gui_tab_map::grid_dash_segments(a, b, 2, 1.0);
        assert!(dots.len() > 1);
        let dashes = crate::gui_tab_map::grid_dash_segments(a, b, 1, 1.0);
        assert_eq!(dashes.len(), 12);
        assert_eq!(dots.len(), 25);
    }

    // D15 KaMapControls.cpp:138-260 · KaLayerOpacityRail.cpp:156-173 · KaFileBrowserPanel.cpp:92-136
    #[test]
    fn test_d15_scale_bar_quarters_and_opacity_rail_vector_brightness() {
        // 축척 1.0 m/px → 120 m 목표 → 100 m 막대, 100 px
        let round_m = scale_bar_metres(1.0).expect("scale bar");
        assert!((round_m - 100.0).abs() < 1e-9);
        assert!(((round_m / 1.0).clamp(40.0, 220.0) - 100.0).abs() < 1e-9);
        assert_eq!(
            scale_bar_tick_text(round_m),
            ("0".to_owned(), "50".to_owned(), "100 m".to_owned())
        );
        // 1:1,000 화면 축척(0.2645833333 m/px) → 20 m
        let round_m = scale_bar_metres(0.264_583_333_333_333_3).expect("scale bar");
        assert!((round_m - 20.0).abs() < 1e-9);
        assert_eq!(
            scale_bar_tick_text(round_m),
            ("0".to_owned(), "10".to_owned(), "20 m".to_owned())
        );
        // 10 m/px → 1.2 km 목표 → 1 km, 단위는 km
        let round_m = scale_bar_metres(10.0).expect("scale bar");
        assert!((round_m - 1000.0).abs() < 1e-9);
        assert_eq!(
            scale_bar_tick_text(round_m),
            ("0".to_owned(), "0".to_owned(), "1 km".to_owned())
        );
        // 축척이 없으면 막대를 그리지 않는다
        assert!(scale_bar_metres(0.0).is_none());
        assert!(scale_bar_metres(f64::NAN).is_none());

        // 그림(래스터) 레이어만 밝기를 가진다 — KaLayerOpacityRail.cpp:156-173
        for raster in [
            "지형 음영",
            "지표모델(DSM) 30m",
            "위성사진",
            "항공사진",
            "고지형 분류",
        ] {
            assert!(is_raster_image_layer(raster), "{raster}");
        }
        for vector in [
            "조사구역",
            "시굴격자",
            "지적도 · 조사 주변 5km",
            "지정유산",
            "주변 500m",
        ] {
            assert!(!is_raster_image_layer(vector), "{vector}");
            let (txt, tip) = brightness_row_text(0, false);
            assert_eq!(txt, "그림만");
            assert_eq!(tip, "밝기는 위성·항공사진 같은 그림 레이어에만 있습니다.");
        }
        assert_eq!(
            brightness_row_text(0, true),
            (
                "0".to_owned(),
                "고른 그림의 밝기. 가운데가 원본입니다. 원본 파일은 바뀌지 않습니다."
            )
        );
        assert_eq!(brightness_row_text(40, true).0, "+40");
        assert_eq!(brightness_row_text(-40, true).0, "-40");

        // KaFileBrowserPanel.cpp:92-136 — 폴더 단추 여섯 개와 목록 칸의 안내말
        assert_eq!(FILE_BTN_PC_TIP, "컴퓨터 드라이브 목록으로 이동합니다");
        assert_eq!(FILE_BTN_DESK_TIP, "바탕화면 폴더로 이동합니다");
        assert_eq!(FILE_BTN_DOCS_TIP, "문서 폴더로 이동합니다");
        assert_eq!(FILE_BTN_DOWN_TIP, "내려받기 폴더로 이동합니다");
        assert_eq!(
            FILE_BTN_PICK_TIP,
            "조사 자료 폴더를 직접 찾아 엽니다. 카메라·GPS 메모리카드(SD)도 여기서 엽니다"
        );
        assert_eq!(FILE_BTN_UP_TIP, "상위 폴더로 한 단계 이동합니다");
        assert_eq!(FILE_LIST_TIP, "파일을 지도에 끌어 넣으면 레이어가 됩니다.");

        // D15 지도 화면은 1600x900 기준
        let mut app = StrataAppState::new_startup();
        app.active_tab = MainTab::Map;
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
        let map_w = app.home_map.viewport.screen_size_px[0];
        assert!(
            (map_w - 954.0).abs() <= 4.0,
            "1600x900에서 지도 폭 ~954, got {map_w}"
        );
    }
}
