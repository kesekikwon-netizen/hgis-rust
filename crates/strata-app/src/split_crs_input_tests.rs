//! Split capture admission, target/cpp-ref/src/app/KaCaptureMapTool.cpp:393-455 and MainWindowEditing.cpp:794-816.
use super::menu_selection_keys_tests::key;
use super::popup_input_harness::Harness;
use super::vertex_crs_input_tests::{fixture_table, records};
use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2};
use strata_canvas2d::map_tools::MapToolKind;

fn start(source: i32) -> Harness {
    let mut h = fixture_table("feature_poly", source);
    let label = h.app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").expect("real row").label_ko.clone();
    let row = h.point_at(&label);
    h.press(row, PointerButton::Primary);
    assert!(h.app.home_map.legend_focus);
    assert_eq!(h.app.right_inspector.selected_layer_key.as_deref(), Some(label.as_str()));
    let draw = h.point_at("그리기");
    h.press(draw, PointerButton::Primary);
    assert!(h.app.draw_row_open(), "actual drawing button did not open its tools");
    let split = h.point_at("폴리곤 나누기");
    h.press(split, PointerButton::Primary);
    assert_eq!(h.app.active_tool, MapToolKind::DrawLine, "actual split button did not start capture");
    assert_eq!(h.app.home_map.splitting.as_deref(), Some("feature_poly"));
    assert!(h.app.capture_active() && h.app.home_map.selected_shape.is_none());
    assert_eq!(h.app.work_crs.auth_id(), "EPSG:5187");
    h
}

fn at(h: &mut Harness, work: [f64; 2]) -> Pos2 {
    let probe = Pos2::from([900.0, 550.0]);
    h.frame(vec![Event::PointerMoved(probe)]);
    let origin = h.app.home_map.viewport.world_to_screen(h.app.home_map.cursor_world_xy);
    let target = h.app.home_map.viewport.world_to_screen(work);
    Pos2::from([probe.x + (target[0] - origin[0]) as f32, probe.y + (target[1] - origin[1]) as f32])
}

fn draw(h: &mut Harness, source: i32, outside: bool, misses: bool) -> Vec<[f64; 2]> {
    let raw = if outside {
        [[100_000_000.0, 99_999_990.0], [100_000_000.0, 100_000_010.0]]
    } else if misses {
        [[200_005.0, 450_006.0], [200_005.0, 450_023.0]]
    } else {
        [[200_016.0, 450_006.0], [200_016.0, 450_023.0]]
    };
    let mut actual = Vec::new();
    for xy in raw {
        let work = if !outside && matches!(source, 5186 | 5187) { strata_engine::Transformer::new(u32::try_from(source).expect("known CRS"), 5187).expect("known pair").transform(xy).expect("local point") } else { xy };
        let position = at(h, work);
        h.frame(vec![Event::PointerMoved(position)]);
        actual.push(h.app.home_map.cursor_world_xy);
        h.press(position, PointerButton::Primary);
    }
    assert_eq!(h.app.draw_sketch.points, actual, "actual clicks did not create the intended split draft");
    actual
}

fn finish(h: &mut Harness, enter: bool) {
    if enter {
        key(h, Key::Enter, Modifiers::default());
    } else {
        h.press(Pos2::from([900.0, 600.0]), PointerButton::Secondary);
    }
}

fn rejected(source: i32, outside: bool) {
    let mut h = start(source);
    if source == 0 {
        // target/cpp-ref/src/app/MainWindowSession.cpp:1225-1228: prepare unrelated unsaved work and both history directions.
        let table = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_line").expect("other table");
        let fid = table.layer.features[0].fid;
        assert!(table.layer.change_attributes(fid, &[("note".to_owned(), Some("first pending".to_owned()))]));
        assert!(table.layer.change_attributes(fid, &[("note".to_owned(), Some("second pending".to_owned()))]));
        for _ in 0..2 {
            h.app.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table("feature_line".to_owned()));
        }
        h.app.is_dirty = true;
        key(&mut h, Key::Z, Modifiers { ctrl: true, ..Default::default() });
        assert_eq!((h.app.undo_tables.len(), h.app.redo_tables.len()), (1, 1));
    }
    if outside {
        assert!(strata_engine::Transformer::new(5187, 5186).expect("known pair").transform([100_000_000.0, 100_000_000.0]).is_err());
        h.app.home_map.viewport.center_xy = [100_000_000.0, 100_000_000.0];
        h.frame(vec![]);
    }
    let before = records(&h);
    let dirty = h.app.is_dirty;
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source");
    let draft = draw(&mut h, source, outside, false);
    finish(&mut h, false);
    h.save(&format!("split-crs-{source}-{outside}-rejected"));
    assert_eq!(records(&h), before, "unknown target CRS changed original polygons with raw work coordinates");
    assert_eq!(h.app.draw_sketch.points, draft, "conversion failure discarded the pending split line");
    for enter in [true, false] {
        h.app.show_status_message("retry pending");
        finish(&mut h, enter);
        assert!(h.app.status_left_message().starts_with("아직 넣지 않았습니다"), "finish did not report its conversion error");
        assert_eq!(records(&h), before);
        assert_eq!(h.app.draw_sketch.points, draft);
        assert_eq!(h.app.is_dirty, dirty);
        assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
        assert!(h.app.home_map.feature_form.is_none() && h.app.home_map.selected_shape.is_none());
    }
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty() && h.app.capture_active());
    assert_eq!(h.app.home_map.splitting.as_deref(), Some("feature_poly"));
    let again = draw(&mut h, source, outside, false);
    finish(&mut h, false);
    assert_eq!(h.app.draw_sketch.points, again);
    assert_eq!(records(&h), before);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty());
    assert_eq!(h.app.is_dirty, dirty);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    if source == 0 {
        key(&mut h, Key::Y, Modifiers { ctrl: true, ..Default::default() });
        let table = h.app.survey_tables.iter().find(|t| t.table == "feature_line").expect("other table");
        assert_eq!(table.layer.features[0].attributes.get("note").map(String::as_str), Some("second pending"));
        key(&mut h, Key::Z, Modifiers { ctrl: true, ..Default::default() });
        assert_eq!(records(&h), before, "failed split damaged prior Undo or Redo");
    }
    assert_eq!(std::fs::read(&path).expect("unchanged source"), bytes);
}

