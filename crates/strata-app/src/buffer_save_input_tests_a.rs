//! Save/retry/reopen preservation, target/cpp-ref/src/core/SurveyStorage.cpp:703-786 and test_buffer.cpp:166-198.
use super::{click, generated, key, records, start, Harness, Key, Modifiers};
use strata_types::crs::WorkCrs;

fn stored(path: &std::path::Path) -> Vec<(String, Vec<strata_storage::edit_buffer::FeatureRecord>)> {
    strata_storage::survey_gpkg_read::read_survey_tables(path)
        .expect("saved feature tables")
        .into_iter()
        .map(|t| (t.table, t.layer.features.to_vec()))
        .collect()
}

fn reshape(h: &mut Harness) {
    let source = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("source");
    let fid = source.layer.features[0].fid;
    let wkt = "POLYGON ((200000 450000,200060 450000,200060 450060,200000 450060,200000 450000))".to_owned();
    assert!(source.layer.change_add_delete(&[(fid, wkt)], vec![], &[]).is_some());
    h.app.is_dirty = true;
}

#[test]
fn buffer_save_input_two_radii_repeat_and_replace_without_duplicates() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temp path");
    generated(&mut h, 500);
    generated(&mut h, 1000);
    click(&mut h, "저장");
    let first = stored(&path);
    click(&mut h, "저장");
    assert_eq!(stored(&path), first, "repeated Save duplicated or altered stored feature IDs, coordinates or attributes");
    let old1000 = h.app.home_map.real_survey.distance_rings.get(&1000).expect("1000").clone();
    reshape(&mut h);
    let new500 = generated(&mut h, 500);
    let originals = records(&h);
    key(&mut h, Key::S, Modifiers::CTRL);
    h.app.request_open_survey(path.clone());
    h.frame(vec![]);
    assert_eq!(h.app.home_map.real_survey.distance_rings.len(), 2);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&500), Some(&new500));
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&1000), Some(&old1000));
    assert_eq!(records(&h), originals);
    assert_eq!(stored(&path).len(), 11, "seven original tables plus two line/point pairs only");
    h.app.home_map.viewport.center_xy = [200_030.0, 450_030.0];
    h.app.home_map.viewport.meters_per_pixel = 4.0;
    h.save("buffer-save-two-radii-replaced-reopened");
}

#[test]
fn buffer_save_input_failed_save_retains_previous_file_edits_and_rings_then_retry() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temp path");
    generated(&mut h, 500);
    click(&mut h, "저장");
    let previous = std::fs::read(&path).expect("saved generation");
    reshape(&mut h);
    let replacement = generated(&mut h, 500);
    let line = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_line").expect("unrelated line");
    let fid = line.layer.features[0].fid;
    assert!(line.layer.change_add_delete(&[(fid, "LINESTRING Z (0 0 1,1 1 1)".to_owned())], vec![], &[]).is_some());
    h.app.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table("feature_line".to_owned()));
    let pending = records(&h);
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    click(&mut h, "저장");
    assert!(h.app.is_dirty);
    assert!(h.app.status_left_message().contains("저장 실패"));
    assert_eq!(records(&h), pending);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&500), Some(&replacement));
    assert_eq!(std::fs::read(&path).expect("protected previous generation"), previous);
    h.save("buffer-save-failed-preserved");
    key(&mut h, Key::Z, Modifiers::CTRL);
    key(&mut h, Key::S, Modifiers::CTRL);
    assert!(!h.app.is_dirty);
    h.app.request_open_survey(path);
    h.frame(vec![]);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&500), Some(&replacement));
}

#[test]
fn buffer_save_input_same_name_unowned_table_is_preserved() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temp path");
    let conn = rusqlite::Connection::open(&path).expect("temp copy");
    conn.execute_batch("CREATE TABLE \"주변_500m\" (fid INTEGER PRIMARY KEY,note TEXT); INSERT INTO \"주변_500m\" VALUES (21,'protected original');")
        .expect("prepare an unrelated existing table");
    drop(conn);
    let ring = generated(&mut h, 500);
    click(&mut h, "저장");
    let conn = rusqlite::Connection::open(&path).expect("saved copy");
    let original: (i64, String) = conn.query_row("SELECT fid,note FROM \"주변_500m\"", [], |r| Ok((r.get(0)?, r.get(1)?))).expect("original row still exists");
    assert_eq!(original, (21, "protected original".to_owned()));
    drop(conn);
    h.app.request_open_survey(path);
    h.frame(vec![]);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&500), Some(&ring));
}

#[test]
fn buffer_save_input_work_crs_switch_preserves_creation_crs_and_projects_reopen() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temp path");
    let ring = generated(&mut h, 500);
    click(&mut h, "작업 5187 · 동부원점 ▼");
    assert_eq!(h.app.work_crs, WorkCrs::Epsg5186);
    click(&mut h, "저장");
    let tables = strata_storage::survey_gpkg_read::read_survey_tables(&path).expect("actual saved geometry metadata");
    let own = tables
        .iter()
        .find(|t| t.layer.features.iter().any(|f| f.attributes.get("note").map(String::as_str) == Some("주변 500m")))
        .expect("saved boundary");
    assert_eq!(own.srs_id, 5187, "view toggle relabeled saved coordinates without a conversion");
    let tr = strata_engine::Transformer::new(5187, 5186).expect("explicit known CRS route");
    let expected_lines: Vec<_> = ring.lines.iter().map(|line| tr.transform_all(line).expect("known local route")).collect();
    let expected_labels = tr.transform_all(&ring.labels).expect("known local labels");
    h.app.request_open_survey(path);
    h.frame(vec![]);
    let opened = h.app.home_map.real_survey.distance_rings.get(&500).expect("reopened boundary");
    assert_eq!(opened.crs, WorkCrs::Epsg5186);
    // target/cpp-ref/src/core/BufferAnalysis.cpp:201-206: verify CRS routing, not PROJ's numerical error bound.
    assert_eq!(opened.lines, expected_lines);
    assert_eq!(opened.labels, expected_labels);
    h.app.home_map.viewport.center_xy = tr.transform([200_020.0, 450_020.0]).expect("prepared view");
    h.app.home_map.viewport.meters_per_pixel = 4.0;
    h.save("buffer-save-work-crs-reopened");
}

