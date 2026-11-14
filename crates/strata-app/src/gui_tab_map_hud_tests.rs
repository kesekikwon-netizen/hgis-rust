//! Guide band title, hint, and live sketch measure tests for line and polygon capture
//! (`target/cpp-ref/src/app/KaDrawSketchTools.cpp:173-199,250-260`,
//! `target/cpp-ref/src/app/MainWindowChrome.cpp:71-73`,
//! `target/cpp-ref/src/app/KaCaptureMapTool.cpp:218-225`,
//! `target/cpp-ref/src/app/KaSketchMeasure.cpp:7-26`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};

use super::MapToolFlags;

fn flags_for(tool: MapToolKind, cursor_world: Option<[f64; 2]>) -> MapToolFlags {
    MapToolFlags {
        is_trench: false,
        is_measure: false,
        is_draw_tool: true,
        is_select_tool: false,
        is_style: false,
        effective_tool: tool,
        cursor_world,
    }
}

#[test]
fn guide_band_shows_tool_chip_title_hint_and_cursor_measure_for_line_and_polygon() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.survey_path = Some(std::env::temp_dir().join("hud.gpkg"));

    // Line: feature_line and section_line titles + hint before 2 measured points.
    app.active_tool = MapToolKind::DrawLine;
    app.edit_layer_key = Some("feature_line".to_owned());
    assert_eq!(
        app.map_guide_band_info(flags_for(MapToolKind::DrawLine, Some([50.0, 50.0]))),
        Some(("그리기 · 유구 선".to_owned(), "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned()))
    );

    app.edit_layer_key = Some("section_line".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0]];
    assert_eq!(
        app.map_guide_band_info(flags_for(MapToolKind::DrawLine, None)),
        Some(("그리기 · 단면선 · 점 1".to_owned(), "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)".to_owned()))
    );

    // Line with 2 sketch points + cursor: KaSketchMeasure::text({{0,0},{3,4},{3,10}}, false).
    app.edit_layer_key = Some("feature_line".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0], [3.0, 4.0]];
    assert_eq!(
        app.map_guide_band_info(flags_for(MapToolKind::DrawLine, Some([3.0, 10.0]))),
        Some(("그리기 · 유구 선 · 점 2".to_owned(), "변 6.00 m · 길이 11.00 m".to_owned()))
    );

    // Polygon with 1 sketch point + cursor: KaSketchMeasure::text({{0,0},{4.2,0}}, true).
    app.active_tool = MapToolKind::DrawPolygon;
    app.edit_layer_key = Some("feature_poly".to_owned());
    app.draw_sketch.points = vec![[0.0, 0.0]];
    assert_eq!(
        app.map_guide_band_info(flags_for(MapToolKind::DrawPolygon, Some([4.2, 0.0]))),
        Some(("그리기 · 유구 면 · 점 1".to_owned(), "변 4.20 m".to_owned()))
    );

    // Polygon with 2 sketch points + cursor: KaSketchMeasure::text({{0,0},{10,0},{10,5}}, true).
    app.draw_sketch.points = vec![[0.0, 0.0], [10.0, 0.0]];
    assert_eq!(
        app.map_guide_band_info(flags_for(MapToolKind::DrawPolygon, Some([10.0, 5.0]))),
        Some(("그리기 · 유구 면 · 점 2".to_owned(), "변 5.00 m · 면적 25.0 ㎡".to_owned()))
    );
}