fn valid(source: i32, enter: bool, misses: bool) {
    let mut h = start(source);
    let before = records(&h);
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source");
    draw(&mut h, source, false, misses);
    finish(&mut h, enter);
    h.save(&format!("split-crs-{source}-{enter}-{misses}-valid"));
    assert!(h.app.draw_sketch.points.is_empty(), "admitted split line must be consumed even when it crosses nothing");
    assert_eq!(h.app.home_map.splitting.as_deref(), Some("feature_poly"));
    if misses {
        assert_eq!(records(&h), before);
        assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
        assert!(h.app.status_left_message().contains("분할선이 관통하는 폴리곤이 없습니다"));
    } else {
        let after = records(&h);
        for (table, old) in &before {
            let rows = &after.iter().find(|t| t.0 == *table).expect("table preserved").1;
            if table != "feature_poly" {
                assert_eq!(rows, old, "split changed another table");
                continue;
            }
            assert_eq!(rows.len(), 3, "one of the two original rectangles must split into two");
            assert_eq!(rows.iter().find(|f| f.fid == old[1].fid), Some(&old[1]), "unrelated rectangle changed");
            let kept = rows.iter().find(|f| f.fid == old[0].fid).expect("original split identifier retained");
            assert_eq!(kept.attributes, old[0].attributes);
            let added = rows.iter().find(|f| !old.iter().any(|o| o.fid == f.fid)).expect("separate new original identifier");
            let mut expected = old[0].attributes.clone();
            expected.remove("uid");
            assert_eq!(added.attributes, expected, "new split part lost source attributes");
            // target/cpp-ref/tests/test_buffer.cpp:302-312: relative area control 1/10000, applied to the demo's 108 m2 rectangle.
            for part in [kept, added] {
                let area = strata_engine::wkt_area(&part.wkt_geom).expect("real target-CRS geometry").abs();
                assert!((area - 54.0).abs() < 108.0 / 10_000.0, "EPSG:{source} split area {area} missed the captured midline");
            }
        }
        assert_eq!(h.app.status_left_message(), "폴리곤을 성공적으로 나누었습니다.");
        // target/cpp-ref/src/app/MainWindowSession.cpp:1219-1230: modified layer buffers require the save question.
        key(&mut h, Key::N, Modifiers { ctrl: true, ..Default::default() });
        assert_eq!(h.app.save_before, Some(crate::app_state::app_state_save_before::SaveBefore::NewSurvey), "new survey bypassed the unsaved split");
        h.save(&format!("split-save-before-{source}-{enter}"));
        let cancel = h.point_at("취소");
        h.press(cancel, PointerButton::Primary);
        assert!(h.app.save_before.is_none() && h.app.modals.active.is_none());
        assert_eq!(records(&h), after, "Cancel lost the pending split");
        assert!(h.app.is_dirty && h.app.undo_tables.len() == 1, "split dirty={}, undo steps={}", h.app.is_dirty, h.app.undo_tables.len());
        key(&mut h, Key::Z, Modifiers { ctrl: true, ..Default::default() });
        assert_eq!(records(&h), before, "actual Undo did not restore all original records");
    }
    assert_eq!(std::fs::read(&path).expect("unchanged file"), bytes);
}

#[test]
fn split_crs_input_undefined_target_never_changes_original_polygons() {
    rejected(0, false);
}
#[test]
fn split_crs_input_negative_target_never_changes_original_polygons() {
    rejected(-1, false);
}
#[test]
fn split_crs_input_unknown_target_retains_failed_line() {
    rejected(999_999, false);
}
#[test]
fn split_crs_input_known_pair_point_failure_retains_failed_line() {
    rejected(5186, true);
}
#[test]
fn split_crs_input_identity_right_click_preserves_other_records_and_undo() {
    valid(5187, false, false);
}
#[test]
fn split_crs_input_cross_crs_right_click_preserves_other_records_and_undo() {
    valid(5186, false, false);
}
#[test]
fn split_crs_input_identity_enter_preserves_other_records_and_undo() {
    valid(5187, true, false);
}
#[test]
fn split_crs_input_cross_crs_enter_preserves_other_records_and_undo() {
    valid(5186, true, false);
}
#[test]
fn split_crs_input_identity_miss_consumes_line_without_editing() {
    valid(5187, false, true);
}
#[test]
fn split_crs_input_cross_crs_miss_consumes_line_without_editing() {
    valid(5186, false, true);
}
