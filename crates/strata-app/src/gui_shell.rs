use eframe::egui;
use strata_types::crs::WorkCrs;
use strata_types::theme_tokens::ThemeTokens;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};
use crate::gui_theme_fonts::apply_theme_tokens;

impl StrataAppState {
    /// `MainWindow.cpp` + `MainWindowChrome.cpp` + `KaStatusBar.cpp` 1:1 재현
    pub fn render_workspace_egui(&mut self, ctx: &egui::Context) -> egui::Rect {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.home_map.cad.on_escape(&mut self.home_map.scene);
        }

        // 1. 상단 1행 리본(7개 그룹 + 지역·검색) 및 2행 세부도구바(subToolbar)
        self.render_top_ribbon_and_subtoolbar(ctx);

        // 2. 하단 상태 표시줄 (KaStatusBar.cpp + MainWindow.cpp:1087-1126)
        egui::TopBottomPanel::bottom("ka_status_bar")
            .exact_height(28.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.active_tab == MainTab::Home {
                        // MainWindow.cpp:395,1124 — 홈 탭에서는 안내 문구만 표시하고 좌표·축척·CRS 숨김
                        ui.label("먼저 「새 조사」로 오늘 현장을 만드세요.");
                    } else {
                        let status_msg = self
                            .home_map
                            .draw_guide_message
                            .as_deref()
                            .unwrap_or("준비됨");
                        ui.label(status_msg);
                        if let Some(ref m) = self.home_map.cad.last_measure {
                            ui.separator();
                            ui.strong(format!(
                                "거리 {:.1}m · 면적 {:.1}㎡",
                                m.total_distance_m, m.polygon_area_m2
                            ));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // MainWindowChrome.cpp:163 — 「→ 제출 5179」 클릭 시 검수·제출 열기
                            if ui
                                .button("→ 제출 5179")
                                .on_hover_text("제출용 파일은 항상 EPSG:5179로 나갑니다 (클릭: 검수·제출)")
                                .clicked()
                            {
                                self.modals.active = Some(ActiveModalDialog::ExportSubmitPackage);
                            }
                            if ui
                                .button(format!("{} ▾", self.work_crs.label_ko()))
                                .on_hover_text("클릭하여 중부원점(EPSG:5186) / 동부원점(EPSG:5187) 전환")
                                .clicked()
                            {
                                self.work_crs = match self.work_crs {
                                    WorkCrs::Epsg5187 => WorkCrs::Epsg5186,
                                    WorkCrs::Epsg5186 => WorkCrs::Epsg5187,
                                };
                            }
                            ui.checkbox(&mut self.home_map.grid_enabled, "좌표격자");
                            ui.separator();
                            ui.label("축척 1:2,500");
                            ui.separator();
                            ui.label(format!(
                                "좌표 {:.1}, {:.1}",
                                self.home_map.cursor_world_xy[0], self.home_map.cursor_world_xy[1]
                            ));
                        });
                    }
                });
            });

        // 3. 중앙 탭 바 (m_viewTabs + 우측 끝 KaSurveyBadge & 테마 스위치) 및 각 탭 본문
        let mut central_rect = egui::Rect::NOTHING;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                for &tab in MainTab::all() {
                    let is_studio = matches!(
                        tab,
                        MainTab::Paleo | MainTab::Drawing | MainTab::Terrain3d
                    );
                    let lbl = if is_studio {
                        format!("{} ✕", tab.label_ko())
                    } else {
                        tab.label_ko().to_owned()
                    };
                    if ui.selectable_label(self.active_tab == tab, lbl).clicked() {
                        self.active_tab = tab;
                    }
                }
                // MainWindowChrome.cpp:150-161 — viewTabsCorner (조사 열림 배지 + 테마 ▾)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .checkbox(&mut self.paper_look, "조용한 도판")
                        .changed()
                    {
                        self.theme = if self.paper_look {
                            ThemeTokens::paper_tokens(false)
                        } else {
                            ThemeTokens::stock_tokens(false)
                        };
                        apply_theme_tokens(ctx, &self.theme);
                    }
                    let badge_txt = format!("조사 열림 · {}", self.project_title);
                    if ui
                        .button(badge_txt)
                        .on_hover_text("누르면 지도 탭으로 이동합니다")
                        .clicked()
                    {
                        self.active_tab = MainTab::Map;
                    }
                });
            });
            ui.separator();
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
