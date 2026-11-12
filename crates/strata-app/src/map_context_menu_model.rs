//! 지도 우클릭 메뉴의 항목 — C++ `MainWindow::populateMapContextMenu`(`MainWindowContextMenus.cpp:112-161`).
//!
//! 화면 그리기(`gui_tab_map_context.rs`)와 나눠 항목·흐림·툴팁만 만든다. C++ `addMenuAction`(:70-79)
//! 처럼 못 쓰는 이유가 있으면 흐리게 하고 그 이유를 툴팁으로, 쓸 수 있으면 도움말(좌표 복사의 글)을 툴팁으로.
//! 「이 도형 기록 입력」(:124-133)은 도형 속성 창(지시 CM1b)이 생길 때까지 넣지 않는다.

/// 메뉴 한 줄.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapMenuAction {
    /// C++ objectName(`map.previous` 등).
    pub(crate) id: &'static str,
    pub(crate) text: &'static str,
    pub(crate) enabled: bool,
    pub(crate) tooltip: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapMenuEntry {
    Action(MapMenuAction),
    Submenu { id: &'static str, title: &'static str, items: Vec<MapMenuAction> },
    Separator,
}

/// 메뉴를 만들 때 보는 상태.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MapMenuInput<'a> {
    /// `m_surveyPath` 가 있는가.
    pub(crate) survey_open: bool,
    /// `m_isOpeningSurvey` — 조사 파일을 여는·만드는 중.
    pub(crate) busy: bool,
    /// 범례의 현재 레이어가 있고 쓸 수 있는가.
    pub(crate) current_layer: bool,
    pub(crate) can_previous: bool,
    pub(crate) can_next: bool,
    /// 우클릭한 자리(작업 좌표계).
    pub(crate) click_xy: [f64; 2],
    /// 작업 좌표계 `destinationCrs().authid()`.
    pub(crate) crs_auth: &'a str,
}

fn action(id: &'static str, text: &'static str, unavailable: &str, help: &str) -> MapMenuAction {
    let enabled = unavailable.is_empty();
    MapMenuAction { id, text, enabled, tooltip: if enabled { help } else { unavailable }.to_owned() }
}

/// 「이 위치 좌표 복사」의 글 — `"%1, %2 (%3)"` = x, y(소수 셋째 자리), 좌표계(:153-156).
pub(crate) fn coordinate_text(xy: [f64; 2], crs_auth: &str) -> String {
    format!("{:.3}, {:.3} ({crs_auth})", xy[0], xy[1])
}

/// 메뉴 항목 차례.
pub(crate) fn map_menu_entries(m: &MapMenuInput<'_>) -> Vec<MapMenuEntry> {
    let survey_reason = if m.busy {
        "조사 파일 처리가 끝난 뒤 다시 실행하세요."
    } else if !m.survey_open {
        "먼저 새 조사를 만들거나 조사 파일을 열어 주세요."
    } else {
        ""
    };
    let mut out = Vec::new();
    if m.survey_open {
        out.push(MapMenuEntry::Submenu {
            id: "map.draw",
            title: "조사 도형 그리기",
            items: vec![
                action("map.drawArea", "조사구역", survey_reason, ""),
                action("map.drawFeature", "유구 면", survey_reason, ""),
                action("map.drawLine", "유구 선", survey_reason, ""),
                action("map.drawSection", "단면선 그리기", survey_reason, ""),
            ],
        });
        out.push(MapMenuEntry::Separator);
    }
    let prev = if m.can_previous { "" } else { "이전에 보던 지도 범위가 없습니다." };
    let next = if m.can_next { "" } else { "이전 범위로 돌아간 뒤 사용할 수 있습니다." };
    let layer = if m.current_layer { "" } else { "왼쪽 목록에서 레이어를 선택하세요." };
    out.push(MapMenuEntry::Action(action("map.previous", "이전 지도 범위", prev, "")));
    out.push(MapMenuEntry::Action(action("map.next", "다음 지도 범위", next, "")));
    out.push(MapMenuEntry::Action(action("map.layerZoom", "선택한 레이어로 이동", layer, "")));
    out.push(MapMenuEntry::Action(action("map.full", "전체 지도 보기", "", "")));
    out.push(MapMenuEntry::Action(action("map.refresh", "지도 새로고침", "", "")));
    out.push(MapMenuEntry::Separator);
    let coords = coordinate_text(m.click_xy, m.crs_auth);
    out.push(MapMenuEntry::Action(action("map.copyCoordinates", "이 위치 좌표 복사", "", &coords)));
    out
}

#[cfg(test)]
#[path = "map_context_menu_model_tests.rs"]
mod tests;
