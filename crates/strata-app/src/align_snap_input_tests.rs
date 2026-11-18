//! Target snapping input, target/cpp-ref/src/app/KaAlignMapTool.cpp:464-537 and KaAlignMapToolEdit.cpp:85-106.
use super::popup_input_harness::Harness;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Color32, Event, PointerButton, Pos2, Shape};

#[path = "align_snap_input_tests_b.rs"]
mod interrupted;

const CORNER: [f64; 2] = [200_000.0, 450_000.0]; // Work CRS EPSG:5187, metres.
                                                 // Two f32 screen rounding steps below 2560 logical points at0.1m/point cost<0.00005m; double that bound.
const METRE_EPS: f64 = 0.000_1;

fn start() -> Harness {
    let directory = tempfile::tempdir().expect("temporary survey");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = directory.path().join("demo.gpkg");
    std::fs::copy(root.join("samples/demo_survey/demo.gpkg"), &path).expect("tracked demo copy");
    let mut app = StrataAppState::new_startup();
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    app.history_gis_key_reader = String::new;
    assert!(app.open_survey_gpkg(&path));
    assert_eq!(app.work_crs.auth_id(), "EPSG:5187");
    // Prepared in-memory geometry on the tracked copy, not a claim about the geometry editing UI.
    let table = app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("demo polygons");
    table.srs_id = 5187; // The prepared in-memory coordinates below are EPSG:5187, unlike the copied demo's5186.
    for (i, row) in table.layer.features.iter_mut().enumerate() {
        let x = 200_000 + i * 50;
        row.wkt_geom = format!("POLYGON(({x} 450000,{} 450000,{} 450020,{x} 450020,{x} 450000))", x + 20, x + 20);
    }
    for row in &mut app.left_layers.entries {
        row.visible = row.layer_key == "feature_poly";
        app.home_map.real_survey.layer_visible.insert(row.label_ko.clone(), row.visible);
    }
    let scan = directory.path().join("scan.png");
    std::fs::copy(root.join("data/theme/ka-hgis-app.png"), &scan).expect("tracked first-party image");
    app.add_raster_from_path(&scan).expect("actual raster row");
    app.right_inspector.selected_layer_key = Some(app.pictures.layers[0].key.clone());
    app.active_tab = MainTab::Map;
    let mut h = Harness::new(app, directory);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let output = h.frame(vec![]);
    let at = output
        .shapes
        .iter()
        .rev()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.text() == "정합" && t.pos.y < 100.0 => Some(t.galley.rect.translate(t.pos.to_vec2()).center()),
            _ => None,
        })
        .expect("real alignment action");
    h.press(at, PointerButton::Primary);
    assert!(h.app.align_split_shown());
    h.app.home_map.viewport.center_xy = [200_010.0, 450_010.0];
    h.app.home_map.viewport.meters_per_pixel = 0.1;
    h.app.home_map.cad.snap_enabled = true;
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let started = std::time::Instant::now();
    loop {
        let output = h.frame(vec![]);
        let session = h.app.align.raster.as_ref().expect("session");
        let rect = session.source_rect.expect("source pane");
        let shown = output.shapes.iter().any(|s| match &s.shape {
            Shape::Mesh(m) => !m.vertices.is_empty() && m.vertices.iter().all(|v| rect.contains(v.pos)),
            _ => false,
        });
        if shown {
            break;
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(3), "actual source texture never appeared: {:?}", session.view.error);
        std::thread::yield_now();
    }
    h
}

fn at(h: &Harness, work: [f64; 2]) -> Pos2 {
    let rect = h.app.align.raster.as_ref().expect("session").map_rect.expect("painted target map");
    let at = crate::gui_tab_map::w2s(rect, &h.app.home_map.viewport, work);
    assert!(rect.shrink(20.0).contains(at), "fixture target must be on the actual map");
    at
}

fn source(h: &mut Harness) -> [f64; 2] {
    let rect = h.app.align.raster.as_ref().expect("session").source_rect.expect("painted source");
    h.press(rect.center(), PointerButton::Primary);
    h.app.align.pending_source.expect("actual raster source pick")
}

fn wait(h: &mut Harness, ready: impl Fn(&Harness) -> bool) {
    let start = std::time::Instant::now();
    while !ready(h) && start.elapsed() < std::time::Duration::from_secs(3) {
        h.frame(vec![]);
        std::thread::yield_now();
    }
    assert!(ready(h), "actual target operation did not finish");
}

fn work_eq(actual: [f64; 2], expected: [f64; 2]) {
    for axis in 0..2 {
        assert!((actual[axis] - expected[axis]).abs() <= METRE_EPS, "wrong work-CRS target: {actual:?}, expected {expected:?}");
    }
}

