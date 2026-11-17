//! Actual popup Escape flows, target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391.
use super::popup_input_harness::Harness;
use crate::app_state::MainTab;
use eframe::egui::{self, Event, Key, PointerButton};

#[test]
fn popup_escape_closes_actual_compact_layer_menu_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    let at = h.point_at("레이어");
    h.press(at, PointerButton::Primary);
    h.point_at("파일함");
    h.close_popup_without_losing_capture("popup-escape-native-layer");
    let output = h.frame(vec![]);
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "파일함")), "native menu did not close");
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels after native menu closed");
}

#[test]
fn popup_escape_closes_reopened_compact_ribbon_menu_without_discarding_capture() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    for _ in 0..2 {
        let at = h.point_at("기록");
        h.press(at, PointerButton::Primary);
        h.point_at("그리기");
        h.close_popup_without_losing_capture("popup-escape-native-ribbon");
        let output = h.frame(vec![]);
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "그리기")), "native ribbon menu did not close");
    }
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape blocked after reopening a native menu");
}

#[test]
fn popup_escape_on_native_menu_click_release_keeps_capture() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    let at = h.point_at("레이어");
    let points = h.app.draw_sketch.points.clone();
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
    h.frame(vec![
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Default::default() },
        Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() },
    ]);
    assert_eq!(h.app.draw_sketch.points, points, "batched native menu click/Escape discarded capture");
}

#[test]
fn popup_escape_stale_stock_popup_id_does_not_block_canvas_cancel() {
    let mut h = Harness::drawing();
    h.ctx.memory_mut(|memory| memory.open_popup(egui::Id::new("orphan_combo_after_closed_dialog")));
    assert!(h.ctx.memory(|memory| memory.any_popup_open()));
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "orphan stock popup id blocked real canvas cancellation");
}

#[test]
fn popup_escape_with_enter_in_one_input_batch_does_not_commit_the_cancelled_polygon() {
    let mut h = Harness::drawing();
    let before = h.app.survey_tables.iter().find(|table| table.table == "feature_poly").expect("actual polygon table").layer.features.len();
    h.frame([Key::Escape, Key::Enter].into_iter().map(|key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }).collect());
    let after = h.app.survey_tables.iter().find(|table| table.table == "feature_poly").expect("actual polygon table").layer.features.len();
    assert_eq!(after, before, "Escape/Enter committed the polygon before cancellation");
    assert!(h.app.draw_sketch.points.is_empty());
}

#[test]
fn popup_escape_closes_compact_layer_set_submenu_and_its_parent() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.app.sketch_cancel();
    h.app.active_tool = strata_canvas2d::map_tools::MapToolKind::Pan;
    h.app.open_studio_tab(MainTab::Drawing);
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let at = h.point_at("레이어");
    h.press(at, PointerButton::Primary);
    let at = h.point_at("세트 ▾");
    h.press(at, PointerButton::Primary);
    h.point_at("지금 켜진 레이어를 세트로 저장…");
    h.save("popup-escape-native-layer-set-child-open");
    h.escape();
    h.save("popup-escape-native-layer-set-child-closed");
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    let output = h.frame(vec![]);
    assert!(
        !output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if ["파일함", "지금 켜진 레이어를 세트로 저장…"].contains(&t.galley.text()))),
        "native menu hierarchy stayed open"
    );
}

#[test]
fn popup_escape_closes_actual_dem_menu_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    let at = h.point_at("DEM");
    h.press(at, PointerButton::Secondary);
    assert!(h.app.ribbon.dem_menu_open, "actual DEM chip did not open its menu");
    h.close_popup_without_losing_capture("popup-escape-dem");
    assert!(!h.app.ribbon.dem_menu_open);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels capture after the menu closed");
}

#[test]
fn popup_escape_closes_actual_layer_menu_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    let name = h.app.left_layers.entries.iter().find(|row| row.layer_key == "feature_poly").expect("actual feature layer").label_ko.clone();
    let at = h.point_at(&name);
    h.press(at, PointerButton::Secondary);
    assert!(h.app.layer_context_menu_open, "actual layer right click did not open its menu");
    h.close_popup_without_losing_capture("popup-escape-layer");
    assert!(!h.app.layer_context_menu_open);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels capture after the menu closed");
}

#[test]
fn popup_escape_closes_actual_region_address_form_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    let at = h.point_at("지역");
    h.press(at, PointerButton::Primary);
    assert!(h.app.ribbon.app_bar.region_popup_open);
    let at = h.point_at("서울");
    h.press(at, PointerButton::Primary);
    assert!(h.app.ribbon.app_bar.address_popup_open, "actual province chip did not open the address form");
    h.close_popup_without_losing_capture("popup-escape-address");
    assert!(!h.app.ribbon.app_bar.address_popup_open);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels capture after the address form closed");
}
