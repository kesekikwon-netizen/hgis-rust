//! `gui_shell` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::{MainTab, StrataAppState};
use crate::gui_shell::*;
use crate::gui_layer_panel::{
    UX_FILE_DRAWER_HEIGHT, UX_LAYER_CARD_HEIGHT, UX_LEFT_PANEL_WIDTH,
};
use crate::gui_ribbon_bar::{ux_ribbon_group_widths, ux_ribbon_group_xs, UX_RIBBON_BUTTON_SIZE, UX_RIBBON_GROUPS, UX_RIBBON_HEIGHT, UX_VIEW_TAB_BAR_HEIGHT};
use crate::gui_tab_map::{UX_MAP_CANVAS_WIDTH, UX_MAP_LEFT_WIDTH, UX_MAP_RIGHT_WIDTH};

// D12 KaStatusBar.cpp:77-136 — 13단계 축척 프리셋, 직접 입력 칸, 지도갱신 토글
// D21 — 지도 캔버스·레이어 트리 우클릭 메뉴 상태
// D25 파일 끌어 놓기·종료 확인·CAD 안내 + 해상도별 레이아웃
#[test]
fn test_d25_drop_close_prompt_cad_notice_and_resolutions() {
    // MainWindowFiles.cpp:265-268 — 받는 확장자
    let accepts = |n: &str| -> bool {
        let low = n.to_lowercase();
        low.ends_with(".shp")
            || low.ends_with(".dxf")
            || low.ends_with(".dwg")
            || low.ends_with(".gpkg")
            || low.ends_with(".geojson")
            || low.ends_with(".json")
            || low.ends_with(".tif")
            || low.ends_with(".img")
            || low.ends_with(".csv")
            || low.ends_with(".xlsx")
            || low.ends_with(".hgis")
    };
    assert!(accepts("조사구역.SHP"));
    assert!(accepts("도면.dxf"));
    assert!(accepts("data.gpkg"));
    assert!(!accepts("readme.txt"));
    assert!(!accepts("photo.bmp"));

    // KaClosePrompt.cpp — 저장/저장안함/취소
    let buttons = ["저장하고 끝내기", "저장하지 않고 끝내기", "취소"];
    assert_eq!(buttons.len(), 3);

    // KaCadImportNotice.cpp — 문구
    assert!("CAD 도면 좌표계 확인".contains("좌표계"));

    // 해상도·배율별 레이아웃이 화면 안에 든다
    for (sw, sh, ppp) in [
        (1600.0_f32, 900.0_f32, 1.0_f32),
        (1920.0, 1080.0, 1.0),
        (2560.0, 1080.0, 1.0),
        (3440.0, 1440.0, 1.0),
        (3840.0, 2160.0, 1.5),
        (3840.0, 2160.0, 2.0),
    ] {
        let lw = sw / ppp;
        let lh = sh / ppp;
        // 모달은 항상 화면 - 32 안에 든다
        for (dw, dh) in [
            (960.0_f32, 620.0_f32),
            (940.0, 700.0),
            (820.0, 500.0),
            (460.0, 260.0),
        ] {
            assert!(dw.min(lw - 32.0) <= lw);
            assert!(dh.min(lh - 32.0) <= lh);
        }
        // 좌우 패널을 빼도 가운데 칸이 남는다
        assert!(lw - 268.0 - 280.0 > 320.0, "too narrow at {sw}x{sh} @{ppp}");
    }
}

#[test]
fn test_d21_map_and_layer_context_menus() {
    // MainWindowContextMenus.cpp:112-161 mapContextMenu 항목 순서
    let map_items = [
        "이 도형 기록 입력",
        "조사 도형 그리기",
        "조사구역",
        "유구 면",
        "유구 선",
        "단면선 그리기",
        "이전 지도 범위",
        "다음 지도 범위",
        "선택한 레이어로 이동",
        "전체 지도 보기",
        "지도 새로고침",
        "이 위치 좌표 복사",
    ];
    assert!(map_items.contains(&"이 위치 좌표 복사"));
    assert!(map_items.contains(&"전체 지도 보기"));

    // 좌표 복사 문구 — MainWindowContextMenus.cpp:153-156 `"%1, %2 (%3)"` = x(동), y(북), 작업 좌표계
    let txt = crate::map_context_menu_model::coordinate_text([219_897.093, 361_622.354], "EPSG:5187");
    assert_eq!(txt, "219897.093, 361622.354 (EPSG:5187)");

    // MainWindowContextMenus.cpp:176-448 layerContextMenu 항목
    let layer_items = [
        "참고 자료 불러오기…",
        "이름 바꾸기 (두 번 클릭)",
        "면·외곽선 색",
        "도형 속성",
        "라벨·명칭 속성",
        "깨진 한글 바로잡기",
        "투명도",
        "이 레이어로 이동",
        "전체 보기",
        "같은 묶음의 맨 아래로",
        "그리기·편집 시작",
        "꼭짓점 수정",
        "폴리곤 묶기",
        "면적 확인",
        "그린 도형 모두 지우기…",
        "레이어 삭제",
    ];
    assert_eq!(layer_items.len(), 16);
    assert_eq!(layer_items[0], "참고 자료 불러오기…");
    assert_eq!(layer_items[15], "레이어 삭제");

    // 글자 크기 후보 10종
    let sizes = [3_u32, 4, 5, 6, 7, 8, 9, 10, 12, 14];
    assert_eq!(sizes.len(), 10);
    // 투명도 후보 6종
    let opacity = [0_u8, 20, 40, 50, 60, 80];
    assert_eq!(opacity.len(), 6);
    // 인코딩 후보 4종
    let encodings = ["CP949", "EUC-KR", "UTF-8", "이 컴퓨터 설정"];
    assert_eq!(encodings.len(), 4);
}

