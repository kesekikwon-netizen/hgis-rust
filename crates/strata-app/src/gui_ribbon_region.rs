//! 지역 찾기 팝업 — 17개 시·도 격자와 주소 팝업 (`KaRegionLocator.cpp:38-286`).
//! 한 함수가 300줄을 넘어 둘로 나눴고, 갈래가 배타적이라 차례로 불러도 결과가 같다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::ribbon::PENINSULA_REGION_TILES;

impl StrataAppState {
    /// 지역 팝업 — 시·도 격자와 주소 팝업.
    pub(crate) fn render_region_locator_popup(&mut self, ctx: &egui::Context) {
        let screen_r = ctx.screen_rect();
        let [btn_min_x, _btn_min_y, _btn_max_x, btn_max_y] =
            self.ribbon.app_bar.region_button_rect.unwrap_or([
                screen_r.right() - 252.0,
                34.0,
                screen_r.right() - 176.0,
                70.0,
            ]);

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.ribbon.app_bar.region_popup_open = false;
            self.ribbon.app_bar.address_popup_open = false;
        }

        // 1단계: 「지역」 드롭다운 메뉴 (17개 광역 시·도 칩 격자만 표시 — media_1794135047748_487cba62.png 1:1)
        if self.ribbon.app_bar.region_popup_open {
            let menu_w = 214.0_f32;
            let menu_x = btn_min_x.clamp(8.0, (screen_r.right() - menu_w - 8.0).max(8.0));
            let menu_pos = Pos2::new(menu_x, btn_max_y + 1.0);
            let mut clicked_sido: Option<String> = None;

            let area_resp = egui::Area::new(egui::Id::new("ka_app_bar_region_menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(menu_pos)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(118, 118, 118)))
                        .corner_radius(0.0)
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            egui::Grid::new("ka_peninsula_17_grid")
                                .spacing([6.0, 6.0])
                                .show(ui, |ui| {
                                    for r in 0..6 {
                                        for c in 0..4 {
                                            if let Some(tile) = PENINSULA_REGION_TILES
                                                .iter()
                                                .find(|t| t.row == r && t.col == c)
                                            {
                                                let (chip_rect, chip_resp) = ui
                                                    .allocate_exact_size(
                                                        Vec2::new(42.0, 26.0),
                                                        egui::Sense::click(),
                                                    );
                                                let is_checked =
                                                    self.ribbon.app_bar.address_popup_open
                                                        && self.ribbon.app_bar.selected_sido
                                                            == tile.name;
                                                let bg = if is_checked {
                                                    Color32::from_rgb(239, 236, 226)
                                                } else if chip_resp.hovered() {
                                                    Color32::from_rgb(245, 244, 237)
                                                } else {
                                                    Color32::WHITE
                                                };
                                                let border_col =
                                                    if is_checked || chip_resp.hovered() {
                                                        Color32::from_rgb(181, 87, 58)
                                                    } else {
                                                        Color32::from_rgb(118, 118, 118)
                                                    };
                                                let cp = ui.painter_at(chip_rect);
                                                cp.rect_filled(chip_rect, 6.0, bg);
                                                cp.rect_stroke(
                                                    chip_rect,
                                                    6.0,
                                                    Stroke::new(1.0, border_col),
                                                    egui::StrokeKind::Inside,
                                                );
                                                cp.text(
                                                    chip_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    tile.name,
                                                    egui::FontId::proportional(12.0),
                                                    Color32::from_rgb(140, 54, 22),
                                                );
                                                if chip_resp
                                                    .on_hover_text(
                                                        "시·도를 누르면 시·동·번지로 찾습니다",
                                                    )
                                                    .clicked()
                                                {
                                                    clicked_sido = Some(tile.name.to_owned());
                                                }
                                            } else {
                                                ui.allocate_space(Vec2::new(42.0, 26.0));
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });

            if let Some(sido_short) = clicked_sido {
                self.ribbon.app_bar.selected_sido = sido_short.clone();
                self.ribbon.app_bar.city_input.clear();
                self.ribbon.app_bar.dong_input.clear();
                self.ribbon.app_bar.lot_input.clear();
                self.ribbon.app_bar.region_popup_open = false;
                self.ribbon.app_bar.address_popup_open = true;
                self.zoom_to_sido_overview(&sido_short);
            } else if ctx.input(|i| i.pointer.any_pressed()) {
                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    let btn_rect =
                        self.ribbon.app_bar.region_button_rect.map(|b| {
                            Rect::from_min_max(Pos2::new(b[0], b[1]), Pos2::new(b[2], b[3]))
                        });
                    let in_btn = btn_rect.is_some_and(|r| r.contains(pos));
                    if !area_resp.response.rect.contains(pos) && !in_btn {
                        self.ribbon.app_bar.region_popup_open = false;
                    }
                }
            }
        }
        self.paint_region_address_popup(ctx, screen_r, btn_min_x, btn_max_y);
        self.render_region_locator_popup_b(ctx);
    }
}
