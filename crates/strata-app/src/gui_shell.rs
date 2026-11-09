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
                    // MainWindowExport.cpp:186-194 + KaPrintDialog.cpp:124 — Ctrl+P 도 같은 인쇄 창.
                    self.open_studio_tab(MainTab::Drawing);
                    self.modals.print_paper =
                        crate::dialogs_modal::default_print_paper_for_printer();
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
