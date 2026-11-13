//! 지도 우클릭 메뉴 항목이 C++ `populateMapContextMenu`(`MainWindowContextMenus.cpp:112-161`)와
//! 같은 차례·흐림·툴팁인지.

use super::*;

fn input(survey_open: bool) -> MapMenuInput<'static> {
    MapMenuInput {
        survey_open,
        shape_under_click: false,
        busy: false,
        current_layer: false,
        can_previous: false,
        can_next: false,
        click_xy: [219_897.093, 361_622.354],
        crs_auth: "EPSG:5187",
    }
}

/// 한 줄 요약: 하위 메뉴는 `>제목[항목…]`, 구분선은 `|`, 흐린 항목은 `~` 를 붙인다.
fn outline(entries: &[MapMenuEntry]) -> Vec<String> {
    let item = |a: &MapMenuAction| format!("{}{}", if a.enabled { "" } else { "~" }, a.text);
    entries
        .iter()
        .map(|e| match e {
            MapMenuEntry::Action(a) => item(a),
            MapMenuEntry::Separator => "|".to_owned(),
            MapMenuEntry::Submenu { title, items, .. } => {
                format!(">{title}[{}]", items.iter().map(item).collect::<Vec<_>>().join(","))
            }
        })
        .collect()
}

fn action<'a>(entries: &'a [MapMenuEntry], id: &str) -> &'a MapMenuAction {
    entries
        .iter()
        .find_map(|e| match e {
            MapMenuEntry::Action(a) if a.id == id => Some(a),
            _ => None,
        })
        .expect("menu action")
}

/// 조사 파일이 없으면 「조사 도형 그리기」 하위 메뉴와 그 뒤 구분선이 없다. 이력이 없으면
/// 이전·다음이 흐리고, 현재 레이어가 없으면 「선택한 레이어로 이동」이 흐리다.
#[test]
fn without_a_survey_the_menu_matches_cpp() {
    let e = map_menu_entries(&input(false));
    assert_eq!(
        outline(&e),
        ["~이전 지도 범위", "~다음 지도 범위", "~선택한 레이어로 이동", "전체 지도 보기", "지도 새로고침", "|", "이 위치 좌표 복사"]
    );
    assert_eq!(action(&e, "map.previous").tooltip, "이전에 보던 지도 범위가 없습니다.");
    assert_eq!(action(&e, "map.next").tooltip, "이전 범위로 돌아간 뒤 사용할 수 있습니다.");
    assert_eq!(action(&e, "map.layerZoom").tooltip, "왼쪽 목록에서 레이어를 선택하세요.");
    assert_eq!(action(&e, "map.full").tooltip, "");
    assert_eq!(action(&e, "map.refresh").tooltip, "");
    assert_eq!(action(&e, "map.copyCoordinates").tooltip, "219897.093, 361622.354 (EPSG:5187)");
}

/// 조사 파일이 열려 있으면 맨 위에 하위 메뉴 네 항목, 그 뒤 구분선. 이력·현재 레이어가 있으면 켜진다.
#[test]
fn with_a_survey_the_draw_submenu_comes_first() {
    let mut i = input(true);
    i.current_layer = true;
    i.can_previous = true;
    i.can_next = true;
    let e = map_menu_entries(&i);
    assert_eq!(
        outline(&e),
        [
            ">조사 도형 그리기[조사구역,유구 면,유구 선,단면선 그리기]",
            "|",
            "이전 지도 범위",
            "다음 지도 범위",
            "선택한 레이어로 이동",
            "전체 지도 보기",
            "지도 새로고침",
            "|",
            "이 위치 좌표 복사"
        ]
    );
    let MapMenuEntry::Submenu { id, items, .. } = &e[0] else { panic!("submenu first") };
    assert_eq!(*id, "map.draw");
    let ids: Vec<&str> = items.iter().map(|a| a.id).collect();
    assert_eq!(ids, ["map.drawArea", "map.drawFeature", "map.drawLine", "map.drawSection"]);
    assert!(items.iter().all(|a| a.tooltip.is_empty()));
    assert_eq!(action(&e, "map.previous").tooltip, "");
}

/// 조사 파일을 처리하는 중이면 그리기 네 항목이 흐리고 이유가 툴팁이다.
#[test]
fn busy_greys_the_draw_items_with_the_reason() {
    let mut i = input(true);
    i.busy = true;
    let e = map_menu_entries(&i);
    let MapMenuEntry::Submenu { items, .. } = &e[0] else { panic!("submenu first") };
    assert!(items.iter().all(|a| !a.enabled && a.tooltip == "조사 파일 처리가 끝난 뒤 다시 실행하세요."));
}

/// 「이 위치 좌표 복사」 — `"%1, %2 (%3)"` = x(동), y(북), 소수 셋째 자리, 작업 좌표계.
#[test]
fn copied_coordinates_are_x_then_y_with_the_work_crs() {
    assert_eq!(coordinate_text([219_897.093, 361_622.354], "EPSG:5187"), "219897.093, 361622.354 (EPSG:5187)");
    assert_eq!(coordinate_text([200_010.0, 450_010.5], "EPSG:5186"), "200010.000, 450010.500 (EPSG:5186)");
    assert_eq!(coordinate_text([219745.6, 360930.12345], "EPSG:5187"), "219745.600, 360930.123 (EPSG:5187)");
}

/// :120-133 — 우클릭 자리에 조사 도형이 있으면 「이 도형 기록 입력」이 맨 위, 처리 중이면 흐리고 C++ 이유.
#[test]
fn attributes_item_comes_first_over_a_survey_shape() {
    let mut m = input(true);
    assert!(!outline(&map_menu_entries(&m)).contains(&"이 도형 기록 입력".to_owned()));
    m.shape_under_click = true;
    let entries = map_menu_entries(&m);
    assert_eq!(outline(&entries)[0], "이 도형 기록 입력");
    let MapMenuEntry::Action(a) = &entries[0] else { panic!("action") };
    assert_eq!(a.id, "map.attributes");
    m.busy = true;
    let MapMenuEntry::Action(a) = &map_menu_entries(&m)[0] else { panic!("action") };
    assert!(!a.enabled);
    assert_eq!(a.tooltip, "조사 파일 처리가 끝난 뒤 다시 실행하세요.");
}
