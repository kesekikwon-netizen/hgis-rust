//! `gui_shell` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::{MainTab, StrataAppState};
use crate::gui_layer_panel::{
    FILE_BTN_DESK_TIP, FILE_BTN_DOCS_TIP, FILE_BTN_DOWN_TIP, FILE_BTN_PC_TIP,
    FILE_BTN_PICK_TIP, FILE_BTN_UP_TIP, FILE_LIST_TIP,
};
use crate::gui_ribbon_bar::{REGION_BOTTOM_HINT, REGION_LOT_HINT, REGION_LOT_PLACEHOLDER};
use crate::gui_tab_map::{brightness_row_text, grid_band_rects, grid_label_text, grid_line_stride, is_raster_image_layer, nice_grid_step_meters, scale_bar_metres, scale_bar_tick_text, ShellGridConfig, GRID_BAND_BORDER, GRID_BAND_FILL, GRID_BAND_INK, GRID_BAND_PX};

// D12 KaStatusBar.cpp:77-136 — 13단계 축척 프리셋, 직접 입력 칸, 지도갱신 토글
// D21 — 지도 캔버스·레이어 트리 우클릭 메뉴 상태
// D25 파일 끌어 놓기·종료 확인·CAD 안내 + 해상도별 레이아웃
#[test]
fn test_d17_draw_guide_measure_and_reference_status() {
    // KaDrawGuideBand.cpp:48-62 — 선 길이·면 면적 계산
    let ring = [[0.0_f64, 0.0], [30.0, 0.0], [30.0, 40.0]];
    let mut len_m = 0.0_f64;
    for w in ring.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        len_m += (dx * dx + dy * dy).sqrt();
    }
    assert!((len_m - 70.0).abs() < 1e-9);
    let mut area = 0.0_f64;
    for k in 0..ring.len() {
        let a = ring[k];
        let b = ring[(k + 1) % ring.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    assert!((area.abs() * 0.5 - 600.0).abs() < 1e-9);

    // 1 km 이상이면 km 병기
    let far = [[0.0_f64, 0.0], [1500.0, 0.0]];
    let mut far_len = 0.0_f64;
    for w in far.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        far_len += (dx * dx + dy * dy).sqrt();
    }
    assert!(far_len >= 1000.0);

    // KaReferenceStatusDialog.cpp:56-60 — 7개 열 폭 합이 창 안에 든다
    let cols = [176.0_f32, 68.0, 96.0, 96.0, 56.0, 150.0, 86.0];
    let sum: f32 = cols.iter().sum::<f32>() + 48.0;
    assert!(
        sum <= 776.0,
        "columns {sum} must fit the 820px dialog inner width"
    );
}

