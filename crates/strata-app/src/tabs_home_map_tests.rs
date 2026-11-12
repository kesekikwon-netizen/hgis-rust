//! `tabs_home_map` 시험 — 배경지도 투명도·밝기 클램핑, 실행취소 스택 상한·병합.
//!
//! 원래 `tabs_home_map.rs` 안 `mod tests` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 는 이 파일의 부모를 가리키므로
//! `crate::tabs_home_map::*` 로 명시한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::tabs_home_map::*;

#[test]
fn test_map_canvas_overlay_opacity_clamping_150pct_to_1_0_and_undo_redo_pill_sync() {
    let mut map = HomeMapTabState::default();
    map.set_basemap_opacity(1.5);
    assert!((map.basemap_opacity - 1.0).abs() < 1e-6);
    map.set_basemap_opacity(-0.5);
    assert!((map.basemap_opacity - 0.0).abs() < 1e-6);

    map.set_basemap_brightness(2.5);
    assert!((map.basemap_brightness - 1.0).abs() < 1e-6);

    assert_eq!(HomeMapTabState::sync_undo_redo_pills(0, 0), (false, false));
    assert_eq!(HomeMapTabState::sync_undo_redo_pills(1, 0), (true, false));

    // TestKaCommandStack::testMaxDepthLimit50: 60 pushes clamped to 50
    let mut stack = KaCommandStack::new();
    for i in 0..60 {
        stack.push(UndoCommandEntry {
            command_id: 1,
            feature_id: i,
            vertex_index: 0,
            target_xy: [i as f64, i as f64],
        });
    }
    assert_eq!(stack.undo_stack.len(), HGIS_UNDO_MAX_DEPTH);
    assert_eq!(stack.undo_stack.front().expect("oldest entry").feature_id, 10);

    // TestKaCommandStack::testVertexDragMerge: same feature/vertex drags merge
    let mut drag_stack = KaCommandStack::new();
    drag_stack.push(UndoCommandEntry {
        command_id: HGIS_CMD_VERTEX_DRAG_ID,
        feature_id: 7,
        vertex_index: 2,
        target_xy: [100.0, 200.0],
    });
    drag_stack.push(UndoCommandEntry {
        command_id: HGIS_CMD_VERTEX_DRAG_ID,
        feature_id: 7,
        vertex_index: 2,
        target_xy: [105.5, 208.25],
    });
    assert_eq!(drag_stack.undo_stack.len(), 1);
    assert_eq!(
        drag_stack.undo_stack.back().expect("merged entry").target_xy,
        [105.5, 208.25]
    );
}

/// 다각형을 닫아도 조사 GPKG 에 바로 쓰지 않는다 — 메모리에만 둔다.
/// C++ 은 저장(Ctrl+S·저장 단추·닫을 때 저장) 때만 조사 파일에 쓴다.
/// `feature_poly(geom, name, note)` 표를 만들어 두어, 예전처럼 INSERT 하면 바이트가 바뀐다.
#[test]
fn test_drawn_polygon_stays_in_memory_and_never_touches_the_survey_gpkg() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut survey = crate::real_survey::RealSurveyData::new_empty_survey(dir.path());
    {
        let conn = rusqlite::Connection::open(&survey.gpkg_path).expect("open temp gpkg");
        conn.execute_batch(
            "CREATE TABLE feature_poly (fid INTEGER PRIMARY KEY, geom BLOB, name TEXT, note TEXT);",
        )
        .expect("create feature_poly");
    }
    let before = std::fs::read(&survey.gpkg_path).expect("read before");

    let ring = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    survey.save_drawn_polygon(ring.clone());

    assert_eq!(survey.survey_rings.last(), Some(&ring));
    assert_eq!(std::fs::read(&survey.gpkg_path).expect("read after"), before);
}
