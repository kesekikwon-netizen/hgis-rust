//! `KaShellGridControls.cpp:41-193` — 우하단 좌표격자 토글·종류·간격·라벨·선색.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_map_consts::{
    ShellGridConfig, SHELL_GRID_DASH_LABELS, SHELL_GRID_SWATCHES,
};
impl StrataAppState {
    pub(crate) fn paint_shell_grid_controls(
    &mut self,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    p: &egui::Painter,
    rect: Rect,
) {
    let mut cfg = ctx.data_mut(|d| {
        d.get_temp::<ShellGridConfig>(egui::Id::new("ka_shell_grid_cfg"))
            .unwrap_or_default()
    });
    let enabled = self.home_map.grid_enabled;

    // KaShellGridControls.cpp:120-125 — 세부 설정은 「좌표격자」를 켤 때만 보인다
    let detail_h = if enabled { 60.0 } else { 0.0 };
    let bar_h = 28.0 + detail_h;
    let bar_w = if enabled { 344.0 } else { 96.0 };
    let bar_r = Rect::from_min_size(
        Pos2::new(rect.right() - bar_w - 12.0, rect.bottom() - bar_h - 10.0),
        Vec2::new(bar_w, bar_h),
    );
    p.rect_filled(
        bar_r,
        6.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, 246),
    );
    p.rect_stroke(
        bar_r,
        6.0,
        Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
        egui::StrokeKind::Middle,
    );

    let detail_r = Rect::from_min_size(bar_r.min, Vec2::new(bar_w, detail_h));
    if enabled {
        ui.scope_builder(egui::UiBuilder::new().max_rect(detail_r.shrink2(Vec2::new(8.0, 6.0))), |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
            // Row 0 — 미터/경위도 · 간격 · 회전 (KaShellGridControls.cpp:113-116)
            ui.horizontal(|ui| {
                let kind_lbl = if cfg.is_geo { "경위도 격자" } else { "미터 격자" };
                if ui
                    .selectable_label(cfg.is_geo, kind_lbl)
                    .on_hover_text(
                        "미터 격자는 작업 좌표계(5186·5187) 좌표로, 경위도 격자는 도·분·초로 긋습니다.",
                    )
                    .clicked()
                {
                    cfg.is_geo = !cfg.is_geo;
                }
                ui.add_enabled(
                    !cfg.is_geo,
                    egui::DragValue::new(&mut cfg.step_m)
                        .range(0.1..=10000.0)
                        .speed(1.0)
                        .fixed_decimals(1)
                        .prefix("간격 ")
                        .suffix(" m"),
                )
                .on_hover_text("격자 간격(미터). 시굴격자를 옮길 때도 이 간격에 붙습니다.");
                ui.add_enabled(
                    !cfg.is_geo,
                    egui::DragValue::new(&mut cfg.rotation_deg)
                        .range(0.0..=360.0)
                        .speed(1.0)
                        .fixed_decimals(1)
                        .prefix("회전 ")
                        .suffix("°"),
                )
                .on_hover_text("격자 회전 (동쪽 기준 시계 방향)");
            });
            // Row 1 — 굵기 · 선 모양 · 색 견본 (KaShellGridControls.cpp:116-118)
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut cfg.line_width)
                        .range(0.5..=5.0)
                        .speed(0.1)
                        .fixed_decimals(1)
                        .prefix("굵기 "),
                )
                .on_hover_text("격자 선 굵기");
                let dash_lbl = SHELL_GRID_DASH_LABELS[cfg.dash_idx.min(2)];
                if ui
                    .selectable_label(false, dash_lbl)
                    .on_hover_text("격자 선 모양")
                    .clicked()
                {
                    cfg.dash_idx = (cfg.dash_idx + 1) % SHELL_GRID_DASH_LABELS.len();
                }
                for (si, (scol, stip)) in SHELL_GRID_SWATCHES.into_iter().enumerate() {
                    let (sr, resp) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::click());
                    p.rect_filled(sr, 3.0, scol);
                    if cfg.color_idx == si {
                        p.rect_stroke(
                            sr,
                            3.0,
                            Stroke::new(1.5, Color32::from_rgb(38, 37, 34)),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if resp.on_hover_text(stip).clicked() {
                        cfg.color_idx = si;
                    }
                }
                // 「…」 그 밖의 색을 직접 고른다 (KaShellGridControls.cpp:101-111)
                if ui
                    .small_button("…")
                    .on_hover_text("그 밖의 색을 직접 고릅니다")
                    .clicked()
                {
                    cfg.color_idx = (cfg.color_idx + 1) % SHELL_GRID_SWATCHES.len();
                }
            });
        });
    }

    // 1) ☐ 좌표격자 체크박스 (KaShellGridControls.cpp:49-51, 124)
    let chk_r = Rect::from_min_size(
        Pos2::new(bar_r.right() - 88.0, bar_r.bottom() - 25.0),
        Vec2::new(82.0, 22.0),
    );
    let chk_lbl = if enabled {
        "☑ 좌표격자"
    } else {
        "☐ 좌표격자"
    };
    p.text(
        chk_r.center(),
        egui::Align2::CENTER_CENTER,
        chk_lbl,
        FontId::proportional(11.5),
        Color32::from_rgb(28, 25, 23),
    );
    if ui
        .interact(chk_r, ui.id().with("map_grid_toggle"), egui::Sense::click())
        .on_hover_text("지도에 좌표 격자를 켭니다. 켠 뒤 미터·경위도 격자를 고릅니다.")
        .clicked()
    {
        self.home_map.grid_enabled = !self.home_map.grid_enabled;
    }

    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("ka_shell_grid_cfg"), cfg);
    });
    }
}
