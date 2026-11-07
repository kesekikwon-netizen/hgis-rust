use eframe::egui::{self, Color32};
use strata_types::crs::WorkCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::{render_active_modal_dialogs, ActiveModalDialog};

impl StrataAppState {
    /// C++ `MainWindow.cpp` + `MainWindowChrome.cpp` + `KaStatusBar.cpp` 1:1 재현
    pub fn render_workspace_egui(&mut self, ctx: &egui::Context) -> egui::Rect {
        self.poll_debug_remote_commands(ctx);
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.home_map.cad.on_escape(&mut self.home_map.scene);
            self.modals.active = None;
        }

        // 1. 상단 1행 리본(그룹명 상단 + 40x40 아이콘 박스 + 우측 지역/주소검색) + 보조 툴바 + 탭 스트립
        egui::TopBottomPanel::top("ka_cpp_top_chrome").show(ctx, |ui| {
            self.render_cpp_top_bars(ctx, ui);
        });

        // 2. 하단 상태 표시줄 (KaStatusBar.cpp 1:1)
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
                                "1:4846 ▼"
                            };
                            let _ = ui.small_button(scale_str);
                            ui.label(egui::RichText::new("축척").size(11.0));
                            if self.active_tab == MainTab::Map {
                                ui.separator();
                                ui.label(egui::RichText::new("X 219,333.191    Y 361,059.545").size(11.0));
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

        // 3. 중앙 본문 (탭별 1:1 화면)
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
