//! Actual interrupted target flows, target/cpp-ref/src/app/KaAlignMapTool.cpp:530-555.
use super::*;

fn held_drag(h: &mut Harness) -> Pos2 {
    source(h);
    let from = at(h, CORNER);
    h.press(from, PointerButton::Primary);
    wait(h, |h| h.app.align.pairs.len() == 1);
    let target = at(h, [200_020.0, 450_020.0]) + egui::vec2(4.0, -4.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    }]);
    h.frame(vec![Event::PointerMoved(target)]);
    assert_eq!(h.app.align.raster.as_ref().expect("session").drag, Some((false, 0)), "actual target handle was not grabbed");
    target
}
fn release(h: &mut Harness, at: Pos2) {
    h.frame(vec![Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }]);
    for _ in 0..5 {
        h.frame(vec![]);
    }
}

#[test]
fn align_snap_escape_during_a_held_target_drag_cannot_commit_on_release() {
    let mut h = start();
    let baseline = original(&h);
    let target = held_drag(&mut h);
    let pairs = h.app.align.pairs.clone();
    h.escape();
    release(&mut h, target);
    assert_eq!(h.app.align.pairs, pairs, "Escape cancelled the preview but release still moved the target pair");
    assert_eq!(h.app.active_tool, strata_canvas2d::map_tools::MapToolKind::GeorefAlign);
    assert_eq!(original(&h), baseline);
}

#[test]
fn align_snap_secondary_cancel_can_return_to_a_new_source_pick() {
    let mut h = start();
    source(&mut h);
    let cursor = at(&h, CORNER) + egui::vec2(-4.0, 4.0);
    h.press(cursor, PointerButton::Secondary);
    assert!(h.app.align.pending_source.is_none());
    h.press(cursor, PointerButton::Primary);
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert!(h.app.align.pairs.is_empty(), "cancelled source was reused by a target click");
    let pixel = source(&mut h);
    h.press(cursor, PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    assert_eq!(h.app.align.pairs[0].src, pixel);
    work_eq(h.app.align.pairs[0].map, CORNER);
}

#[test]
fn align_snap_segment_and_outside_tolerance_have_distinct_actual_results() {
    for (work, offset, expected) in [([200_010.0, 450_000.0], egui::vec2(0.0, 4.0), [200_010.0, 450_000.0]), (CORNER, egui::vec2(-16.0, 16.0), [199_998.4, 449_998.4])] {
        let mut h = start();
        let baseline = original(&h);
        source(&mut h);
        h.press(at(&h, work) + offset, PointerButton::Primary);
        wait(&mut h, |h| h.app.align.pairs.len() == 1);
        work_eq(h.app.align.pairs[0].map, expected);
        assert_eq!(original(&h), baseline);
    }
}

#[test]
fn align_snap_hiding_a_ready_source_invalidates_its_target_index() {
    let mut h = start();
    source(&mut h);
    let cursor = at(&h, CORNER) + egui::vec2(-4.0, 4.0);
    h.press(cursor, PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    work_eq(h.app.align.pairs[0].map, CORNER);
    let label = h.app.left_layers.entries.iter().find(|r| r.layer_key == "feature_poly").expect("fixture layer").label_ko.clone();
    // Prepared visibility boundary, not a claim about the legend checkbox UI.
    h.app.home_map.real_survey.layer_visible.insert(label, false);
    let pixel = source(&mut h);
    h.press(cursor, PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 2);
    assert_eq!(h.app.align.pairs[1].src, pixel);
    work_eq(h.app.align.pairs[1].map, [199_999.6, 449_999.6]);
    work_eq(h.app.align.pairs[0].map, CORNER);
}

fn loaded_vector(group: strata_types::schema::LegendGroup, current_only: bool, expected: [f64; 2]) {
    use crate::ref_vector::data::{LoadSpec, Look};
    use strata_engine::{Row, ShapeKind, Shapes, TableLayout};
    use strata_types::schema::LegendGroup;
    let mut h = start();
    for row in &h.app.left_layers.entries {
        h.app.home_map.real_survey.layer_visible.insert(row.label_ko.clone(), false);
    }
    let file = h.app.survey_path.as_ref().expect("temp survey").parent().expect("temp folder").join("snap-reference.gpkg");
    let layout = TableLayout {
        table: "recorded_line",
        epsg: 5187,
        kind: ShapeKind::Lines,
        columns: &[],
    };
    let row = Row {
        shapes: Shapes::Lines(vec![vec![CORNER, [200_020.0, 450_000.0]]]),
        cells: vec![],
    };
    strata_engine::write_new_table(&file, &layout, &mut std::iter::once(row)).expect("owned temporary vector, EPSG:5187");
    let bytes = std::fs::read(&file).expect("vector bytes");
    let key = "alignment-vector-fixture";
    let spec = LoadSpec {
        file: file.clone(),
        table: layout.table.to_owned(),
        work_epsg: 5187,
        look: Look::Lines { stroke: [60, 80, 120, 255], width_mm: 0.2 },
        label: None,
    };
    // Prepared layer installation, not a claim about the download/import dialog or live provider.
    match group {
        LegendGroup::Cadastral => h.app.show_top_vector_layer(key, "recorded vector", spec),
        LegendGroup::SurveyData => h.app.show_user_vector_layer(key, "recorded vector", spec),
        LegendGroup::ReferenceMap => h.app.show_reference_vector_layer(key, "recorded vector", spec),
        _ => unreachable!(),
    }
    assert!(
        !h.app.left_layers.entries.iter().find(|r| r.layer_key == key).expect("real vector row").snap_enabled,
        "fixture must preserve the production row policy"
    );
    h.app.home_map.cad.snap_target_current_only = current_only;
    if current_only {
        h.app.right_inspector.selected_layer_key = Some(key.to_owned());
    }
    wait(&mut h, |h| h.app.ref_vectors.extent_of(key).is_some());
    let baseline = original(&h);
    source(&mut h);
    h.press(at(&h, CORNER) + egui::vec2(-4.0, 4.0), PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    work_eq(h.app.align.pairs[0].map, expected);
    assert_eq!(original(&h), baseline);
    assert_eq!(std::fs::read(file).expect("vector bytes"), bytes);
}

#[test]
fn align_snap_loaded_cadastral_vector_is_an_eligible_target() {
    loaded_vector(strata_types::schema::LegendGroup::Cadastral, false, CORNER);
}
#[test]
fn align_snap_loaded_user_vector_is_an_eligible_target() {
    loaded_vector(strata_types::schema::LegendGroup::SurveyData, false, CORNER);
}
#[test]
fn align_snap_loaded_general_reference_is_not_an_eligible_target() {
    loaded_vector(strata_types::schema::LegendGroup::ReferenceMap, false, [199_999.6, 449_999.6]);
}

#[test]
fn align_snap_loaded_current_reference_uses_active_layer_scope() {
    loaded_vector(strata_types::schema::LegendGroup::ReferenceMap, true, CORNER);
}
