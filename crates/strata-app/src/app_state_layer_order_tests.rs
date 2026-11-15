//! Dragging a row of the legend up or down, on a temp copy of the demo survey.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, pos2, vec2, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use strata_types::schema::LegendGroup;

use crate::app_state::{MainTab, StrataAppState};

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app
}

fn keys(app: &StrataAppState) -> Vec<String> {
    app.left_layers.entries.iter().map(|e| e.layer_key.clone()).collect()
}

/// The first three rows of the survey's own layers, top to bottom.
fn three_survey_rows(app: &StrataAppState) -> [String; 3] {
    let mut rows = app.left_layers.entries.iter().filter(|e| e.group == LegendGroup::SurveyData).map(|e| e.layer_key.clone());
    [rows.next().unwrap(), rows.next().unwrap(), rows.next().unwrap()]
}

/// A row dropped on the upper half of another row of its group goes above it, on the lower half below it
/// (QTreeView drag and drop of the layer list, MainWindow.cpp:658-665). The survey has unsaved changes then,
/// and Ctrl+Z puts the rows back as one step (`KaUndoAction::LayerTreeChanged`, MainWindowUndo.cpp:391-393).
#[test]
fn a_dropped_row_goes_above_or_below_the_row_it_is_dropped_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = opened_demo(dir.path());
    let before = keys(&app);
    let [first, second, third] = three_survey_rows(&app);

    assert!(app.move_layer_row(&third, &first, false));
    assert_eq!(three_survey_rows(&app), [third.clone(), first.clone(), second.clone()]);
    assert!(app.is_dirty);

    assert!(app.move_layer_row(&third, &second, true));
    assert_eq!(three_survey_rows(&app), [first.clone(), second.clone(), third.clone()]);
    assert_eq!(keys(&app), before, "the other rows were not touched");

    assert!(!app.move_layer_row(&first, &second, false), "already just above it: nothing to do");
    assert!(app.move_layer_row(&first, &third, true));
    app.undo_map_action();
    assert_eq!(keys(&app), before);
    assert_eq!(app.status_left_message(), "이전 상태로 되돌렸습니다.");
}

/// A row stays in its own part of the legend: a survey layer is not dropped among the base maps.
#[test]
fn a_row_is_not_dropped_into_another_group() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = opened_demo(dir.path());
    let before = keys(&app);
    let [first, ..] = three_survey_rows(&app);
    let other = app.left_layers.entries.iter().find(|e| e.group != LegendGroup::SurveyData).map(|e| e.layer_key.clone()).expect("the base maps are in the legend");
    assert!(!app.move_layer_row(&first, &other, true));
    assert!(!app.move_layer_row(&first, &first, true));
    assert_eq!(keys(&app), before);
}

fn frame(app: &mut StrataAppState, ctx: &egui::Context, time: &mut f64, event: Event) {
    *time += 0.3;
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), time: Some(*time), events: vec![event], ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() }
}

/// With the mouse: the second row of the survey's layers is taken and let go on the upper half of the first,
/// and is the first row then. While the list is filtered rows are not dragged (KaLayerListChrome.cpp:100).
#[test]
fn a_row_is_dragged_with_the_mouse() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = opened_demo(dir.path());
    let (ctx, mut time) = (egui::Context::default(), 0.0);
    let [first, second, third] = three_survey_rows(&app);
    let label = |app: &StrataAppState, key: &str| app.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone());

    // Down the legend until the first row answers a click. It answers from the title band of its part of the
    // legend on (16 points); the rows themselves are 22 points high.
    let top = (0..100u8).map(|step| 120.0 + 4.0 * f32::from(step)).find(|y| {
        frame(&mut app, &ctx, &mut time, Event::PointerMoved(pos2(120.0, *y)));
        frame(&mut app, &ctx, &mut time, button(pos2(120.0, *y), true));
        frame(&mut app, &ctx, &mut time, button(pos2(120.0, *y), false));
        app.right_inspector.selected_layer_key == label(&app, &first)
    });
    let top = top.expect("the first row of the survey's layers answers a click");
    let (on_first, on_second) = (pos2(120.0, top + 16.0 + 4.0), pos2(120.0, top + 16.0 + 22.0 + 8.0));

    let drag = |app: &mut StrataAppState, time: &mut f64, from: Pos2, to: Pos2| {
        frame(app, &ctx, time, Event::PointerMoved(from));
        frame(app, &ctx, time, button(from, true));
        frame(app, &ctx, time, Event::PointerMoved(pos2(from.x, (from.y + to.y) * 0.5)));
        frame(app, &ctx, time, Event::PointerMoved(to));
        frame(app, &ctx, time, button(to, false));
    };

    // Filtered to the three rows with 「구」 in their names (유구 (선), 유구 (면), 조사구역), which then stand at
    // the top without a title band: the third is taken to the upper half of the first.
    let unfiltered = keys(&app);
    app.home_map.layer_filter = "구".to_owned();
    drag(&mut app, &mut time, pos2(120.0, top + 60.0), pos2(120.0, top + 12.0));
    assert_eq!(keys(&app), unfiltered, "a filtered list is not reordered");

    app.home_map.layer_filter.clear();
    drag(&mut app, &mut time, on_second, on_first);
    assert_eq!(three_survey_rows(&app), [second, first, third]);
    assert!(app.home_map.legend_drag.is_none());
}