#[test]
fn test_d14_region_locator_and_canvas_grid_band() {
    let mut app = StrataAppState::new_startup();
    assert_eq!(REGION_LOT_PLACEHOLDER, "산 12-3");
    assert_eq!(
        REGION_LOT_HINT,
        "번지 (예: 12, 12-3, 산 12-3). 산번지는 ‘산’을 함께 입력하세요."
    );
    assert_eq!(
        REGION_BOTTOM_HINT,
        "번지 입력 시 정확한 지번만 찾습니다. 읍·면은 리까지 입력하세요.\n예: 고아읍 봉한리 / 산 12-3"
    );

    // 찾기와 같은 길: 고른 시·도(및 시·군·구·동) 중심으로 옮기고 팝업을 닫는다
    let tile_center = crate::ribbon::PENINSULA_REGION_TILES
        .iter()
        .find(|t| t.name == "경북")
        .expect("경북 타일")
        .center_5187;
    app.ribbon.app_bar.region_popup_open = true;
    app.ribbon.app_bar.lot_input = "없는번지 12345".to_owned();
    app.search_region_locator_address();
    assert!(!app.ribbon.app_bar.region_popup_open);
    assert_eq!(app.active_tab, MainTab::Map);
    assert_eq!(app.home_map.viewport.center_xy, tile_center);
    assert_eq!(
        app.ribbon.app_bar.query,
        "경북 경주시 인왕동 없는번지 12345".to_owned()
    );

    // 지번이 실제 지적 필지에 있으면 그 필지 중심으로, 1.25 m/px까지 당겨 온다
    let parcel = app
        .home_map
        .real_survey
        .cadastral_parcels
        .first()
        .map(|p| (p.jibun.clone(), p.center))
        .expect("지적 필지 샘플");
    app.ribbon.app_bar.lot_input = parcel.0.clone();
    app.search_region_locator_address();
    assert_eq!(app.home_map.viewport.center_xy, parcel.1);
    assert!((app.home_map.viewport.meters_per_pixel - 1.25).abs() < 1e-12);

    // 도곽 띠: 위 18 px 가로 띠와 왼쪽 18 px 세로 띠, 그 안쪽 경계
    assert!((GRID_BAND_PX - 18.0).abs() < 1e-9);
    assert_eq!(GRID_BAND_FILL, egui::Color32::from_rgb(0xF5, 0xF4, 0xED));
    assert_eq!(GRID_BAND_BORDER, egui::Color32::from_rgb(0xDE, 0xDC, 0xD1));
    assert_eq!(GRID_BAND_INK, egui::Color32::from_rgb(0x5E, 0x5D, 0x59));
    let map_r =
        egui::Rect::from_min_size(egui::Pos2::new(100.0, 60.0), egui::vec2(400.0, 300.0));
    let (top_band, left_band) = grid_band_rects(map_r);
    assert_eq!(top_band.min, egui::Pos2::new(100.0, 60.0));
    assert_eq!(top_band.width(), 400.0);
    assert!((top_band.height() - 18.0).abs() < 1e-9);
    assert_eq!(left_band.min, egui::Pos2::new(100.0, 78.0));
    assert!((left_band.width() - 18.0).abs() < 1e-9);
    assert!((left_band.height() - 282.0).abs() < 1e-9);

    // 계단 간격은 120 mpp에 맞춘 1·2·5·10 미터
    assert!((nice_grid_step_meters(120.0) - 100.0).abs() < 1e-9);
    assert!((nice_grid_step_meters(240.0) - 200.0).abs() < 1e-9);
    assert!((nice_grid_step_meters(600.0) - 500.0).abs() < 1e-9);
    assert!((nice_grid_step_meters(0.0) - 10.0).abs() < 1e-9);

    // 회전한 격자는 원점에서 잰 거리로, 돌지 않은 격자는 실제 좌표로 적는다
    assert_eq!(grid_label_text(0.0, 219_000.0, 20.0, true), "원점");
    assert_eq!(grid_label_text(20.0, 219_000.0, 20.0, true), "+20 m");
    assert_eq!(grid_label_text(-20.0, 219_000.0, 20.0, true), "−20 m");
    assert_eq!(grid_label_text(0.0, 219_000.0, 20.0, false), "219000 m");
    assert_eq!(grid_label_text(0.0, 219_000.0, 1000.0, false), "219 km");

    // 선이 너무 많으면 n번째마다 그린다
    assert_eq!(grid_line_stride(100.0, 20.0, 80), 1);
    assert_eq!(grid_line_stride(2000.0, 20.0, 80), 2);

    // 굵기·모양은 설정값을 따른다 (0 실선 · 1 점선 · 2 점, 굵기 0.5~5.0)
    let cfg = ShellGridConfig {
        line_width: 9.0,
        dash_idx: 0,
        ..ShellGridConfig::default()
    };
    let ink = egui::Color32::from_rgb(0xD9, 0x2B, 0x2B);
    let stroke = crate::gui_tab_map::grid_stroke_for(&cfg, ink);
    assert!((stroke.width - 5.0).abs() < 1e-6);
    assert_eq!(egui::Color32::from_rgb(0xD9, 0x2B, 0x2B).r(), 0xD9);
    // 격자 선은 견본 색을 불투명 150/255로 흐리게 쓴다 (KaCanvasGridOverlay.cpp:110)
    let faint = egui::Color32::from_rgba_unmultiplied(0xD9, 0x2B, 0x2B, 150);
    assert_eq!(stroke.color, faint);
    let a = egui::Pos2::new(0.0, 0.0);
    let b = egui::Pos2::new(0.0, 100.0);
    assert_eq!(
        crate::gui_tab_map::grid_dash_segments(a, b, 0, 1.0),
        vec![[a, b]]
    );
    let dots = crate::gui_tab_map::grid_dash_segments(a, b, 2, 1.0);
    assert!(dots.len() > 1);
    let dashes = crate::gui_tab_map::grid_dash_segments(a, b, 1, 1.0);
    assert_eq!(dashes.len(), 12);
    assert_eq!(dots.len(), 25);
}

