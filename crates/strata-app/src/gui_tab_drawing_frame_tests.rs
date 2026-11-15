//! F1 — 도면 지도의 좌표: 용지 점 ↔ 지상 좌표가 같은 변환, 가운데는 지도 화면 가운데(경주 기본점 없음).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::Pos2;

use super::*;

/// 좌표점 콜아웃은 그 점의 실제 지상 좌표(KaDrawingCoordCallout — 지도 칸 변환의 역).
#[test]
fn sheet_point_and_world_point_round_trip() {
    let mc = Pos2::new(400.0, 300.0);
    let center = [312_345.0, 451_234.0];
    let ppm = 2.5;
    let world = [312_400.0, 451_200.0];
    let px = world_to_sheet_px(mc, center, ppm, world);
    let back = sheet_px_to_world(mc, center, ppm, px);
    assert!((back[0] - world[0]).abs() < 0.5 && (back[1] - world[1]).abs() < 0.5, "{back:?}");
    assert_eq!(sheet_px_to_world(mc, center, ppm, mc), center);
}

/// KaDrawingStudio.cpp:188-195 studioMapExtent — 도면 지도는 지도 화면 범위(가운데)를 쓴다.
#[test]
fn sheet_map_centre_is_the_map_view_centre() {
    let mut app = crate::app_state::StrataAppState::new_startup();
    assert_eq!(app.sheet_map_center(), app.home_map.viewport.center_xy);
    app.home_map.viewport.center_xy = [250_000.0, 400_000.0];
    assert_eq!(app.sheet_map_center(), [250_000.0, 400_000.0]);
}

/// A survey whose survey area is a 200 m square around `centre`, with a small second shape 8 km away and
/// the map looking 40 km to the east.
fn survey_looking_elsewhere(centre: [f64; 2]) -> crate::app_state::StrataAppState {
    use strata_storage::edit_buffer::LayerEditBuffer;
    let (x, y) = (centre[0], centre[1]);
    let square = format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))", x - 100.0, y - 100.0, x + 100.0, y - 100.0, x + 100.0, y + 100.0, x - 100.0, y + 100.0, x - 100.0, y - 100.0);
    let mut layer = LayerEditBuffer::new(strata_types::schema::DomainLayerKey::SurveyArea);
    layer.add_unsaved_feature(square, std::collections::BTreeMap::new());
    let far = format!("POLYGON (({} {}, {} {}, {} {}, {} {}))", x + 6_000.0, y - 5_000.0, x + 6_020.0, y - 5_000.0, x + 6_020.0, y - 4_970.0, x + 6_000.0, y - 5_000.0);
    layer.add_unsaved_feature(far, std::collections::BTreeMap::new());
    let mut app = crate::app_state::StrataAppState::new_startup();
    app.survey_tables.push(strata_storage::survey_gpkg_read::SurveyFileTable {
        table: "survey_area".to_owned(),
        srs_id: 5187,
        geometry_type: "POLYGON".to_owned(),
        columns: Vec::new(),
        baseline: Vec::new(),
        layer,
    });
    app.home_map.viewport.center_xy = [x + 40_000.0, y];
    app
}

/// Owner 2026-11-15: 「모든도면은 조사구역이 중심이되게하라」. A sheet is centred on the survey area wherever
/// the map is looking (the C++ sheet takes the middle of the map view, KaDrawingStudio.cpp:188-195, and with
/// the map elsewhere the frame was empty). A place the user chose for the sheet stays.
#[test]
fn a_sheet_is_centred_on_the_survey_area() {
    let centre = [250_000.0, 400_000.0];
    let mut app = survey_looking_elsewhere(centre);
    assert_eq!(app.sheet_map_center(), centre);
    app.drawing.sheet_center = Some([251_000.0, 399_000.0]);
    assert_eq!(app.sheet_map_center(), [251_000.0, 399_000.0]);
}

/// 「레이어를 가운데」 with no layer chosen goes to the survey area (KaDrawingStudio.cpp:3708-3755) and says so.
#[test]
fn layer_to_the_middle_without_a_chosen_layer_goes_to_the_survey_area() {
    let centre = [250_000.0, 400_000.0];
    let mut app = survey_looking_elsewhere(centre);
    app.right_inspector.selected_layer_key = None;
    app.center_layer_in_sheet();
    assert_eq!(app.drawing.sheet_center, Some(centre));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("축척 1 : 5000 을 유지한 채 용지 가운데로 옮겼습니다."));
}
