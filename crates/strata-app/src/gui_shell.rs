use eframe::egui::{self};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{ActiveModalDialog};
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
    pub(crate) fn handle_cpp_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let wants_kb = ctx.wants_keyboard_input();
        // 저장은 파일 창을 띄울 수 있어 입력 잠금(`ctx.input`) 밖에서 한다.
        let mut save_requested = false;
        let mut open_requested = false;
        // Some(true) = 되돌리기(Ctrl+Z), Some(false) = 다시하기(Ctrl+Y)
        let mut undo_requested: Option<bool> = None;
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
                if self.home_map.scene.active_rubberband.is_empty() {
                    // 그리는 중이 아니면 고른 도형을 지운다(C++ 선택 도형 삭제).
                    let _ = self.delete_selected_feature();
                } else {
                    self.home_map.cad.on_backspace(&mut self.home_map.scene);
                }
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
                    open_requested = true;
                } else if i.key_pressed(egui::Key::S) {
                    // C++ `saveProject` — 리본 「저장」과 같은 실제 저장. 결과 문구는 저장 함수가 남긴다.
                    save_requested = true;
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
                    // MainWindowExport.cpp:186-194 + KaPrintDialog.cpp:124 — Ctrl+P 도 같은 인쇄 창.
                    self.open_studio_tab(MainTab::Drawing);
                    self.modals.print_paper =
                        crate::dialogs_modal::default_print_paper_for_printer();
                    self.modals.active = Some(ActiveModalDialog::DrawingPrint);
                } else if i.key_pressed(egui::Key::E) {
                    self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                } else if i.key_pressed(egui::Key::Z) {
                    undo_requested = Some(true);
                } else if i.key_pressed(egui::Key::Y) {
                    undo_requested = Some(false);
                }
            }
        });
        if save_requested {
            let _ = self.save_survey_project();
        }
        match undo_requested {
            Some(true) => {
                let _ = self.undo_edit();
            }
            Some(false) => {
                let _ = self.redo_edit();
            }
            None => {}
        }
        if open_requested {
            // C++ `openProject` (Ctrl+O) — 파일 고르기 창.
            self.open_survey_file_dialog();
        }
    }
}

impl eframe::App for StrataAppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                eprintln!("[WIN] screen_rect={:.1}x{:.1} monitor={:?} ppp={:.3}",
                    full.width(), full.height(), dbg_mon, ctx.pixels_per_point());
            }
            if let Some(mon) = dbg_mon {
                // 창이 모니터보다 크면 작업영역(대략 모니터 - 작업표시줄 48px)에 맞춘다.
                let max_w = mon.x;
                let max_h = (mon.y - 48.0).max(600.0);
                if full.width() > max_w + 1.0 || full.height() > max_h + 1.0 {
                    let new_w = full.width().min(max_w);
                    let new_h = full.height().min(max_h);
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(
                        new_w, new_h,
                    )));
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
        // --snap 검증 모드: 액션→캡처를 프레임마다 하나씩 처리한다.
        self.snap_tick(ctx);
    }
}
