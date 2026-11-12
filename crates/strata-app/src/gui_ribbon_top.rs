//! 상단 띠 한 프레임 — 리본 그룹·앱바·보조툴바·탭바를 순서대로 그린다.
//!
//! 원래 `render_cpp_top_bars` 한 함수(710줄)이던 것을 연속 구간으로 나눴다.
//! 본문은 그대로이고, 구간끼리 넘기는 값은 `RibbonFrame` 하나로 모은다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};
use crate::ribbon::SubToolsMode;

use super::gui_ribbon_consts::{
    ribbon_chip_base_width, ribbon_group_layout, UX_RIBBON_GROUPS, UX_RIBBON_HEIGHT,
    RIBBON_CHIP_SCALE,
};

/// 상단 띠가 쓰는 값 묶음 — 원래 `render_cpp_top_bars` 의 지역변수들이었다.
#[derive(Clone)]
pub(crate) struct RibbonFrame {
    pub(crate) ribbon_rect: Rect,
    pub(crate) h_scale: f32,
    pub(crate) extra_h: f32,
    pub(crate) group_xs: [f32; 7],
    pub(crate) group_widths: [f32; 7],
    pub(crate) is_home: bool,
    pub(crate) dirty_save: bool,
}

impl StrataAppState {
    /// C++ `MainWindowRibbon.cpp` + `KaBeginnerRibbon.cpp` 1:1 (Images 1, 2, 4, 9, 10, 11, 12, 15)
    pub(crate) fn render_cpp_top_bars(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let is_home = self.active_tab == MainTab::Home;
        let dirty_save = !is_home && self.is_dirty;
        let [pr, pg, pb, _] = self.theme.chrome;
        ui.spacing_mut().item_spacing.y = 0.0;

        let avail_for_ribbon = (ui.available_width() - 266.0).max(640.0);
        let h_scale = (avail_for_ribbon / 1328.0).clamp(0.72, 1.38);
        let scaled_tile = (39.0 * h_scale).clamp(28.0, 56.0);
        let extra_h = (scaled_tile - 39.0).max(0.0);
        let ribbon_h = UX_RIBBON_HEIGHT + extra_h;

        let (ribbon_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), ribbon_h),
            egui::Sense::hover(),
        );
        {
            let rp = ui.painter_at(ribbon_rect);
            rp.rect_filled(ribbon_rect, 0.0, Color32::from_rgb(pr, pg, pb));
            rp.line_segment(
                [
                    Pos2::new(ribbon_rect.left(), ribbon_rect.bottom() - 1.0),
                    Pos2::new(ribbon_rect.right(), ribbon_rect.bottom() - 1.0),
                ],
                Stroke::new(1.0, Color32::from_rgb(218, 215, 205)),
            );
        }

        // KA_HGIS_DEBUG_RIBBON=1 이면 첫 프레임의 실제 배치 값을 찍는다.
        if std::env::var("KA_HGIS_DEBUG_RIBBON").is_ok() {
            let full = ui.ctx().screen_rect();
            eprintln!("[RIBBON] screen={:.1}x{:.1} ppp={:.4} avail={:.1} h_scale={:.4} ribbon_rect.l={:.1} w={:.1}",
                full.width(), full.height(), ui.ctx().pixels_per_point(),
                ui.available_width(), h_scale, ribbon_rect.left(), ribbon_rect.width());
        }
        let group_layout = ribbon_group_layout();
        let frame = RibbonFrame {
            ribbon_rect,
            h_scale,
            extra_h,
            group_xs: group_layout.0,
            group_widths: group_layout.1,
            is_home,
            dirty_save,
        };

        self.paint_ribbon_groups(ui, &frame, ribbon_h);
        self.paint_ribbon_app_bar(ui, &frame);
        self.paint_ribbon_sub_toolbar(ctx, ui);
        self.paint_ribbon_tab_bar(ctx, ui);
        self.render_region_locator_popup(ctx);
    }

    /// 7그룹(조사·그리기·편집·보기·분석·자료·도움)과 각 단추 칩.
    fn paint_ribbon_groups(&mut self, ui: &mut egui::Ui, f: &RibbonFrame, ribbon_h: f32) {
        let ribbon_rect = f.ribbon_rect;
        let h_scale = f.h_scale;
        let extra_h = f.extra_h;
        let group_xs = f.group_xs;
        let group_widths = f.group_widths;
        let is_home = f.is_home;
        let dirty_save = f.dirty_save;

        let groups: &[(&str, &[&str])] = &UX_RIBBON_GROUPS;

        let chip_base_w = |g_idx: usize, label: &str| -> f32 {
            ribbon_chip_base_width(g_idx, label) * RIBBON_CHIP_SCALE
        };

        for (g_idx, (g_title, items)) in groups.iter().enumerate() {
            let gx = ribbon_rect.left() + group_xs[g_idx] * h_scale;
            let gw = group_widths[g_idx] * h_scale;
            let g_rect = Rect::from_min_size(
                Pos2::new(gx, ribbon_rect.top() + 1.0),
                Vec2::new(gw, ribbon_h - 2.0),
            );
            let gp = ui.painter_at(ribbon_rect);
            if g_idx > 0 {
                gp.line_segment(
                    [
                        Pos2::new(gx, g_rect.top() + 3.0),
                        Pos2::new(gx, g_rect.bottom() - 3.0),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(218, 215, 205)),
                );
            }
            gp.text(
                Pos2::new(g_rect.left() + 6.0, g_rect.top() + 9.0),
                egui::Align2::LEFT_CENTER,
                *g_title,
                egui::FontId::proportional(11.5),
                Color32::from_rgb(94, 93, 89),
            );
            let mut bx = g_rect.left() + 2.0 * h_scale;
            for &label in items.iter() {
                let cw = chip_base_w(g_idx, label) * h_scale;
                let btn_rect = Rect::from_min_size(
                    Pos2::new(bx, g_rect.top() + 18.0),
                    Vec2::new(cw, 69.0 + extra_h),
                );
                if std::env::var("KA_HGIS_DEBUG_RIBBON").is_ok() {
                    eprintln!("[CHIP] {:<10} bx={:7.1} cw={:6.1} rect=({:.1}..{:.1})",
                        label, bx, cw, btn_rect.left(), btn_rect.right());
                }
                bx += cw;
                // MainWindowRibbon.cpp:572 / MainWindow.cpp:1086 — GeoTIFF 는 「지도」 탭에서만
                // 켜진다. 다른 탭(홈·도면·고지형·입체지형)에서는 흐리게 그린다.
                let muted = (is_home && *g_title == "기록")
                    || (label == "GeoTIFF" && self.active_tab != MainTab::Map);
                let highlight_save = label == "저장" && dirty_save;
                let dark_active = (label == "선택" && self.active_tool == MapToolKind::Select)
                    || (label == "측거" && self.active_tool == MapToolKind::Measure)
                    || (label == "그리기"
                        && (self.active_tool == MapToolKind::Measure
                            || (self.ribbon.sub_toolbar_visible
                                && self.ribbon.sub_mode == SubToolsMode::Draw)))
                    || (label == "버퍼"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Buffer)
                    || (label == "정합"
                        && self.ribbon.sub_toolbar_visible
                        && self.ribbon.sub_mode == SubToolsMode::Align);
                let warm_active = label == "DEM" && !is_home;
                let mut chip_ui = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
                let (chip_clicked, chip_secondary) = self.draw_ribbon_chip(
                    &mut chip_ui,
                    label,
                    muted,
                    highlight_save,
                    dark_active,
                    warm_active,
                );
                if chip_clicked {
                    if label == "옛 지도" || label == "더보기" {
                        self.open_ribbon_context_menu(label, btn_rect);
                    } else {
                        self.handle_ribbon_click(label);
                    }
                }
                // KaPaleoMenu::install 등 우클릭 팝업
                if chip_secondary {
                    self.open_ribbon_context_menu(label, btn_rect);
                }
            }
        }
    }
}
