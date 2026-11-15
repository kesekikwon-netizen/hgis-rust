//! The scale box of the sheet — C++ `m_scaleSpin` with `keyboardTracking` off and `applyOnScreenScale`
//! (KaDrawingStudio.cpp:1061-1105, 3121-3137): what is typed waits for 「적용」, Enter or leaving the box; a
//! preset number applies at once.

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// What the box shows: the number typed against the sheet's present scale, else that scale.
    pub(crate) fn sheet_scale_entry(&self) -> u32 {
        let now = self.drawing.sheet.scale_denominator;
        self.drawing.scale_entry.filter(|(_, against)| *against == now).map_or(now, |(typed, _)| typed)
    }

    /// A number typed or dragged in the box: the sheet keeps its scale.
    pub(crate) fn edit_sheet_scale(&mut self, typed: u32) {
        self.drawing.scale_entry = Some((typed, self.drawing.sheet.scale_denominator));
    }

    /// 「적용」, Enter or leaving the box.
    pub(crate) fn apply_sheet_scale_entry(&mut self) {
        self.pick_sheet_scale(self.sheet_scale_entry());
    }

    /// The sheet takes `wanted` (10 at least), and the status line says so.
    pub(crate) fn pick_sheet_scale(&mut self, wanted: u32) {
        let wanted = wanted.max(10);
        self.drawing.sheet.scale_denominator = wanted;
        self.drawing.scale_entry = None;
        self.show_status_message(format!("축척을 1 : {wanted} 로 맞췄습니다."));
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_scale_tests.rs"]
mod tests;