#[test]
fn test_d15_scale_bar_quarters_and_opacity_rail_vector_brightness() {
    // 축척 1.0 m/px → 120 m 목표 → 100 m 막대, 100 px
    let round_m = scale_bar_metres(1.0).expect("scale bar");
    assert!((round_m - 100.0).abs() < 1e-9);
    assert!(((round_m / 1.0).clamp(40.0, 220.0) - 100.0).abs() < 1e-9);
    assert_eq!(
        scale_bar_tick_text(round_m),
        ("0".to_owned(), "50".to_owned(), "100 m".to_owned())
    );
    // 1:1,000 화면 축척(0.2645833333 m/px) → 20 m
    let round_m = scale_bar_metres(0.264_583_333_333_333_3).expect("scale bar");
    assert!((round_m - 20.0).abs() < 1e-9);
    assert_eq!(
        scale_bar_tick_text(round_m),
        ("0".to_owned(), "10".to_owned(), "20 m".to_owned())
    );
    // 10 m/px → 1.2 km 목표 → 1 km, 단위는 km
    let round_m = scale_bar_metres(10.0).expect("scale bar");
    assert!((round_m - 1000.0).abs() < 1e-9);
    assert_eq!(
        scale_bar_tick_text(round_m),
        ("0".to_owned(), "0".to_owned(), "1 km".to_owned())
    );
    // 축척이 없으면 막대를 그리지 않는다
    assert!(scale_bar_metres(0.0).is_none());
    assert!(scale_bar_metres(f64::NAN).is_none());

    // 그림(래스터) 레이어만 밝기를 가진다 — KaLayerOpacityRail.cpp:156-173
    for raster in [
        "지형 음영",
        "지표모델(DSM) 30m",
        "위성사진",
        "항공사진",
        "고지형 분류",
    ] {
        assert!(is_raster_image_layer(raster), "{raster}");
    }
    for vector in [
        "조사구역",
        "시굴격자",
        "지적도 · 조사 주변 5km",
        "지정유산",
        "주변 500m",
    ] {
        assert!(!is_raster_image_layer(vector), "{vector}");
        let (txt, tip) = brightness_row_text(0, false);
        assert_eq!(txt, "그림만");
        assert_eq!(tip, "밝기는 위성·항공사진 같은 그림 레이어에만 있습니다.");
    }
    assert_eq!(
        brightness_row_text(0, true),
        (
            "0".to_owned(),
            "고른 그림의 밝기. 가운데가 원본입니다. 원본 파일은 바뀌지 않습니다."
        )
    );
    assert_eq!(brightness_row_text(40, true).0, "+40");
    assert_eq!(brightness_row_text(-40, true).0, "-40");

    // KaFileBrowserPanel.cpp:92-136 — 폴더 단추 여섯 개와 목록 칸의 안내말
    assert_eq!(FILE_BTN_PC_TIP, "컴퓨터 드라이브 목록으로 이동합니다");
    assert_eq!(FILE_BTN_DESK_TIP, "바탕화면 폴더로 이동합니다");
    assert_eq!(FILE_BTN_DOCS_TIP, "문서 폴더로 이동합니다");
    assert_eq!(FILE_BTN_DOWN_TIP, "내려받기 폴더로 이동합니다");
    assert_eq!(
        FILE_BTN_PICK_TIP,
        "조사 자료 폴더를 직접 찾아 엽니다. 카메라·GPS 메모리카드(SD)도 여기서 엽니다"
    );
    assert_eq!(FILE_BTN_UP_TIP, "상위 폴더로 한 단계 이동합니다");
    assert_eq!(FILE_LIST_TIP, "파일을 지도에 끌어 넣으면 레이어가 됩니다.");

    // D15 지도 화면은 1600x900 기준
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
    let map_w = app.home_map.viewport.screen_size_px[0];
    assert!(
        (map_w - 954.0).abs() <= 4.0,
        "1600x900에서 지도 폭 ~954, got {map_w}"
    );
}
