//! Studio tab geometry, target/cpp-ref/src/app/KaSurveyBadge.cpp:32-62; owner monitor-layout correction.
use crate::app_state::{MainTab, StrataAppState};
use crate::ribbon_icons::paint_ribbon_icon;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
impl StrataAppState {
    pub(super) fn paint_workspace_tab_buttons(&mut self, ui: &mut egui::Ui, tabbar_h: f32, compact: bool) {
        ui.spacing_mut().item_spacing.x = if compact { 6.0 } else { 16.0 };
        for (tab, ico, txt) in [(MainTab::Home, "유산", "홈"), (MainTab::Map, "옛 지도", "지도")] {
            let sel = self.active_tab == tab;
            let (tr, resp) = ui.allocate_exact_size(Vec2::new(if compact { 52.0 } else { 64.0 }, tabbar_h - 4.0), egui::Sense::click());
            let tp = ui.painter_at(tr);
            paint_ribbon_icon(&tp, Rect::from_center_size(Pos2::new(tr.left() + 12.0, tr.center().y), Vec2::new(26.0, 26.0)), ico, Color32::from_rgb(45, 44, 40));
            tp.text(Pos2::new(tr.left() + 28.0, tr.center().y), egui::Align2::LEFT_CENTER, txt, egui::FontId::proportional(13.0), Color32::from_rgb(30, 30, 28));
            if sel {
                tp.line_segment([Pos2::new(tr.left(), tr.bottom()), Pos2::new(tr.right(), tr.bottom())], Stroke::new(2.5, Color32::from_rgb(35, 35, 32)));
            }
            if resp.clicked() {
                self.active_tab = tab;
            }
        }
        let open_tabs = self.open_studio_tabs.clone();
        for (extra, ico, lbl) in
            [(MainTab::Drawing, "도면", "도면"), (MainTab::Print, "인쇄", "인쇄"), (MainTab::Paleo, "고지형", "고지형"), (MainTab::Terrain3d, "검수·제출", "입체지형")]
        {
            if open_tabs.contains(&extra) || self.active_tab == extra {
                let sel = self.active_tab == extra;
                let width = if compact {
                    match extra {
                        MainTab::Drawing | MainTab::Print => 78.0,
                        MainTab::Paleo => 92.0,
                        _ => 106.0,
                    }
                } else {
                    106.0
                };
                let (tr, resp) = ui.allocate_exact_size(Vec2::new(width, tabbar_h - 4.0), egui::Sense::click());
                let tp = ui.painter_at(tr);
                paint_ribbon_icon(&tp, Rect::from_center_size(Pos2::new(tr.left() + 12.0, tr.center().y), Vec2::new(22.0, 22.0)), ico, Color32::from_rgb(35, 35, 32));
                tp.text(Pos2::new(tr.left() + 28.0, tr.center().y), egui::Align2::LEFT_CENTER, lbl, egui::FontId::proportional(13.0), Color32::from_rgb(30, 30, 28));
                let close_rect = Rect::from_min_max(Pos2::new(tr.right() - 18.0, tr.top()), tr.max);
                tp.text(close_rect.center(), egui::Align2::CENTER_CENTER, "×", egui::FontId::proportional(13.5), Color32::from_rgb(90, 89, 84));
                if sel {
                    tp.line_segment([Pos2::new(tr.left(), tr.bottom()), Pos2::new(tr.right(), tr.bottom())], Stroke::new(2.5, Color32::from_rgb(35, 35, 32)));
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
    }
}
