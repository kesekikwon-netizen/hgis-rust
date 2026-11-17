//! Coordinate admission, target/cpp-ref/src/app/KaVertexEditTool.cpp:110-133.
use super::popup_input_harness::Harness;
use eframe::egui::{Event, Modifiers, PointerButton, Pos2};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::edit_buffer::FeatureRecord;

pub(super) fn fixture(source: i32) -> Harness {
    fixture_table("feature_poly", source)
}

pub(super) fn fixture_table(table: &str, source: i32) -> Harness {
    let table = strata_types::schema::DomainLayerKey::from_id(table).expect("known demo domain table").id();
    let mut h = Harness::drawing();
    let path = h.app.survey_path.clone().expect("temporary survey");
    let conn = rusqlite::Connection::open(&path).expect("open temporary copy");
    strata_storage::gpkg_sql_functions::register_gpkg_sql_functions(&conn).expect("real spatial functions");
    let blobs: Vec<(i64, Vec<u8>)> = conn.prepare(&format!("SELECT fid, geom FROM \"{table}\" ORDER BY fid")).expect("prepare").query_map([], |row| Ok((row.get(0)?, row.get(1)?))).expect("query").collect::<Result<_, _>>().expect("rows");
    // target/cpp-ref/src/app/KaVertexEditTool.cpp:110-116: relabel only this copied fixture, retaining stored coordinates.
    conn.execute("INSERT OR IGNORE INTO gpkg_spatial_ref_sys (srs_name,srs_id,organization,organization_coordsys_id,definition,description) VALUES ('test source',?1,'NONE',?1,'undefined','test-only source')", (source,))
        .expect("fixture CRS");
    conn.execute("UPDATE gpkg_geometry_columns SET srs_id=?1 WHERE table_name=?2", (source, table)).expect("geometry metadata");
    conn.execute("UPDATE gpkg_contents SET srs_id=?1 WHERE table_name=?2", (source, table)).expect("contents metadata");
    for (fid, mut blob) in blobs {
        assert_eq!(&blob[..2], b"GP");
        assert_eq!(blob[3] & 1, 1, "tracked demo header is little endian");
        blob[4..8].copy_from_slice(&source.to_le_bytes());
        conn.execute(&format!("UPDATE \"{table}\" SET geom=?1 WHERE fid=?2"), (&blob, fid)).expect("consistent fixture header");
    }
    drop(conn);
    assert!(h.app.open_survey_gpkg(&path), "actual production open");
    assert_eq!(h.app.survey_tables.iter().find(|t| t.table == table).expect("opened table").srs_id, source);
    h.app.active_tab = crate::app_state::MainTab::Map;
    h.app.active_tool = MapToolKind::Select;
    h.app.ribbon.sub_toolbar_visible = false;
    for entry in h.app.left_layers.entries.clone() {
        h.app.home_map.real_survey.layer_visible.insert(entry.label_ko, entry.layer_key == table);
    }
    for _ in 0..3 {
        h.frame(vec![]);
    }
    // target/cpp-ref/src/app/KaVertexEditTool.cpp:110-116: raw fixture numbers are not a valid work-CRS conversion.
    h.app.home_map.viewport.center_xy = if matches!(source, 5186 | 5187) { mapped(source, [200_016.0, 450_014.5]) } else { [200_016.0, 450_014.5] };
    h.app.home_map.viewport.meters_per_pixel = 0.1;
    h.frame(vec![]);
    assert!(h.app.home_map.selected_shape.is_none());
    assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
    h
}

fn at(h: &mut Harness, raw: [f64; 2]) -> Pos2 {
    let probe = Pos2::from([900.0, 550.0]);
    h.frame(vec![Event::PointerMoved(probe)]);
    assert!(h.app.home_map.cursor_seen);
    let origin = h.app.home_map.viewport.world_to_screen(h.app.home_map.cursor_world_xy);
    let target = h.app.home_map.viewport.world_to_screen(raw);
    Pos2::from([probe.x + (target[0] - origin[0]) as f32, probe.y + (target[1] - origin[1]) as f32])
}

fn drag(h: &mut Harness, from: Pos2, to: Pos2) {
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }]);
    h.frame(vec![Event::PointerMoved(to)]);
    h.frame(vec![Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }]);
    h.frame(vec![]);
}

pub(super) fn records(h: &Harness) -> Vec<(String, Vec<FeatureRecord>)> {
    h.app.survey_tables.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect()
}

fn mapped(source: i32, stored: [f64; 2]) -> [f64; 2] {
    strata_engine::Transformer::new(u32::try_from(source).expect("known source"), 5187).expect("valid fixture CRS").transform(stored).expect("fixture point")
}

