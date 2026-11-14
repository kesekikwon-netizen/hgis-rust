//! 켠 직후 지도 탭 = C++ 실물(docs/senior/shots/cpp-start-maptab.png) — 상태줄 글, 도구 이름, 좌표 칸,
//! 축척, 범례 빈 상태, 「전체 켜기」, 배경지도.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};

fn map_start() -> StrataAppState {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app
}

/// `MainWindow::updateNextActionStatus`(MainWindow.cpp:393-413) — 조사가 없으면 「먼저 「새 조사」…」.
#[test]
fn next_action_message_follows_cpp() {
    let mut app = map_start();
    assert_eq!(app.next_action_status(), "먼저 「새 조사」로 오늘 현장을 만드세요.");
    app.survey_path = Some(std::env::temp_dir().join("s1.gpkg"));
    assert_eq!(app.next_action_status(), "위성·지적이 없습니다. 더보기 → API 키 입력에서 VWorld 키를 확인하세요.");
    app.left_layers.apply_default_basemaps();
    assert_eq!(app.next_action_status(), "「그리기」로 구역을 그리세요.");
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::SurveyArea);
    assert_eq!(app.next_action_status(), "다 그렸으면 리본의 「도면」으로 종이에 옮기세요.");
    // 켠 직후 지도 탭의 상태줄 왼쪽은 그 글이다(다른 알림이 없을 때).
    let start = map_start();
    assert_eq!(start.status_left_message(), "먼저 「새 조사」로 오늘 현장을 만드세요.");
}

/// `currentToolLabel`(KaDrawSketchTools.cpp:173-212) — 이동·도형선택·줄자·그리기 · <대상>.
#[test]
fn tool_chip_names_the_current_tool() {
    let mut app = map_start();
    assert_eq!(app.tool_chip_label(), "이동");
    app.active_tool = MapToolKind::Select;
    assert_eq!(app.tool_chip_label(), "도형선택");
    app.active_tool = MapToolKind::Measure;
    assert_eq!(app.tool_chip_label(), "줄자");
    app.active_tool = MapToolKind::DrawPolygon;
    assert_eq!(app.tool_chip_label(), "그리기");
    app.edit_layer_key = Some("feature_poly".to_owned());
    assert_eq!(app.tool_chip_label(), "그리기 · 유구 면");
    app.draw_sketch.points = vec![[0.0, 0.0], [1.0, 1.0]];
    assert_eq!(app.tool_chip_label(), "그리기 · 유구 면 · 점 2");
    app.active_tab = MainTab::Home;
    assert_eq!(app.tool_chip_label(), "", "only on the map tab");
}

// Line tool chip counts draw_sketch vertices too (target/cpp-ref/src/app/KaDrawSketchTools.cpp:173-199).
#[test]
fn line_tool_chip_counts_draw_sketch_points() {
    let mut app = map_start();
    app.active_tool = MapToolKind::DrawLine;
    app.edit_layer_key = Some("feature_line".to_owned());
    assert_eq!(app.tool_chip_label(), "그리기 · 유구 선");
    app.draw_sketch.points = vec![[0.0, 0.0], [5.0, 0.0]];
    assert_eq!(app.tool_chip_label(), "그리기 · 유구 선 · 점 2");
    app.edit_layer_key = Some("section_line".to_owned());
    app.draw_sketch.points.push([5.0, 5.0]);
    assert_eq!(app.tool_chip_label(), "그리기 · 단면선 · 점 3");
}

/// `KaStatusBar::clearCoordinate`·`setCoordinate`(KaStatusBar.cpp:167-175) — 마우스가 지도에 오기 전엔 「X —    Y —」.
#[test]
fn xy_readout_waits_for_the_mouse() {
    let mut app = map_start();
    assert_eq!(app.xy_readout(), "X —    Y —");
    app.home_map.cursor_world_xy = [219_745.6, 360_930.5];
    app.home_map.cursor_seen = true;
    assert_eq!(app.xy_readout(), "X 219,745.600    Y 360,930.500");
    app.home_map.cursor_world_xy = [-1_234.5, 5.0];
    assert_eq!(app.xy_readout(), "X -1,234.500    Y 5.000");
}

/// 축척은 QGIS 처럼 96 dpi — 1:N 의 한 점은 N × 0.0254 / 96 m.
#[test]
fn scale_uses_ninety_six_dpi() {
    let mut app = map_start();
    app.home_map.viewport.meters_per_pixel = 3_803_713.0 * 0.0254 / 96.0;
    assert_eq!(app.map_scale_denominator(), 3_803_713);
    app.scale_input_buf = "1:2500".to_owned();
    assert!(app.apply_scale_input());
    assert!((app.home_map.viewport.meters_per_pixel - 2_500.0 * 0.0254 / 96.0).abs() < 1e-12);
}

/// 켠 직후: 배경지도 레이어가 없으니 위성을 칠하지 않는다(C++ 은 흰 지도). 범례는 빈 안내와 흐린 「전체 켜기」.
#[test]
fn start_map_has_no_basemap_and_an_empty_legend() {
    let mut app = map_start();
    assert_eq!(app.basemap_kind_to_paint(), None);
    assert!(!app.cadastral_layer_listed());
    assert_eq!(
        crate::gui_layer_panel::gui_layer_panel_row_spec::layer_panel_empty_hint(),
        "레이어가 없습니다.\n파일함에서 SHP·DXF·DWG를 끌어 넣거나\n위성·지적 배경을 올리세요."
    );
    assert_eq!(app.toggle_all_state(), ("전체 켜기", false));
    app.left_layers.apply_default_basemaps();
    assert_eq!(app.basemap_kind_to_paint(), Some(crate::basemap_tiles::TileKind::Satellite));
    assert!(app.cadastral_layer_listed());
    assert_eq!(app.toggle_all_state(), ("전체 끄기", true));
    app.toggle_all_layers();
    assert_eq!(app.toggle_all_state(), ("전체 켜기", true));
    assert_eq!(app.basemap_kind_to_paint(), None);
}