fn original(h: &Harness) -> impl PartialEq + std::fmt::Debug {
    (
        super::vertex_crs_input_tests::records(h),
        format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables),
        std::fs::read(h.app.survey_path.as_ref().expect("survey")).expect("survey bytes"),
        std::fs::read(&h.app.pictures.layers[0].file).expect("raster bytes"),
    )
}

fn circle(h: &mut Harness, colour: Color32) -> Option<Pos2> {
    let rect = h.app.align.raster.as_ref().expect("session").map_rect.expect("target map");
    h.frame(vec![]).shapes.iter().find_map(|s| match &s.shape {
        Shape::Circle(c) if c.stroke.color == colour && rect.contains(c.center) => Some(c.center),
        _ => None,
    })
}

#[test]
fn align_snap_existing_target_pick_uses_the_vertex_without_writing_originals() {
    let mut h = start();
    let baseline = original(&h);
    let pixel = source(&mut h);
    let cursor = at(&h, CORNER) + egui::vec2(-4.0, 4.0);
    h.press(cursor, PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    work_eq(h.app.align.pairs[0].map, CORNER);
    assert_eq!(h.app.align.pairs[0].src, pixel);
    assert!(h.app.align.pending_source.is_none());
    assert_eq!(original(&h), baseline);
    h.save("align-snap-existing-pick");
}

#[test]
fn align_snap_hover_shows_the_same_target_as_the_eventual_pick() {
    let mut h = start();
    let baseline = original(&h);
    source(&mut h);
    let expected = at(&h, CORNER);
    let cursor = expected + egui::vec2(-4.0, 4.0);
    h.frame(vec![Event::PointerMoved(cursor)]);
    for _ in 0..5 {
        h.frame(vec![]);
    }
    h.save("align-snap-hover");
    let preview = circle(&mut h, Color32::from_rgb(30, 103, 198)).expect("C++ target snap marker missing during hover");
    // One logical point covers the C++ two-point marker stroke; this is a GUI bound, not a CRS bound.
    assert!(preview.distance(expected) <= 1.0, "preview followed the unsnapped cursor instead of the snap target");
    h.press(cursor, PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    work_eq(h.app.align.pairs[0].map, CORNER);
    assert_eq!(original(&h), baseline);
}

#[test]
fn align_snap_drag_previews_the_snapped_target_without_premature_pair_mutation() {
    let mut h = start();
    source(&mut h);
    let from = at(&h, CORNER);
    h.press(from + egui::vec2(-4.0, 4.0), PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    let before = h.app.align.pairs.clone();
    let expected = at(&h, [200_020.0, 450_020.0]);
    let cursor = expected + egui::vec2(4.0, -4.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    }]);
    h.frame(vec![Event::PointerMoved(cursor)]);
    for _ in 0..5 {
        h.frame(vec![]);
    }
    h.save("align-snap-drag-held");
    assert_eq!(h.app.align.pairs, before, "drag preview changed stored control points before release");
    let preview = circle(&mut h, Color32::from_rgb(220, 38, 38)).expect("target pair mark");
    assert!(preview.distance(expected) <= 1.0, "target pair mark did not follow the snapped drag position");
    h.frame(vec![Event::PointerButton {
        pos: cursor,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }]);
    wait(&mut h, |h| h.app.align.pairs[0].map != before[0].map);
    work_eq(h.app.align.pairs[0].map, [200_020.0, 450_020.0]);
    assert_eq!(h.app.align.pairs[0].src, before[0].src);
}

#[test]
fn align_snap_current_raster_scope_does_not_use_another_survey_layer() {
    let mut h = start();
    h.app.home_map.cad.snap_target_current_only = true;
    assert!(h.app.right_inspector.selected_layer_key.as_deref().expect("current raster").starts_with("picture:"));
    source(&mut h);
    h.press(at(&h, CORNER) + egui::vec2(-4.0, 4.0), PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    work_eq(h.app.align.pairs[0].map, [199_999.6, 449_999.6]);
}

#[test]
fn align_snap_disabled_keeps_raw_work_coordinates_and_hides_the_marker() {
    let mut h = start();
    h.app.home_map.cad.snap_enabled = false;
    source(&mut h);
    let cursor = at(&h, CORNER) + egui::vec2(-4.0, 4.0);
    h.frame(vec![Event::PointerMoved(cursor)]);
    assert!(circle(&mut h, Color32::from_rgb(30, 103, 198)).is_none());
    h.press(cursor, PointerButton::Primary);
    wait(&mut h, |h| h.app.align.pairs.len() == 1);
    work_eq(h.app.align.pairs[0].map, [199_999.6, 449_999.6]);
}
