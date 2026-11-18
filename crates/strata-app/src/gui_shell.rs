use eframe::egui::{self};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

#[path = "gui_shell_escape.rs"]
mod escape;
#[path = "native_menu_input.rs"]
mod native_menu_input;
pub(crate) use native_menu_input::menu_button as native_menu_button;
pub(crate) use native_menu_input::popup_below_widget as native_popup_below_widget;

/// UX_LAYOUT_SPEC.md 1절: 세로 5단 골격 (창 1600×900 기준, 상태줄 y=839, h=32)
pub const UX_STATUS_BAR_Y: f32 = 839.0;
pub const UX_STATUS_BAR_HEIGHT: f32 = 32.0;
pub const UX_TAB_BODY_Y: f32 = 158.0;
pub const UX_TAB_BODY_HEIGHT: f32 = 681.0;

/// KaStatusBar.cpp:86-87 — 13개 기본 축척 프리셋
pub const KA_STATUS_SCALE_PRESETS: [u32; 13] = [100, 200, 250, 500, 1_000, 2_000, 5_000, 10_000, 25_000, 50_000, 100_000, 250_000, 500_000];

impl StrataAppState {
    /// C++ `MainWindow.cpp` + `MainWindowRibbon.cpp` + `KaShellFocus.h` 단축키 1:1 처리
    pub(crate) fn handle_cpp_keyboard_shortcuts(&mut self, ctx: &egui::Context) -> escape::PendingCanvasInput {
        let pending = self.pending_canvas_input(ctx);
        // C++ 모달 창(KaFeatureFormDialog·조사구역 설정·도형 속성) 뒤에서는 주 창 단축키가 듣지 않는다 — 창이 Enter·Esc 를 맡는다.
        if self.modal_window_open() || self.modals.active == Some(ActiveModalDialog::PaperSetup) {
            return pending;
        }
        let wants_kb = ctx.wants_keyboard_input();
        let mut save_requested: Option<bool> = None;
        let mut open_requested = false;
        let mut paper_requested = false;
        // KaCaptureMapTool::keyPressEvent (target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391) and
        // target/cpp-ref/src/app/MainWindowUndo.cpp:486-495 apply Esc, Enter, Backspace and Delete to capture.
        let capture = !wants_kb && self.modals.active.is_none() && self.capture_active();
        let history = ctx.input(|i| {
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
            // target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391: capture keys run after menu owners.
            if !capture {
                if !wants_kb && self.active_tool == MapToolKind::GeorefAlign && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
                    self.delete_selected_align_point();
                } else if !wants_kb && self.active_tab == MainTab::Drawing && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
                    // On the sheet both keys go to the sheet (KaDrawingStudio.cpp:3512-3522).
                    if self.modals.active.is_none() && !self.modal_window_open() {
                        self.delete_sheet_selection();
                    }
                }
            }
            if i.modifiers.ctrl {
                if i.key_pressed(egui::Key::N) {
                    // `MainWindowRibbon.cpp:280` QKeySequence::New → newSurvey(「새 조사」 창).
                    self.request_new_survey();
                } else if i.key_pressed(egui::Key::O) {
                    // :282 QKeySequence::Open → openProject. 파일 창은 입력 잠금 밖에서 연다.
                    open_requested = true;
                } else if i.key_pressed(egui::Key::S) {
                    // 저장은 입력 잠금 밖에서 한다(다른 이름으로 저장은 파일 창을 연다).
                    save_requested = Some(i.modifiers.shift);
                } else if i.key_pressed(egui::Key::Num1) {
                    self.active_tab = MainTab::Map;
                    self.active_tool = MapToolKind::Select;
                } else if i.key_pressed(egui::Key::D) {
                    // MainWindowRibbon.cpp:354-360 — 「그리기」 단추와 같다.
                    self.toggle_draw_sub_tools();
                } else if i.key_pressed(egui::Key::L) {
                    // Same entry as the ribbon, target/cpp-ref/src/app/MainWindowExport.cpp:204-217.
                    self.handle_ribbon_click("도면");
                    paper_requested = self.modals.active == Some(ActiveModalDialog::PaperSetup);
                } else if i.key_pressed(egui::Key::P) {
                    // MainWindowExport.cpp:186-194 + KaPrintDialog.cpp:124 — Ctrl+P 도 같은 인쇄 창.
                    self.open_studio_tab(MainTab::Print);
                } else if i.key_pressed(egui::Key::E) {
                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                } else if !wants_kb && i.key_pressed(egui::Key::Z) && self.active_tab == MainTab::Drawing && self.drawing.coord_callout_mode {
                    // While points are placed on the sheet, the last point (KaDrawingStudio.cpp:3480-3483).
                    return Some(escape::HistoryAction::UndoCoordinates);
                } else if !wants_kb && i.key_pressed(egui::Key::Z) {
                    // undoMapAction — 그리던 점, 아니면 가장 최근 레이어 편집.
                    return Some(escape::HistoryAction::UndoMap);
                } else if !wants_kb && i.key_pressed(egui::Key::Y) {
                    return Some(escape::HistoryAction::RedoMap);
                }
            }
            None
        });
        if paper_requested { crate::modal_paper_account::paper_keyboard_entry(ctx); }
        let pending = pending.with_history(self, ctx, history);
        if open_requested {
            self.open_survey_file_dialog();
        }
        // C++ Ctrl+S = saveProject, Ctrl+Shift+S = saveProjectAs(리본 「저장」·「다른 이름」과 같은 길).
        match save_requested {
            Some(false) => {
                let _ = self.save_survey_project();
            }
            Some(true) => {
                let _ = self.save_survey_project_as();
            }
            None => {}
        }
        pending
    }
}

