//! Explicit buffer save, target/cpp-ref/tests/test_buffer.cpp:166-198.
use super::menu_selection_keys_tests::key;
use super::popup_input_harness::Harness;
use super::vertex_crs_input_tests::{fixture_table, records};
use eframe::egui::{Key, Modifiers, PointerButton};

fn click(h: &mut Harness, title: &str) {
    let at = h.point_at(title);
    h.press(at, PointerButton::Primary);
}

fn start() -> Harness {
    let mut h = fixture_table("feature_poly", 5187);
    // target/cpp-ref/tests/test_buffer.cpp:166-198: offline geometry fixture never reads the owner's tile key.
    h.app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    let table = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("source");
    let first = table.layer.features[0].fid;
    let second = table.layer.features[1].fid;
    // target/cpp-ref/tests/test_buffer.cpp:180: prepare its 40m square on the tracked demo copy.
    let wkt = "POLYGON ((200000 450000,200040 450000,200040 450040,200000 450040,200000 450000))".to_owned();
    assert!(table.layer.change_add_delete(&[(first, wkt)], vec![], &[second]).is_some());
    h.app.after_layer_edit("feature_poly");
    h.app.is_dirty = true;
    click(&mut h, "저장");
    assert!(!h.app.is_dirty, "prepare a clean copied survey before creating any buffer");
    h.app.home_map.viewport.center_xy = [200_020.0, 450_020.0];
    h.app.home_map.viewport.meters_per_pixel = 4.0;
    h.frame(vec![]);
    let title = h.app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").expect("source row").label_ko.clone();
    click(&mut h, &title);
    click(&mut h, "버퍼");
    h
}

fn generated(h: &mut Harness, meters: u32) -> crate::real_survey::DistanceRing {
    let path = h.app.survey_path.clone().expect("temporary survey");
    let bytes = std::fs::read(&path).expect("before buffer");
    click(h, &format!("{meters}m"));
    assert_eq!(std::fs::read(path).expect("after buffer"), bytes, "creating a buffer must not write a survey");
    let ring = h.app.home_map.real_survey.distance_rings.get(&meters).expect("actual ring").clone();
    assert_eq!((ring.lines.len(), ring.labels.len()), (1, 4), "C++ save fixture produces one boundary and four labels");
    ring
}

fn save_and_reopen(meters: u32, shortcut: bool) {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temporary path");
    let original = records(&h);
    let ring = generated(&mut h, meters);
    if shortcut {
        key(&mut h, Key::S, Modifiers::CTRL);
    } else {
        click(&mut h, "저장");
    }
    assert!(!h.app.is_dirty);
    let tables = strata_storage::survey_gpkg_read::read_survey_tables(&path).expect("read actual saved GeoPackage");
    let matching = |kind: &str, field: &str, value: &str| {
        tables
            .iter()
            .filter(|t| t.geometry_type == kind)
            .flat_map(|t| t.layer.features.iter())
            .filter(|f| f.attributes.get(field).map(String::as_str) == Some(value))
            .count()
    };
    assert_eq!(matching("LINESTRING", "note", &format!("주변 {meters}m")), 1, "explicit Save did not store the buffer boundary");
    assert_eq!(matching("POINT", "label", &format!("{meters}m")), 4, "explicit Save did not store distance labels");
    // target/cpp-ref/tests/test_buffer.cpp:191-198: reopen the saved copy through production open.
    h.app.request_open_survey(path.clone());
    h.frame(vec![]);
    assert!(h.app.save_before.is_none());
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&meters), Some(&ring), "reopen discarded saved buffer coordinates or labels");
    for (table, rows) in original {
        assert_eq!(h.app.survey_tables.iter().find(|t| t.table == table).expect("original table retained").layer.features.as_slice(), rows.as_slice());
    }
    for kind in ["ring", "label"] {
        assert_eq!(
            h.app.left_layers.entries.iter().filter(|e| e.layer_key == format!("user:buffer_{kind}_{meters}")).count(),
            1,
            "saved layer identity lost or duplicated"
        );
    }
    h.app.home_map.viewport.center_xy = [200_020.0, 450_020.0];
    h.app.home_map.viewport.meters_per_pixel = 4.0;
    h.save(&format!("buffer-save-{meters}-{shortcut}-reopened"));
}

fn pending_buffer(meters: u32) {
    let mut h = start();
    let ring = generated(&mut h, meters);
    key(&mut h, Key::N, Modifiers::CTRL);
    h.save(&format!("buffer-save-{meters}-save-before"));
    assert!(h.app.save_before.is_some(), "new survey bypassed the save question for an unsaved buffer");
    click(&mut h, "취소");
    assert!(h.app.save_before.is_none() && h.app.is_dirty);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&meters), Some(&ring));
}

#[test]
fn buffer_save_input_500_ribbon_stores_geometry_and_reopens() {
    save_and_reopen(500, false);
}
#[test]
fn buffer_save_input_1000_ctrl_s_stores_geometry_and_reopens() {
    save_and_reopen(1000, true);
}
#[test]
fn buffer_save_input_500_new_survey_asks_and_cancel_keeps_ring() {
    pending_buffer(500);
}
#[test]
fn buffer_save_input_1000_new_survey_asks_and_cancel_keeps_ring() {
    pending_buffer(1000);
}

#[path = "buffer_save_input_tests_a.rs"]
mod buffer_save_input_tests_a;