fn valid_pointer_drag(source: i32) {
    let mut h = fixture(source);
    let before = records(&h);
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source bytes");
    let inside = at(&mut h, mapped(source, [200_016.0, 450_014.0]));
    h.press(inside, PointerButton::Primary);
    assert_eq!(h.app.home_map.selected_shape, Some(("feature_poly".to_owned(), 1)));
    let from = at(&mut h, mapped(source, [200_010.0, 450_010.0]));
    drag(&mut h, from, Pos2::from([from.x + 25.0, from.y + 15.0]));
    let mut after = records(&h);
    let old = &before.iter().find(|t| t.0 == "feature_poly").expect("original table").1[0].wkt_geom;
    let changed = &mut after.iter_mut().find(|t| t.0 == "feature_poly").expect("same table").1[0];
    assert_ne!(&changed.wkt_geom, old, "valid pointer drag did not edit the feature");
    let parts = strata_storage::gpkg_wkt::wkt_to_parts(&changed.wkt_geom).expect("edited geometry");
    let moved = crate::app_state::vertex_edit_geom::vertices(&parts)[0];
    let shown = mapped(source, moved);
    let released = h.app.home_map.cursor_world_xy;
    // target/cpp-ref/tests/test_editing_tools.cpp:263: placement within 1e-6 m; this checks the local PROJ round trip only.
    let placement_error_m = (shown[0] - released[0]).hypot(shown[1] - released[1]);
    assert!(placement_error_m < 1e-6, "EPSG:{source} stored vertex missed the released work-CRS EPSG:5187 position by {placement_error_m} m");
    changed.wkt_geom = old.clone();
    assert_eq!(after, before, "valid vertex drag altered other records or attributes");
    assert!(h.app.is_dirty && h.app.undo_tables.len() == 1);
    h.save(&format!("vertex-crs-{source}-valid-drag"));
    super::menu_selection_keys_tests::key(&mut h, egui::Key::Z, Modifiers { ctrl: true, ..Default::default() });
    assert_eq!(records(&h), before, "actual Undo must restore source coordinates and identifiers");
    assert_eq!(std::fs::read(&path).expect("source remains"), bytes);
}

#[test]
fn vertex_crs_input_valid_cross_crs_pointer_drag_remains_undoable() {
    valid_pointer_drag(5186);
}

#[test]
fn vertex_crs_input_valid_identity_pointer_drag_remains_undoable() {
    valid_pointer_drag(5187);
}

fn unknown_pointer_drag(source: i32) {
    let mut h = fixture(source);
    let before = records(&h);
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source bytes");
    h.save(&format!("vertex-crs-{source}-fixture"));
    let inside = at(&mut h, [200_016.0, 450_014.0]);
    h.press(inside, PointerButton::Primary);
    eprintln!("source={source} actual pointer selection={:?}", h.app.home_map.selected_shape);
    let from = at(&mut h, [200_010.0, 450_010.0]);
    let to = Pos2::from([from.x + 25.0, from.y + 15.0]);
    drag(&mut h, from, to);
    h.save(&format!("vertex-crs-{source}-after-drag"));
    assert_eq!(records(&h), before, "unknown source CRS allowed a pointer vertex edit");
    assert!(h.app.undo_tables.is_empty() && h.app.redo_tables.is_empty() && !h.app.is_dirty);
    assert!(h.app.home_map.selected_shape.is_none(), "unknown coordinates became a selected work-CRS shape");
    assert_eq!(std::fs::read(&path).expect("unchanged source"), bytes);
}

#[test]
fn vertex_crs_input_undefined_source_never_drives_pointer_vertex_edit() {
    unknown_pointer_drag(0);
}

#[test]
fn vertex_crs_input_negative_source_never_drives_pointer_vertex_edit() {
    unknown_pointer_drag(-1);
}

#[test]
fn vertex_crs_input_unknown_positive_source_never_drives_pointer_vertex_edit() {
    unknown_pointer_drag(999_999);
}

#[test]
fn vertex_crs_input_undefined_source_never_enters_box_selection() {
    let mut h = fixture(0);
    let before = records(&h);
    let path = h.app.survey_path.clone().expect("temporary copy");
    let bytes = std::fs::read(&path).expect("source bytes");
    let from = at(&mut h, [200_008.0, 450_008.0]);
    let to = at(&mut h, [200_024.0, 450_021.0]);
    drag(&mut h, from, to);
    assert!(h.app.home_map.selected_shape.is_none() && h.app.home_map.also_selected.is_empty(), "box treated undefined source as work CRS");
    assert_eq!(records(&h), before);
    assert!(!h.app.is_dirty && h.app.undo_tables.is_empty());
    assert_eq!(std::fs::read(&path).expect("source remains"), bytes);
}
