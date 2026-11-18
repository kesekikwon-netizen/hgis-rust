//! 리본 우클릭 팝업 메뉴(뒷부분) — 더보기·바깥 클릭 닫기.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use strata_net::tile_fetcher::BasemapSource;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;

/// Rows that are drawn and do not run yet (WU-07). The account window is not their stand-in.
const DEFERRED_WU07: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 WU-07)";
/// MainWindowRibbon.cpp:659-660, key from KaShellFocus.h:27.
const TIP_MAP_FOCUS: &str =
    "리본과 왼쪽 패널을 잠시 숨겨 지도를 넓게 봅니다. 다시 누르면 돌아옵니다 (Ctrl+F11)";
/// MainWindowRibbon.cpp:670-671, key from KaShellFocus.h:28.
const TIP_LEFT_PANEL: &str = "레이어·파일함 패널을 접거나 다시 폅니다 (F9)";

struct MoreRow {
    label: &'static str,
    tip: Option<&'static str>,
    enabled: bool,
}

fn more_row(label: &'static str) -> MoreRow {
    MoreRow {
        label,
        tip: None,
        enabled: true,
    }
}

fn deferred_row(label: &'static str) -> MoreRow {
    MoreRow {
        label,
        tip: Some(DEFERRED_WU07),
        enabled: false,
    }
}

impl StrataAppState {
    /// 팝업 메뉴 뒷부분. `anchor` 는 앞부분과 같은 값을 쓴다.
    pub(crate) fn render_ribbon_context_menus_b(
        &mut self,
        ctx: &egui::Context,
        anchor: Pos2,
        old_map_rect: Option<egui::Rect>,
    ) {
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
                    for row in [
                        more_row("벡터·도면 불러오기"),
                        deferred_row("참조 벡터를 조사 파일 밖으로…"),
                        deferred_row("조사 폴더 임시 자료 정리…"),
                        deferred_row("토양도 SHP 불러오기"),
                        deferred_row("CSV 기준점"),
                        more_row("맞추기"),
                        more_row("OSM 배경"),
                        more_row("Google 위성"),
                    ] {
                        if paint_more_row(ui, &row) {
                            picked = Some(row.label);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("화면")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for row in [
                        more_row("파일함 보이기/숨기기"),
                        MoreRow {
                            label: "지도 넓게 보기",
                            tip: Some(TIP_MAP_FOCUS),
                            enabled: true,
                        },
                        MoreRow {
                            label: "왼쪽 패널 접기/펴기",
                            tip: Some(TIP_LEFT_PANEL),
                            enabled: true,
                        },
                        more_row("화면 보기 (고대비·큰 글씨·테마)…"),
                    ] {
                        if paint_more_row(ui, &row) {
                            picked = Some(row.label);
                        }
                    }
                    ui.separator();
                    ui.label(
                        egui::RichText::new("계정과 키 · 상태와 정보")
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(110, 108, 102)),
                    );
                    for row in [
                        more_row("API 키 입력 · 계정 설정…"),
                        deferred_row("자료 준비 상태"),
                        more_row("정보"),
                    ] {
                        if paint_more_row(ui, &row) {
                            picked = Some(row.label);
                        }
                    }
                });
            if let Some(item) = picked {
                self.ribbon.more_menu_open = false;
                self.apply_more_menu_item(item);
            }
        }
        // target/cpp-ref/src/app/MainWindowContextMenus.cpp:192: QMenu closes on Escape as well as an outside click.
        // target/cpp-ref/src/app/MainWindowRibbon.cpp:490: the opening click belongs to the whole toolbutton.
        let just_opened = std::mem::take(&mut self.ribbon.context_menu_just_opened);
        let outside = ctx.input(|i| {
            !just_opened
                && i.pointer.any_click()
                && i.pointer.interact_pos().is_some_and(|p| {
                    old_map_rect.map_or_else(
                        || {
                            !(anchor.x - 16.0 <= p.x
                                && p.x <= anchor.x + 450.0
                                && anchor.y - 36.0 <= p.y
                                && p.y <= anchor.y + 420.0)
                        },
                        |rect| !rect.contains(p),
                    )
                })
        });
        if outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
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

    /// One row, one action. Unmatched text does not open the account window.
    fn apply_more_menu_item(&mut self, item: &str) {
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
            self.open_vector_layer();
        } else if item == "지도 넓게 보기" {
            self.toggle_map_focus();
        } else if item == "왼쪽 패널 접기/펴기" {
            self.toggle_left_panel();
        } else if item.contains("API 키") {
            self.modals.active = Some(ActiveModalDialog::AccountSettings);
        } else if item == "정보" {
            self.modals.active = Some(ActiveModalDialog::About);
        }
    }
}

fn paint_more_row(ui: &mut egui::Ui, row: &MoreRow) -> bool {
    let button = egui::Button::new(egui::RichText::new(row.label).size(12.0))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(232.0, 22.0));
    let response = ui.add_enabled(row.enabled, button);
    let response = match row.tip {
        Some(tip) if row.enabled => response.on_hover_text(tip),
        Some(tip) => response.on_disabled_hover_text(tip),
        None => response,
    };
    response.clicked()
}
