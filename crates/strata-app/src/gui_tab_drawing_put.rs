//! Taking an item of a sheet off the paper and putting it back — C++ `handleDeleteKey` and `deleteSelectedItems`
//! (KaDrawingStudio.cpp:3512-3522, 3213-3257), the place buttons (:1606-1660, KaDrawingStudioSheets.cpp:35-69)
//! and `undoLastCoordCallout` (:1806-1821).

use crate::app_state::StrataAppState;
use crate::tab_drawing::SheetItem;

impl StrataAppState {
    /// Whether `item` is on the paper.
    pub(crate) fn sheet_item_on(&self, item: SheetItem) -> bool {
        match item {
            SheetItem::Legend => self.drawing.sheet.legend_visible,
            SheetItem::TitleBlock => self.drawing.sheet.title_block_visible,
            other => !self.drawing.removed_items.contains(&other),
        }
    }

    /// Delete or Backspace on the sheet: while coordinate points are placed the last one goes; else the
    /// selected item leaves the paper until its button puts it back.
    pub(crate) fn delete_sheet_selection(&mut self) {
        if self.drawing.coord_callout_mode {
            return self.undo_last_coord_callout();
        }
        let Some(item) = self.drawing.selected_item.take() else {
            return self.show_status_message("지울 항목을 먼저 선택하세요.");
        };
        match item {
            SheetItem::Legend => self.drawing.sheet.legend_visible = false,
            SheetItem::TitleBlock => self.drawing.sheet.title_block_visible = false,
            other => drop(self.drawing.removed_items.insert(other)),
        }
        self.show_status_message("선택한 항목 1개를 지웠습니다.");
    }

    /// The last coordinate point goes.
    pub(crate) fn undo_last_coord_callout(&mut self) {
        if self.drawing.coord_callouts.pop().is_none() {
            return self.show_status_message("지울 좌표점이 없습니다.");
        }
        match self.drawing.coord_callouts.len() {
            0 => self.show_status_message("좌표점을 모두 지웠습니다."),
            left => self.show_status_message(format!("마지막 좌표점을 지웠습니다. 남은 점 {left}개.")),
        }
    }

    /// The button of `item` puts it on the paper and selects it, ending the other tools of the sheet. An
    /// item that was taken off comes back at the sheet's own place; one still on the paper stays where it is.
    pub(crate) fn put_item_on_sheet(&mut self, item: SheetItem) {
        if !self.sheet_item_on(item) {
            self.drawing.item_offsets_mm.remove(&item);
        }
        match item {
            SheetItem::Legend => self.drawing.sheet.legend_visible = true,
            SheetItem::TitleBlock => self.drawing.sheet.title_block_visible = true,
            other => drop(self.drawing.removed_items.remove(&other)),
        }
        self.end_sheet_map_adjust();
        self.drawing.coord_callout_mode = false;
        self.drawing.selected_item = Some(item);
        let survey = &self.home_map.real_survey;
        let words = match item {
            SheetItem::Legend => "범례를 넣었습니다. 끌어 옮기세요.",
            SheetItem::TitleBlock if survey.survey_name.is_empty() && survey.site_name.is_empty() => "표제란을 넣었습니다. 조사구역에 조사명·유적명이 없어 ―로 두었습니다.",
            SheetItem::TitleBlock => "표제란을 넣었습니다. 끌어 옮기세요. 필요 없으면 Delete로 지웁니다.",
            SheetItem::NorthArrow => "방위표를 넣었습니다. 끌어 옮기세요.",
            SheetItem::ScaleBar => "축척자를 넣었습니다. 끌어 옮기세요.",
            SheetItem::ScaleText => "축척 글자를 넣었습니다. 끌어 옮기세요.",
            SheetItem::CrsLabel => "좌표계를 넣었습니다. 끌어 옮기세요.",
        };
        self.show_status_message(words);
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_put_tests.rs"]
mod tests;