#[path = "startup_notice.rs"]
pub(crate) mod startup_notice;

impl eframe::App for StrataAppState {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.drop_heritage_run();
    }

    /// See-through around the card of the startup notice.
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        if self.startup_notice.is_some() {
            [0.0; 4]
        } else {
            visuals.panel_fill.to_normalized_gamma_f32()
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.show_startup_notice(ctx) {
            return;
        }
        self.drive_topographic_download(frame, ctx.input(|i| i.time));
        self.drive_topographic_post(ctx);
        self.drive_reference_maps(ctx, ctx.input(|i| i.time));
        self.drive_cadastral(ctx, ctx.input(|i| i.time));
        self.drive_heritage(ctx, frame, ctx.input(|i| i.time));
        let perf_frame = crate::perf_recorder::begin_frame();
        // 첫 프레임에 창이 화면보다 크면 화면 안으로 줄인다.
        //
        // C++ `MainWindow.cpp:277` 은 `showMaximized()` 로 시작한다. 그런데 Windows
        // 배율이 200% 인 화면에서 eframe/winit 은 요청한 inner_size 를 물리 픽셀로
        // 잡아, 논리 1280×900 이 3866×2090 창이 되어 리본 오른쪽 절반이 화면 밖으로
        // 나간다(사용자 지적 2026-11-09: 「배경 지도」 뒤 그룹이 안 보인다).
        // OS 최대화에 기대지 않고, 실제 모니터 작업영역 안으로 직접 맞춘다.
        if !self.startup_window_fitted {
            self.startup_window_fitted = true;
            let full = ctx.screen_rect();
            let dbg_mon = ctx.input(|i| i.viewport().monitor_size);
            if std::env::var("KA_HGIS_DEBUG_WIN").is_ok() {
                eprintln!("[WIN] screen_rect={:.1}x{:.1} monitor={:?} ppp={:.3}", full.width(), full.height(), dbg_mon, ctx.pixels_per_point());
            }
            if let Some(mon) = dbg_mon {
                // 창이 모니터보다 크면 작업영역(대략 모니터 - 작업표시줄 48px)에 맞춘다.
                let max_w = mon.x;
                let max_h = (mon.y - 48.0).max(600.0);
                if full.width() > max_w + 1.0 || full.height() > max_h + 1.0 {
                    let new_w = full.width().min(max_w);
                    let new_h = full.height().min(max_h);
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(new_w, new_h)));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                }
            }
        }
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
        crate::sound::on_frame_end(ctx, self);
        drop(perf_frame);
        // --snap 검증 모드: 액션→캡처를 프레임마다 하나씩 처리한다.
        self.snap_tick(ctx);
    }
}

#[cfg(test)]
#[path = "gui_shell_keys_tests.rs"]
mod keys_tests;

#[cfg(test)]
#[path = "popup_escape_tests.rs"]
mod popup_escape_tests;

#[cfg(test)]
#[path = "popup_input_harness.rs"]
mod popup_input_harness;

#[cfg(test)]
#[path = "native_popup_escape_tests.rs"]
mod native_popup_escape_tests;

#[cfg(test)]
#[path = "menu_capture_keys_tests.rs"]
mod menu_capture_keys_tests;

#[cfg(test)]
#[path = "menu_selection_keys_tests.rs"]
mod menu_selection_keys_tests;

#[cfg(test)]
#[path = "vertex_key_rejection_tests.rs"]
mod vertex_key_rejection_tests;

#[cfg(test)]
#[path = "save_before_input_tests.rs"]
mod save_before_input_tests;

#[cfg(test)]
#[path = "close_prompt_input_tests.rs"]
mod close_prompt_input_tests;

#[cfg(test)]
#[path = "vertex_crs_input_tests.rs"]
mod vertex_crs_input_tests;

#[cfg(test)]
#[path = "vertex_crs_boundary_tests.rs"]
mod vertex_crs_boundary_tests;

#[cfg(test)]
#[path = "capture_crs_input_tests.rs"]
mod capture_crs_input_tests;

#[cfg(test)]
#[path = "split_crs_input_tests.rs"]
mod split_crs_input_tests;

#[cfg(test)]
#[path = "buffer_crs_input_tests.rs"]
mod buffer_crs_input_tests;

#[cfg(test)]
#[path = "buffer_save_input_tests.rs"]
mod buffer_save_input_tests;

#[cfg(test)]
#[path = "old_map_input_tests.rs"]
mod old_map_input_tests;

#[cfg(test)]
#[path = "contextual_tools_input_tests.rs"]
mod contextual_tools_input_tests;

#[cfg(test)]
#[path = "align_snap_input_tests.rs"]
mod align_snap_input_tests;

#[cfg(test)]
#[path = "paper_setup_input_tests.rs"]
mod paper_setup_input_tests;

#[cfg(test)]
#[path = "legend_frame_input_tests.rs"]
mod legend_frame_input_tests;
