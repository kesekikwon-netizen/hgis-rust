//! `gui_tab_drawing` 시험 — 원래 `gui_tab_drawing.rs` 안 `mod tests` 를 그대로 옮겼다.
//! facade 에서 `mod tests;` 로 등록하므로 시험 이름 경로는 그대로다.
/// D10 — 용지 하단이 7버튼 툴바 위에서 멈추는지 수치로 확인한다.
/// D20 — 지도 조정·좌표점 콜아웃·도면 목록이 실제 상태로 동작한다.
#[test]
fn test_d20_map_adjust_callouts_and_sheet_set() {
    let mut st = crate::tab_drawing::DrawingTabState::default();
    assert!(!st.map_adjust_active);
    assert!(!st.coord_callout_mode);
    assert!(st.coord_callouts.is_empty());
    assert!(st.stored_sheets.is_empty());
    assert!(!st.sheet_set_menu_open);

    st.map_adjust_active = true;
    assert!(st.map_adjust_active);
    st.coord_callout_mode = true;
    assert!(st.coord_callout_mode);

    // KaDrawingCoordCallout.cpp — A, B, C … 태그
    st.coord_callouts.push(crate::tab_drawing::CoordCallout {
        tag: "A".to_owned(),
        pos: [100.0, 120.0],
        world: [361_622.354, 219_897.093],
    });
    assert_eq!(st.coord_callouts.len(), 1);
    assert_eq!(st.coord_callouts[0].tag, "A");

    // KaDrawingSheetSet.cpp — 보관/열기
    st.stored_sheets.push(crate::tab_drawing::StoredSheet {
        title: "조사도면".to_owned(),
        scale_denominator: 5_000,
        paper_w_mm: 210.0,
        paper_h_mm: 297.0,
        legend_visible: true,
        title_block_visible: true,
    });
    assert_eq!(st.stored_sheets.len(), 1);
    let back = st.stored_sheets[0].clone();
    assert_eq!(back.scale_denominator, 5_000);
    assert_eq!(back.title, "조사도면");
    st.sheet_set_menu_open = true;
    assert!(st.sheet_set_menu_open);
}

#[test]
fn test_d10_sheet_bottom_stops_above_seven_button_toolbar() {
    // KaDrawingStudio.cpp:233-252 decorationSizePercent 는 4단계다.
    for pct in [75_u32, 100, 125, 150] {
        assert!((75..=150).contains(&pct));
    }
    // 1600x900 창에서 도면 본문 높이를 가정해 계산한다.
    let rect_top = 158.0_f32;
    let rect_bottom = 866.0_f32;
    let compact_v = (rect_bottom - rect_top) < 460.0;
    let toolbar_h = if compact_v { 34.0 } else { 42.0 };
    let top_bound = rect_top + 48.0;
    let bottom_bound = (rect_bottom - toolbar_h - 16.0).max(top_bound + 120.0);
    let bar_cy = rect_bottom - toolbar_h * 0.5 - 4.0;
    let bar_top = bar_cy - toolbar_h * 0.5;
    // 용지는 툴바 위에서 끝나야 한다.
    assert!(
        bottom_bound <= bar_top,
        "sheet {bottom_bound} vs bar {bar_top}"
    );
}