#[test]
fn buffer_save_input_created_in_work5186_saves_and_reopens_without_relabeling() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temp path");
    click(&mut h, "작업 5187 · 동부원점 ▼");
    let tr = strata_engine::Transformer::new(5187, 5186).expect("explicit work route");
    h.app.home_map.viewport.center_xy = tr.transform([200_020.0, 450_020.0]).expect("prepared local view");
    h.frame(vec![]);
    let original = records(&h);
    let ring = generated(&mut h, 500);
    assert_eq!(ring.crs, WorkCrs::Epsg5186);
    click(&mut h, "저장");
    h.app.request_open_survey(path);
    h.frame(vec![]);
    assert_eq!(records(&h), original);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&500), Some(&ring));
    h.app.home_map.viewport.center_xy = tr.transform([200_020.0, 450_020.0]).expect("prepared view");
    h.app.home_map.viewport.meters_per_pixel = 4.0;
    h.save("buffer-save-created-work5186-reopened");
}

fn reject_bad_open(mode: u8) {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temporary current survey");
    let ring = generated(&mut h, 500);
    click(&mut h, "저장");
    let bad = path.with_file_name("bad-buffer.gpkg");
    std::fs::copy(&path, &bad).expect("corrupt only a separate temp copy");
    let conn = rusqlite::Connection::open(&bad).expect("copied fixture");
    strata_storage::gpkg_sql_functions::register_gpkg_sql_functions(&conn).expect("real spatial functions");
    let table: String = conn
        .query_row("SELECT table_name FROM gpkg_contents WHERE description='ka-hgis:distance-buffer:v1:500:label'", [], |r| r.get(0))
        .expect("own label table");
    match mode {
        0 => {
            conn.execute("UPDATE gpkg_contents SET description=replace(description,'v1:','v99:') WHERE description LIKE 'ka-hgis:distance-buffer:%'", [])
                .expect("unknown version");
        }
        1 => {
            conn.execute("UPDATE gpkg_geometry_columns SET srs_id=0 WHERE table_name=?1", [&table]).expect("unknown CRS metadata");
        }
        2 => {
            let mut blob: Vec<u8> = conn.query_row(&format!("SELECT geom FROM \"{}\" WHERE fid=1", table.replace('"', "\"\"")), [], |r| r.get(0)).expect("real label header");
            blob[4..8].copy_from_slice(&5186_i32.to_le_bytes());
            conn.execute(&format!("UPDATE \"{}\" SET geom=?1 WHERE fid=1", table.replace('"', "\"\"")), [blob]).expect("mismatched CRS header");
        }
        3 => {
            conn.execute(&format!("UPDATE \"{}\" SET label='edited externally' WHERE fid=1", table.replace('"', "\"\"")), [])
                .expect("unrepresentable label attribute");
        }
        _ => {
            conn.execute("UPDATE gpkg_contents SET description='' WHERE table_name=?1", [&table]).expect("incomplete owned pair");
        }
    }
    drop(conn);
    let t = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("original pending record");
    let fid = t.layer.features[0].fid;
    assert!(t.layer.change_attributes(fid, &[("note".to_owned(), Some("pending before failed open".to_owned()))]));
    h.app.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table("feature_poly".to_owned()));
    h.app.is_dirty = true;
    let pending = records(&h);
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    let bytes = std::fs::read(&path).expect("current saved copy");
    h.app.request_open_survey(bad);
    h.frame(vec![]);
    assert!(h.app.save_before.is_some());
    click(&mut h, "저장 안 함");
    h.save(&format!("buffer-save-bad-open-{mode}"));
    assert_eq!(h.app.survey_path.as_deref(), Some(path.as_path()), "malformed saved buffers replaced the currently edited survey");
    assert!(h.app.is_dirty);
    assert_eq!(records(&h), pending);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    assert_eq!(h.app.home_map.real_survey.distance_rings.get(&500), Some(&ring));
    assert_eq!(std::fs::read(path).expect("current survey bytes"), bytes);
    assert!(h.app.status_left_message().contains("조사 열기 실패"));
}

#[test]
fn buffer_save_input_unknown_format_rejects_before_discarding_current_work() {
    reject_bad_open(0);
}
#[test]
fn buffer_save_input_unknown_saved_crs_rejects_before_discarding_current_work() {
    reject_bad_open(1);
}
#[test]
fn buffer_save_input_mismatched_header_crs_rejects_before_discarding_current_work() {
    reject_bad_open(2);
}
#[test]
fn buffer_save_input_changed_label_rejects_before_discarding_current_work() {
    reject_bad_open(3);
}
#[test]
fn buffer_save_input_incomplete_pair_rejects_before_discarding_current_work() {
    reject_bad_open(4);
}
