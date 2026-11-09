//! 리본 우클릭 팝업 메뉴(뒷부분) — 더보기·바깥 클릭 닫기.

use eframe::egui::{ self, Color32, Pos2, Stroke };

use strata_net::tile_fetcher::BasemapSource;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

impl StrataAppState {
    /// 팝업 메뉴 뒷부분. `anchor` 는 앞부분과 같은 값을 쓴다.
    pub(crate) fn render_ribbon_context_menus_b(&mut self, ctx: &egui::Context, anchor: Pos2) {
        // ── 더보기 (MainWindowRibbon.cpp:628-710 — 불러오기·배경 / 화면 / 계정과 키 / 상태와 정보)
        if self.ribbon.more_menu_open {
            let mut picked: Option<&'static str> = None;
            egui::Window::new("ka_more_menu")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(anchor)
                .show(ctx, |ui| {
                    ui.set_min_width(236.0);
                    ui.label(
                        egui::RichText::new("불러오기·배경")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in [
                        "벡터·도면 불러오기",
                        "참조 벡터를 조사 파일 밖으로…",
                        "조사 폴더 임시 자료 정리…",
                        "토양도 SHP 불러오기",
                        "CSV 기준점",
                        "맞추기",
                        "OSM 배경",
                        "Google 위성",
                    ] {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("화면")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in [
                        "파일함 보이기/숨기기",
                        "지도 넓게 보기 (F11)",
                        "왼쪽 패널 접기/펴기 (F9)",
                        "화면 보기 (고대비·큰 글씨·테마)…",
                    ] {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
                                egui::Button::new(egui::RichText::new(item).size(12.0))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            picked = Some(item);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("계정과 키 · 상태와 정보")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for item in ["API 키 입력 · 계정 설정…", "자료 준비 상태", "정보"]
                    {
                        if ui
                            .add_sized(
                                [232.0, 22.0],
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
                self.ribbon.more_menu_open = false;
                if item.contains("화면 보기") {
                    self.theme_menu_open = !self.theme_menu_open;
                } else if item.contains("맞추기") {
                    self.handle_ribbon_click("정합");
                } else if item.contains("OSM 배경") {
                    self.active_tab = MainTab::Map;
                    self.home_map.active_basemap = BasemapSource::VworldBase;
                } else if item.contains("Google 위성") {
                    self.active_tab = MainTab::Map;
                    self.home_map.active_basemap = BasemapSource::VworldSatellite;
                } else if item.contains("파일함") {
                    self.home_map.files_open = !self.home_map.files_open;
                } else if item.contains("벡터·도면") {
                    self.modals.active = Some(ActiveModalDialog::CadImportNotice);
                } else {
                    self.modals.active = Some(ActiveModalDialog::AccountSettings);
                }
            }
        }
        // 바깥을 누르면 닫는다
        if ctx.input(|i| i.pointer.any_click()) {
            let any_open = self.ribbon.paleo_menu_open
                || self.ribbon.geology_menu_open
                || self.ribbon.dem_menu_open
                || self.ribbon.soil_menu_open
                || self.ribbon.cadastral_menu_open
                || self.ribbon.topo_menu_open
                || self.ribbon.old_map_menu_open
                || self.ribbon.more_menu_open;
            if any_open {
                let pos = ctx.input(|i| i.pointer.interact_pos());
                if let Some(p) = pos {
                    let near = anchor.x - 16.0 <= p.x
                        && p.x <= anchor.x + 450.0
                        && anchor.y - 36.0 <= p.y
                        && p.y <= anchor.y + 420.0;
                    if !near {
                        self.ribbon.paleo_menu_open = false;
                        self.ribbon.geology_menu_open = false;
                        self.ribbon.dem_menu_open = false;
                        self.ribbon.soil_menu_open = false;
                        self.ribbon.cadastral_menu_open = false;
                        self.ribbon.topo_menu_open = false;
                        self.ribbon.old_map_menu_open = false;
                        self.ribbon.more_menu_open = false;
                    }
                }
            }
        }
    }
}
