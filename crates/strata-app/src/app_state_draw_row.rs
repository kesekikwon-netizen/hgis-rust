//! 그리기 보조 도구줄 열기·닫기 — C++ `showSubToolsDraw`(MainWindowRibbon.cpp:841-952)·`hideSubTools`(:769-781).
//! 줄을 열어도 도구는 그대로(이동)이고, 그릴 대상을 누를 때 그리기가 시작된다.

use crate::app_state::{MainTab, StrataAppState};
use crate::ribbon::SubToolsMode;

/// MainWindowRibbon.cpp:950-951.
pub(crate) const DRAW_ROW_OPEN_TEXT: &str =
    "그릴 도구를 고르세요 — 조사구역·유구·유구 선·유물 가운데 하나를 누르면 그리기가 시작됩니다.";

impl StrataAppState {
    /// 그리기 줄이 보이는가.
    pub(crate) fn draw_row_open(&self) -> bool {
        self.ribbon.sub_toolbar_visible && self.ribbon.sub_mode == SubToolsMode::Draw
    }

    /// 리본 「그리기」·Ctrl+D — 닫혀 있으면 열고(지도 탭으로), 열려 있으면 닫는다.
    pub(crate) fn toggle_draw_sub_tools(&mut self) {
        if self.draw_row_open() {
            self.hide_sub_tools();
            return;
        }
        self.active_tab = MainTab::Map;
        self.ribbon.sub_toolbar_visible = true;
        self.ribbon.sub_mode = SubToolsMode::Draw;
        self.show_status_message(DRAW_ROW_OPEN_TEXT);
    }

    // Keep sketch while a capture tool (DrawPolygon or DrawLine) stays active; drop on any other tool
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:245-258).
    pub(crate) fn drop_sketch_when_tool_changed(&mut self) {
        if !Self::is_capture_tool(self.active_tool) {
            self.draw_sketch.points.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use strata_canvas2d::map_tools::MapToolKind;

    use super::*;

    /// 「그리기」를 누르면 줄이 열리고 도구는 이동 그대로, C++ 안내 글. 다시 누르면 닫히고 그리던 점이 지워진다.
    #[test]
    fn draw_button_opens_the_row_without_starting_a_tool_and_closes_it_again() {
        let mut app = StrataAppState::new_startup();
        app.status_bar_hint = "줄자: 남아 있던 안내".to_owned();
        app.handle_ribbon_click("그리기");
        assert!(app.draw_row_open());
        assert_eq!(app.active_tab, MainTab::Map);
        assert_eq!(app.active_tool, MapToolKind::Pan);
        assert_eq!(app.status_left_message(), DRAW_ROW_OPEN_TEXT);

        app.active_tool = MapToolKind::DrawPolygon;
        app.draw_sketch.points.push([1.0, 2.0]);
        app.handle_ribbon_click("그리기");
        assert!(!app.draw_row_open());
        assert_eq!(app.active_tool, MapToolKind::Pan);
        assert!(app.draw_sketch.points.is_empty());
        assert_eq!(app.status_left_message(), DRAW_ROW_OPEN_TEXT, "hideSubTools shows no message");
    }

    /// 다른 도구(도형선택)로 바꾸면 그리던 점이 없어지고, 버퍼 줄을 열거나 탭만 바꾸면 남는다(C++ 과 같다).
    #[test]
    fn the_sketch_lives_only_while_the_polygon_tool_is_on() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("그리기");
        app.active_tool = MapToolKind::DrawPolygon;
        app.draw_sketch.points = vec![[1.0, 2.0], [3.0, 4.0]];
        app.handle_ribbon_click("버퍼");
        app.active_tab = MainTab::Home;
        app.drop_sketch_when_tool_changed();
        assert_eq!(app.draw_sketch.points.len(), 2, "buffer row and tab switch keep the sketch");
        app.active_tab = MainTab::Map;
        app.handle_ribbon_click("선택");
        app.drop_sketch_when_tool_changed();
        assert!(app.draw_sketch.points.is_empty(), "another map tool drops it");
    }

    // Line sketch survives while DrawLine stays active across tab/row switches and drops on another tool
    // (target/cpp-ref/src/app/KaCaptureMapTool.cpp:245-258).
    #[test]
    fn the_sketch_lives_while_the_line_tool_is_on() {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click("그리기");
        app.active_tool = MapToolKind::DrawLine;
        app.draw_sketch.points = vec![[1.0, 2.0], [3.0, 4.0]];
        app.handle_ribbon_click("버퍼");
        app.active_tab = MainTab::Home;
        app.drop_sketch_when_tool_changed();
        assert_eq!(app.draw_sketch.points.len(), 2, "buffer row and tab switch keep the line sketch");
        app.active_tab = MainTab::Map;
        app.handle_ribbon_click("선택");
        app.drop_sketch_when_tool_changed();
        assert!(app.draw_sketch.points.is_empty(), "another map tool drops it");
    }
}
