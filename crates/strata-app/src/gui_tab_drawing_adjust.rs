//! 「지도 조정」 and 「화면 맞춤」 of a sheet — C++ `KaLayoutMapAdjustTool` (`KaDrawingTools.cpp:55-150`),
//! `beginActivateMap` · `endActivateMap` · `fitToMapCanvas` (`KaDrawingStudio.cpp:3194-3210`, `:3588-3607`,
//! `:3623-3635`).

use eframe::egui::{self, Rect};

use crate::app_state::StrataAppState;

/// No scale box of the studio goes below 1:10 (`displayScale`, KaDrawingStudio.cpp:2879-2883), and the wheel
/// zooms in only above it (`KaDrawingTools.cpp:139`). The table of standard scales ends at 1:500,000.
const CLOSEST: f64 = 10.0;
const WIDEST: f64 = 500_000.0;

impl StrataAppState {
    /// The map frame under the pointer while 「지도 조정」 is on: a drag moves the map under the frame, the
    /// wheel zooms around the pointer by the steps of the map (a fifth to the notch, a third of that with
    /// Ctrl). `map_rect` is the frame on the screen, which shows the paper at `px_per_mm`.
    pub(crate) fn adjust_sheet_map(&mut self, ui: &egui::Ui, map_rect: Rect, px_per_mm: f32) {
        if !self.drawing.map_adjust_active || self.drawing.rail_pointer_owns {
            return;
        }
        let response = ui.interact(map_rect, ui.id().with("drawing_map_adjust"), egui::Sense::click_and_drag());
        if response.hovered() {
            // Qt::OpenHandCursor, closed while the map is held (`KaDrawingTools.cpp:49`, `:70`).
            ui.ctx().set_cursor_icon(if response.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        let scale = f64::from(self.drawing.sheet.scale_denominator.max(1));
        let metres_per_px = scale / 1000.0 / f64::from(px_per_mm);
        let mut centre = self.sheet_map_center();
        if response.dragged_by(egui::PointerButton::Primary) {
            let moved = response.drag_delta();
            centre = [centre[0] - f64::from(moved.x) * metres_per_px, centre[1] + f64::from(moved.y) * metres_per_px];
            self.drawing.sheet_center = Some(centre);
        }
        let Some(pointer) = response.hover_pos() else { return };
        let Some(wheel) = crate::map_wheel_zoom::wheel_mpp_factor_of_frame(ui) else { return };
        let zoomed = (scale * wheel).clamp(CLOSEST, WIDEST).round();
        if zoomed == scale {
            return;
        }
        // The ground under the pointer stays under it (`LayoutService::zoomExtentAtAnchor`).
        let middle = map_rect.center();
        let ground = [centre[0] + f64::from(pointer.x - middle.x) * metres_per_px, centre[1] - f64::from(pointer.y - middle.y) * metres_per_px];
        let ratio = zoomed / scale;
        self.drawing.sheet_center = Some([ground[0] + (centre[0] - ground[0]) * ratio, ground[1] + (centre[1] - ground[1]) * ratio]);
        self.drawing.sheet.scale_denominator = zoomed as u32;
    }

    /// The button 「지도 조정」, which reads 「조정 끝」 while the mode is on (`toggleMapAdjust`).
    pub(crate) fn toggle_sheet_map_adjust(&mut self) {
        if self.drawing.map_adjust_active {
            self.end_sheet_map_adjust();
            self.show_status_message("지도 조정을 끝냈습니다.");
        } else {
            // One tool of the sheet at a time (`m_view->setTool`).
            self.drawing.coord_callout_mode = false;
            self.drawing.map_adjust_active = true;
            self.show_status_message("지도 조정: 드래그로 이동, 휠로 확대. 끝나면 「지도조정끝」.");
        }
    }

    /// `endActivateMap`: after an adjustment the scale becomes the next standard scale that still holds what
    /// the frame shows (`snapMapScaleToNice`, KaDrawingStudio.cpp:2830-2854).
    pub(crate) fn end_sheet_map_adjust(&mut self) {
        if std::mem::take(&mut self.drawing.map_adjust_active) {
            let nice = strata_layout::standard_scales::snap_up(f64::from(self.drawing.sheet.scale_denominator));
            if nice > 0 {
                self.drawing.sheet.scale_denominator = nice;
            }
        }
    }

    /// 「화면 맞춤」: the adjusting ends and the sheet takes the middle and the scale of the map view
    /// (`fitToMapCanvas`; `applyCanvasViewToLayoutMap`, LayoutService.cpp:968-978).
    pub(crate) fn fit_sheet_to_map_view(&mut self) {
        self.end_sheet_map_adjust();
        let view = &self.home_map.viewport;
        let scale = (view.meters_per_pixel * crate::status_bar_text::POINTS_PER_METER).round().max(CLOSEST);
        self.drawing.sheet_center = Some(view.center_xy);
        self.drawing.sheet.scale_denominator = scale as u32;
        self.show_status_message(format!("지도 화면과 같은 범위·축척으로 맞췄습니다. 축척 1 : {scale:.0}"));
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_adjust_tests.rs"]
mod tests;
