//! 리본 우클릭 팝업 메뉴(앞부분) — 고지형·지질·DEM·토양·지적도·지형도.

use eframe::egui::{ self, Color32, Stroke };

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 팝업 메뉴를 그린다. 항목을 고르면 닫고 실제 동작을 실행한다.
    pub(crate) fn render_ribbon_context_menus(&mut self, ctx: &egui::Context) {
        let anchor = self.ribbon.context_menu_anchor;
        // ── 고지형 (7항목, KaPaleoMenu::install)
        if self.ribbon.paleo_menu_open {
            let mut open = true;
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_paleo_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 7] = [
                        "「고지형분석」 전용 탭 열기",
                        "수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 창…",
                        "수치지질도(KIGAM 1:5만) 별도 받기 (반경 5km · 도면 범례)",
                        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km · 도면 범례)",
                        "KIGAM 단층·지하수위·단구연대 별도 받기 (반경 10km)",
                        "국토지반정보 포털 열기 (geoinfo.or.kr)…",
                        "KIGAM 지오빅데이터 포털 열기 (data.kigam.re.kr)…",
                    ];
                    for (i, item) in ITEMS.iter().enumerate() {
                        if i == 2 || i == 5 {
                            ui.separator();
                        }
                        if ui
                            .add_sized(
                                [420.0, 24.0],
                                egui::Button::new(egui::RichText::new(*item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.paleo_menu_open = false;
                self.run_paleo_menu_action(item);
                open = false;
            }
            let _ = open;
        }
        // ── 지질 (6항목, MainWindowRibbon.cpp geoMenu)
        if self.ribbon.geology_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_geology_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 6] = [
                        "조사구역 반경 5km 수치지질도 받기 (1:5만 · 권장)",
                        "조사구역 반경 2km 수치지질도 받기",
                        "조사구역 반경 10km 수치지질도 받기",
                        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km)",
                        "KIGAM 단층·선구조·수문 별도 받기 (반경 10km)",
                        "수치지질도 · 시추주상도 별도 내려받기 창…",
                    ];
                    for (i, item) in ITEMS.iter().enumerate() {
                        if i == 3 {
                            ui.separator();
                        }
                        if ui
                            .add_sized(
                                [400.0, 24.0],
                                egui::Button::new(egui::RichText::new(*item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.geology_menu_open = false;
                self.run_geology_menu_action(item);
            }
        }
        // ── DEM (5항목, MainWindowChrome.cpp:215-223)
        if self.ribbon.dem_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_dem_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    const ITEMS: [&str; 5] = [
                        "국토지리정보원 계정 설정·자동 다운로드…",
                        "국토지리원 DEM 불러오기(.img)…",
                        "DEM 파일로 음영 만들기…",
                        "DEM 표현…",
                        "측량점으로 등고선…",
                    ];
                    for item in ITEMS {
                        if ui
                            .add_sized(
                                [330.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.dem_menu_open = false;
                self.run_dem_menu_action(item);
            }
        }
        // ── 토양 (2항목, MainWindowRibbon.cpp:447-452)
        if self.ribbon.soil_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_soil_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in [
                        "분포지형 내려받기 — 현재 화면 범위",
                        "토양도 파일 불러오기(SHP·GPKG)",
                    ] {
                        if ui
                            .add_sized(
                                [250.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(_item) = picked {
                self.ribbon.soil_menu_open = false;
                self.handle_ribbon_click("토양");
            }
        }
        // ── 지적 (2항목, MainWindowRibbon.cpp:477-478)
        if self.ribbon.cadastral_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_cadastral_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["VWorld 계정 설정", "선 색·지번 표시"] {
                        if ui
                            .add_sized(
                                [190.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.cadastral_menu_open = false;
                if item.contains("계정") {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                } else {
                    self.modals.active = Some(ActiveModalDialog::CadastralDownload);
                }
            }
        }
        // ── 수치지형 (2항목, MainWindowRibbon.cpp:391-392)
        if self.ribbon.topo_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_topo_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["국토지리정보원에서 내려받기…", "받아 둔 수치지형도 폴더…"]
                    {
                        if ui
                            .add_sized(
                                [230.0, 24.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(_item) = picked {
                self.ribbon.topo_menu_open = false;
                self.modals.active = Some(ActiveModalDialog::NgiiTopoDownload);
            }
        }
        // ── 옛 지도 (2항목, MainWindowRibbon.cpp:492-500 & 12-oldmap.png / 14-drawing.png)
        if self.ribbon.old_map_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_old_map_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    for item in ["▤  대동여지도", "▤  1919 조선지형도 1:5만"] {
                        if ui
                            .add_sized(
                                [196.0, 26.0],
                                egui::Button::new(egui::RichText::new(item).size(12.5))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.old_map_menu_open = false;
                self.active_tab = MainTab::Map;
                let map_type = if item.contains("대동여지도") {
                    strata_net::tile_fetcher::HistoricalMapType::Daedongyeojido
                } else {
                    strata_net::tile_fetcher::HistoricalMapType::Chosun1919_1to50000
                };
                let spec = strata_net::tile_fetcher::build_historical_tms_layer_spec(map_type, 0.8);
                self.home_map.draw_guide_message = Some(format!(
                    "{} ({}) 배경을 적용했습니다 (투명도 {:.0}%).",
                    spec.layer_title_ko,
                    self.work_crs.auth_id(),
                    spec.opacity * 100.0
                ));
            }
        }
        self.render_ribbon_context_menus_b(ctx, anchor);
    }
}
