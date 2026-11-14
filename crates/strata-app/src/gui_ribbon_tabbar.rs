//! 스튜디오 탭바 — 홈·지도·도면·고고·3D 탭과 열린 도면 탭.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::{MainTab, StrataAppState};
use crate::ribbon_icons::paint_ribbon_icon;

use super::gui_ribbon_consts::UX_VIEW_TAB_BAR_HEIGHT;

impl StrataAppState {
    /// 탭바 한 줄.
    pub(crate) fn paint_ribbon_tab_bar(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) {
        let tabbar_h = if ctx.pixels_per_point() > 1.2 {
            34.0
        } else {
            UX_VIEW_TAB_BAR_HEIGHT
        };
        let (tabbar_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), tabbar_h),
            egui::Sense::hover(),
        );
        {
            let tbp = ui.painter_at(tabbar_rect);
            tbp.rect_filled(tabbar_rect, 0.0, Color32::from_rgb(0xF5, 0xF4, 0xED));
            tbp.line_segment(
                [
                    Pos2::new(tabbar_rect.left(), tabbar_rect.bottom() - 1.0),
                    Pos2::new(tabbar_rect.right(), tabbar_rect.bottom() - 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(214, 211, 200)),
            );
        }
        let tabbar_inner = tabbar_rect.shrink2(Vec2::new(12.0, 2.0));
        let mut tabbar_ui = ui.new_child(egui::UiBuilder::new().max_rect(tabbar_inner));
        tabbar_ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for (tab, ico, txt) in [
                (MainTab::Home, "유산", "홈"),
                (MainTab::Map, "옛 지도", "지도"),
            ] {
                let sel = self.active_tab == tab;
                let (tr, resp) =
                    ui.allocate_exact_size(Vec2::new(64.0, tabbar_h - 4.0), egui::Sense::click());
                let tp = ui.painter_at(tr);
                paint_ribbon_icon(
                    &tp,
                    Rect::from_center_size(
                        Pos2::new(tr.left() + 12.0, tr.center().y),
                        Vec2::new(26.0, 26.0),
                    ),
                    ico,
                    Color32::from_rgb(45, 44, 40),
                );
                tp.text(
                    Pos2::new(tr.left() + 28.0, tr.center().y),
                    egui::Align2::LEFT_CENTER,
                    txt,
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(30, 30, 28),
                );
                if sel {
                    tp.line_segment(
                        [
                            Pos2::new(tr.left(), tr.bottom()),
                            Pos2::new(tr.right(), tr.bottom()),
                        ],
                        Stroke::new(2.5, Color32::from_rgb(35, 35, 32)),
                    );
                }
                if resp.clicked() {
                    self.active_tab = tab;
                }
            }
            let open_tabs = self.open_studio_tabs.clone();
            for (extra, ico, lbl) in [
                (MainTab::Drawing, "도면", "도면"),
                (MainTab::Paleo, "고지형", "고지형"),
                (MainTab::Terrain3d, "검수·제출", "입체지형"),
            ] {
                if open_tabs.contains(&extra) || self.active_tab == extra {
                    let sel = self.active_tab == extra;
                    let (tr, resp) =
                        ui.allocate_exact_size(Vec2::new(92.0, 36.0), egui::Sense::click());
                    let tp = ui.painter_at(tr);
                    paint_ribbon_icon(
                        &tp,
                        Rect::from_center_size(
                            Pos2::new(tr.left() + 12.0, tr.center().y),
                            Vec2::new(22.0, 22.0),
                        ),
                        ico,
                        Color32::from_rgb(35, 35, 32),
                    );
                    tp.text(
                        Pos2::new(tr.left() + 28.0, tr.center().y),
                        egui::Align2::LEFT_CENTER,
                        lbl,
                        egui::FontId::proportional(13.0),
                        Color32::from_rgb(30, 30, 28),
                    );
                    let close_rect =
                        Rect::from_min_max(Pos2::new(tr.right() - 18.0, tr.top()), tr.max);
                    tp.text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        egui::FontId::proportional(13.5),
                        Color32::from_rgb(90, 89, 84),
                    );
                    if sel {
                        tp.line_segment(
                            [
                                Pos2::new(tr.left(), tr.bottom()),
                                Pos2::new(tr.right(), tr.bottom()),
                            ],
                            Stroke::new(2.5, Color32::from_rgb(35, 35, 32)),
                        );
                    }
                    if resp.clicked() {
                        if resp.interact_pointer_pos().is_some_and(|p| close_rect.contains(p)) {
                            self.close_studio_tab(extra);
                        } else {
                            self.active_tab = extra;
                        }
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let mut theme_popup_rect: Option<egui::Rect> = None;
                if self.theme_menu_open {
                    let popup_pos = Pos2::new(
                        (tabbar_rect.right() - 220.0).max(16.0),
                        tabbar_rect.bottom() + 4.0,
                    );
                    let area_resp = egui::Area::new(ui.id().with("ka_theme_menu_popup"))
                        .fixed_pos(popup_pos)
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style())
                                .fill(Color32::from_rgb(252, 251, 247))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .inner_margin(egui::Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.set_min_width(198.0);
                                    for ct in strata_types::theme_tokens::ColorTheme::ALL {
                                        let changed = ui
                                            .radio_value(
                                                &mut self.display_options.color_theme,
                                                ct,
                                                ct.label_ko(),
                                            )
                                            .on_hover_text(ct.tooltip_ko())
                                            .changed();
                                        if changed {
                                            self.theme =
                                                strata_types::theme_tokens::ThemeTokens::for_options(
                                                    &self.display_options,
                                                );
                                            self.theme_dirty = true;
                                        }
                                    }
                                    ui.separator();
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.paper_look,
                                            "새 모양 (종이·흙색)",
                                        )
                                        .changed()
                                    {
                                        self.paper_look = self.display_options.paper_look;
                                        self.theme =
                                            strata_types::theme_tokens::ThemeTokens::for_options(
                                                &self.display_options,
                                            );
                                        self.theme_dirty = true;
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.high_contrast,
                                            "고대비 화면 (햇빛 아래)",
                                        )
                                        .changed()
                                    {
                                        self.theme =
                                            strata_types::theme_tokens::ThemeTokens::for_options(
                                                &self.display_options,
                                            );
                                        self.theme_dirty = true;
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.display_options.large_text,
                                            "큰 글씨",
                                        )
                                        .changed()
                                    {
                                        self.large_font_mode = self.display_options.large_text;
                                        ctx.set_zoom_factor(if self.large_font_mode {
                                            1.15
                                        } else {
                                            1.0
                                        });
                                    }
                                    ui.separator();
                                    for sl in strata_types::theme_display::SoundLevel::ALL {
                                        let changed = ui
                                            .radio_value(
                                                &mut self.display_options.sound_level,
                                                sl,
                                                sl.label_ko(),
                                            )
                                            .changed();
                                        if changed && self.snap.is_none() {
                                            if let Some(rust_ini) = self
                                                .settings_ini
                                                .as_deref()
                                                .and_then(crate::user_settings::rust_settings_ini_from_shared)
                                            {
                                                crate::rust_settings::save_sound_level(
                                                    &rust_ini,
                                                    self.display_options.sound_level,
                                                );
                                            }
                                        }
                                    }
                                });
                        });
                    theme_popup_rect = Some(area_resp.response.rect);
                }
                // MainWindowChrome.cpp:144-155 — 탭 줄 오른쪽 끝: 「테마 ▾」(언제나)와 그 왼쪽의 조사 배지.
                let theme_btn = egui::Button::new(
                    egui::RichText::new("테마 ▼")
                        .size(12.0)
                        .color(Color32::from_rgb(45, 44, 40)),
                )
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, Color32::from_rgb(195, 192, 182)))
                .corner_radius(5.0)
                .min_size(Vec2::new(56.0, 24.0));
                let theme_resp = ui
                    .add(theme_btn)
                    .on_hover_text("화면 테마·모양·글자 크기 설정");
                let theme_toggled = theme_resp.clicked();
                if theme_toggled {
                    self.theme_menu_open = !self.theme_menu_open;
                }
                // KaThemeOptions.cpp — QMenu closes on Escape or outside click.
                if !theme_toggled && self.theme_menu_open {
                    let close = ctx.input(|i| {
                        i.key_pressed(egui::Key::Escape)
                            || ((i.pointer.any_pressed() || i.pointer.any_click())
                                && i.pointer.interact_pos().is_some_and(|p| {
                                    !theme_resp.rect.contains(p)
                                        && theme_popup_rect.is_some_and(|r| !r.contains(p))
                                }))
                    });
                    if close {
                        self.theme_menu_open = false;
                    }
                }
                // KaSurveyBadge.cpp:32-62 — 조사 파일이 열려 있을 때만 「<파일 이름> 저장됨」(새 모양).
                let survey_name = self
                    .survey_path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().trim().to_owned())
                    .filter(|s| !s.is_empty());
                if let Some(name) = survey_name {
                    if self.is_dirty {
                        ui.label(
                            egui::RichText::new("● 저장 안 됨")
                                .size(12.0)
                                .color(Color32::from_rgb(181, 87, 58)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("저장됨")
                                .size(12.0)
                                .color(Color32::from_rgb(94, 93, 89)),
                        );
                    }
                    ui.label(
                        egui::RichText::new(name)
                            .size(12.0)
                            .color(Color32::from_rgb(94, 93, 89)),
                    )
                    .on_hover_text("열려 있는 조사입니다. 누르면 지도 탭으로 갑니다.");
                }
            });
        });

    }
}
