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
