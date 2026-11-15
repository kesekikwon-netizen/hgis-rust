//! 용지 위 팝업 — 지도 조정 안내 띠, 좌표점 찍기, 도면 목록.
//!
//! C++ `KaDrawingStudio.cpp:822-838`, `KaDrawingCoordCallout.cpp`,
//! `KaDrawingSheetSet.cpp:135-160`.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// 지도 조정 안내 띠, 좌표점 콜아웃(A, B, C …), 도면 목록 팝업.
    pub(crate) fn paint_sheet_popups(&mut self, ui: &mut egui::Ui, f: &SheetFrame) {
        let rect = f.rect;
        let p = f.p.clone();
        // KaDrawingStudio.cpp:822-838 m_adjustBar — 지도 조정 중 상단 안내 띠
        if self.drawing.map_adjust_active {
            let bar_w = 540.0_f32.min(rect.width() - 24.0);
            let bar_r = Rect::from_min_size(
                Pos2::new(rect.center().x - bar_w * 0.5, rect.top() + 8.0),
                Vec2::new(bar_w, 30.0),
            );
            p.rect_filled(bar_r, 7.0, Color32::from_rgba_unmultiplied(255, 253, 249, 244));
            p.rect_stroke(
                bar_r,
                7.0,
                Stroke::new(1.0, Color32::from_rgb(0xC5, 0xBF, 0xB2)),
                egui::StrokeKind::Middle,
            );
            p.text(
                Pos2::new(bar_r.left() + 10.0, bar_r.center().y),
                egui::Align2::LEFT_CENTER,
                "지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」.",
                FontId::proportional(11.5),
                Color32::from_rgb(28, 25, 23),
            );
            let done_r = Rect::from_min_size(
                Pos2::new(bar_r.right() - 62.0, bar_r.top() + 4.0),
                Vec2::new(56.0, 22.0),
            );
            if ui
                .interact(done_r, ui.id().with("drawing_adjust_done"), egui::Sense::click())
                .on_hover_text("지도 조정을 끝냅니다.")
                .clicked()
            {
                self.toggle_sheet_map_adjust();
            }
            p.text(
                done_r.center(),
                egui::Align2::CENTER_CENTER,
                "조정끝",
                FontId::proportional(11.0),
                Color32::from_rgb(28, 25, 23),
            );
            // KaDrawingStudio.cpp:835, :838 — the bar's own 「레이어를 가운데」 (centerSurveyInMap).
            let centre_r = Rect::from_min_size(Pos2::new(done_r.left() - 104.0, done_r.top()), Vec2::new(98.0, 22.0));
            if ui.interact(centre_r, ui.id().with("drawing_adjust_centre"), egui::Sense::click()).clicked() {
                self.center_layer_in_sheet();
            }
            p.rect_stroke(centre_r, 4.0, Stroke::new(1.0, Color32::from_rgb(0xC5, 0xBF, 0xB2)), egui::StrokeKind::Middle);
            p.rect_stroke(done_r, 4.0, Stroke::new(1.0, Color32::from_rgb(0xC5, 0xBF, 0xB2)), egui::StrokeKind::Middle);
            p.text(centre_r.center(), egui::Align2::CENTER_CENTER, "레이어를 가운데", FontId::proportional(11.0), Color32::from_rgb(28, 25, 23));
        }

        // KaDrawingCoordCallout.cpp — 좌표점: clicks while points are placed (gui_tab_drawing_coords.rs).
        self.handle_coord_point_clicks(ui, f);

        // KaDrawingSheetSet.cpp:131-162 — 도면 목록 (gui_tab_drawing_sheets.rs).
        if self.drawing.sheet_set_menu_open {
            self.render_sheet_set_menu(ui.ctx(), Pos2::new(rect.right() - 316.0, rect.bottom() - 60.0));
        }
        self.render_sheet_ask(ui.ctx());
    }
}