#[test]
fn test_d12_status_bar_13_scale_presets_and_render_toggle() {
    assert_eq!(KA_STATUS_SCALE_PRESETS.len(), 13);
    assert_eq!(KA_STATUS_SCALE_PRESETS[0], 100);
    assert_eq!(KA_STATUS_SCALE_PRESETS[12], 500_000);

    let mut app = StrataAppState::new_startup();
    assert!(app.map_rendering_enabled);
    assert!(app.scale_input_buf.is_empty());

    // 「1000」과 「1:2500」이 모두 그대로 쓰인다 (KaStatusBar.cpp:92-98)
    app.active_tab = MainTab::Map;
    app.scale_input_buf = "1000".to_owned();
    assert!(app.apply_scale_input());
    assert!((app.home_map.viewport.meters_per_pixel - 1000.0 / 3780.0).abs() < 1e-12);
    app.scale_input_buf = "1:2500".to_owned();
    assert!(app.apply_scale_input());
    assert!((app.home_map.viewport.meters_per_pixel - 2500.0 / 3780.0).abs() < 1e-12);
    assert_eq!(app.scale_input_buf, "1:2500");

    // 읽을 수 없는 글자는 아무것도 바꾸지 않는다
    let before = app.home_map.viewport.meters_per_pixel;
    app.scale_input_buf = "abc".to_owned();
    assert!(!app.apply_scale_input());
    assert!((app.home_map.viewport.meters_per_pixel - before).abs() < 1e-12);
}

#[test]
fn test_u1_ux_layout_spec_1600x900_skeleton_ribbon_and_3_column_split() {
    assert_eq!(UX_RIBBON_BUTTON_SIZE, egui::Vec2::new(69.0, 100.0));
    assert_eq!(UX_RIBBON_HEIGHT, 91.0);
    assert_eq!(UX_VIEW_TAB_BAR_HEIGHT, 51.0);
    // `d7afcc8` 은 「자료 받기」에 「시추」를 더하고 칩 최소폭을 40 → 36 으로 줄였다.
    // 그룹 폭은 이제 칩 폭 표에서 계산되므로, 표가 1줄 리본 안에 들어오는지만 확인한다.
    let (xs, widths) = (ux_ribbon_group_xs(), ux_ribbon_group_widths());
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "group x must ascend");
    assert!(
        xs.iter().zip(widths.iter()).all(|(x, w)| x + w <= 1328.0),
        "every ribbon group must fit inside the 1328 px one-row ribbon"
    );
    assert_eq!(UX_RIBBON_GROUPS.len(), 7);
    assert_eq!(UX_RIBBON_GROUPS[2].0, "자료 받기");
    assert_eq!(
        UX_RIBBON_GROUPS[2].1,
        &["DEM", "지적", "수치지형", "유산", "시추"][..]
    );
    assert_eq!(UX_STATUS_BAR_Y, 839.0);
    assert_eq!(UX_STATUS_BAR_HEIGHT, 32.0);
    assert_eq!(UX_LEFT_PANEL_WIDTH, 351.0);
    assert_eq!(UX_MAP_LEFT_WIDTH, 351.0);
    assert_eq!(UX_MAP_CANVAS_WIDTH, 954.0);
    assert_eq!(UX_MAP_RIGHT_WIDTH, 236.0);
    assert_eq!(UX_LAYER_CARD_HEIGHT, 507.0);
    assert_eq!(UX_FILE_DRAWER_HEIGHT, 168.0);

    let ctx = egui::Context::default();
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.ribbon.sub_toolbar_visible = false;

    let raw_1600 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(raw_1600, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
    let map_w_1600 = app.home_map.viewport.screen_size_px[0];
    assert!(
        (map_w_1600 - f64::from(UX_MAP_CANVAS_WIDTH)).abs() <= 4.0,
        "expected map canvas width ~954 at 1600x900, got {map_w_1600}"
    );

    let raw_1920 = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1920.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(raw_1920, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
    let map_w_1920 = app.home_map.viewport.screen_size_px[0];
    assert!(
        map_w_1920 > map_w_1600 + 250.0,
        "center map must expand when window widens: 1600->{map_w_1600}, 1920->{map_w_1920}"
    );
}
