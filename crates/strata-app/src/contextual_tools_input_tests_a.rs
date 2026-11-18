//! Actual toolbar actions, target/cpp-ref/src/app/MainWindowRibbon.cpp:798-952.
use super::{open, start, visible_texts, Harness};
use eframe::egui::{self, PointerButton, Rect, Shape};

pub(super) fn card(h: &mut Harness, title: &str) -> Rect {
    let output = h.frame(vec![]);
    let caption = visible_texts(&output).into_iter().find(|(s, _)| s.trim() == title).expect("card caption").1;
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Rect(r) if r.rect.contains(caption.center()) && r.rect.width() > 200.0 && r.rect.height() > 30.0 => Some(r.rect),
            _ => None,
        })
        .min_by(|a, b| a.area().total_cmp(&b.area()))
        .expect("actual card background")
}

fn icons(h: &mut Harness) -> Vec<Rect> {
    let area = card(h, "그리기 ›");
    let output = h.frame(vec![]);
    let mut icons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Rect(r) if area.contains_rect(r.rect) && r.rect.width() == r.rect.height() && r.rect.height() > 25.0 && r.rect.height() < 45.0 => Some(r.rect),
            _ => None,
        })
        .collect();
    icons.sort_by(|a, b| a.top().total_cmp(&b.top()).then(a.left().total_cmp(&b.left())));
    icons.dedup();
    icons
}

#[test]
fn context_tools_draw_icons_toggles_and_close_preserve_originals() {
    let mut h = start(false);
    let path = h.app.survey_path.clone().expect("survey");
    let bytes = std::fs::read(&path).expect("before");
    let records = super::super::vertex_crs_input_tests::records(&h);
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    open(&mut h, "그리기", "기록");
    // C++ action order: selection, snap assistance, easy, area, polygon, line, artifact.
    let controls = icons(&mut h);
    assert_eq!(controls.len(), 7, "actual icon controls");
    for (index, tool, layer) in [
        (3, strata_canvas2d::map_tools::MapToolKind::DrawPolygon, "survey_area"),
        (4, strata_canvas2d::map_tools::MapToolKind::DrawPolygon, "feature_poly"),
        (5, strata_canvas2d::map_tools::MapToolKind::DrawLine, "feature_line"),
        (6, strata_canvas2d::map_tools::MapToolKind::PlacePoint, "artifact"),
    ] {
        h.press(controls[index].center(), PointerButton::Primary);
        assert_eq!(h.app.active_tool, tool);
        assert!(h.app.edit_layer_key.as_deref().expect("edit layer").starts_with(layer));
    }
    h.press(controls[0].center(), PointerButton::Primary);
    assert_eq!(h.app.active_tool, strata_canvas2d::map_tools::MapToolKind::Select);
    h.press(controls[2].center(), PointerButton::Primary);
    assert_eq!(h.app.active_tool, strata_canvas2d::map_tools::MapToolKind::Select, "unconnected easy draw acted enabled");
    // egui 0.31.1 response.rs suppresses resting-after-click tooltips; deliberately leave, then hover.
    h.frame(vec![egui::Event::PointerMoved(egui::pos2(700.0, 400.0))]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h.frame(vec![egui::Event::PointerMoved(controls[2].center())]);
    for _ in 0..20 {
        h.frame(vec![]);
    }
    h.point_at("아직 Rust 판에서 연결하지 않았습니다 (지시 MENU17)");
    h.save("context-draw-unconnected-tooltip");
    h.frame(vec![egui::Event::PointerMoved(egui::pos2(700.0, 400.0))]);
    for expected in [true, false] {
        let at = h.point_at("연속 그리기");
        h.press(at, PointerButton::Primary);
        assert_eq!(h.app.ribbon.continuous_draw, expected);
    }
    let at = h.point_at("구간 분리");
    h.press(at, PointerButton::Primary);
    assert!(h.app.modals.active.is_none());
    h.save("context-draw-selected-disabled");
    let at = h.point_at("닫기");
    h.press(at, PointerButton::Primary);
    assert!(!h.app.draw_row_open());
    assert_eq!(h.app.active_tool, strata_canvas2d::map_tools::MapToolKind::Pan);
    assert_eq!(super::super::vertex_crs_input_tests::records(&h), records);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    assert_eq!(std::fs::read(path).expect("after"), bytes);
}

#[test]
fn context_tools_buffer_actions_repeat_close_and_reopen_without_survey_writes() {
    let mut h = start(false);
    let path = h.app.survey_path.clone().expect("survey");
    let bytes = std::fs::read(&path).expect("before");
    let records = super::super::vertex_crs_input_tests::records(&h);
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    for width in [1680, 800, 1280] {
        h.size[0] = width;
        open(&mut h, "버퍼", "기록");
        for radius in [500, 1000, 500] {
            let at = h.point_at(&format!("{radius}m"));
            h.press(at, PointerButton::Primary);
            assert!(h.app.home_map.real_survey.distance_rings.contains_key(&radius));
        }
        assert_eq!(h.app.home_map.real_survey.distance_rings.len(), 2);
        let before = h.app.home_map.real_survey.distance_rings.clone();
        let at = h.point_at("엑셀 내보내기");
        h.press(at, PointerButton::Primary);
        assert!(h.app.modals.active.is_none());
        h.save(&format!("context-buffer-{width}"));
        let at = h.point_at("닫기");
        h.press(at, PointerButton::Primary);
        assert!(!h.app.ribbon.sub_toolbar_visible);
        assert_eq!(h.app.home_map.real_survey.distance_rings, before);
    }
    assert_eq!(super::super::vertex_crs_input_tests::records(&h), records);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    assert_eq!(std::fs::read(path).expect("after"), bytes);
}

#[test]
fn context_tools_map_capture_keeps_the_card_and_snap_popup_escape_keeps_points() {
    let mut h = start(false);
    open(&mut h, "그리기", "기록");
    let controls = icons(&mut h);
    h.press(controls[4].center(), PointerButton::Primary);
    for at in [egui::pos2(700.0, 450.0), egui::pos2(820.0, 450.0), egui::pos2(820.0, 570.0)] {
        h.press(at, PointerButton::Primary);
    }
    assert_eq!(h.app.draw_sketch.points.len(), 3, "persistent toolbar blocked actual map capture");
    assert!(h.app.draw_row_open());
    let at = h.point_at("모든 조사 레이어");
    h.press(at, PointerButton::Primary);
    h.point_at("현재 레이어");
    h.close_popup_without_losing_capture("context-draw-pending");
    h.press(egui::pos2(760.0, 650.0), PointerButton::Primary);
    assert_eq!(h.app.draw_sketch.points.len(), 4);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty());
}
