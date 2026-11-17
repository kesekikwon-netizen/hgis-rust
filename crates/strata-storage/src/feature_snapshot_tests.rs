//! Snapshot isolation; EPSG:5186 coordinate bytes from target/cpp-ref/tests/test_editing_tools.cpp:161-164.
use super::LayerEditBuffer;
use std::collections::BTreeMap;
use strata_types::schema::DomainLayerKey;

const BEFORE: &str = "POINT (200010 450030)";
const AFTER: &str = "POINT (200090 450030)";

fn buffer() -> LayerEditBuffer {
    let mut buffer = LayerEditBuffer::new(DomainLayerKey::ControlPoints);
    buffer.add_unsaved_feature(BEFORE.to_owned(), BTreeMap::new());
    buffer
}

#[test]
fn feature_snapshot_worker_keeps_original_records_after_the_editor_changes_geometry() {
    let mut buffer = buffer();
    let retained = buffer.snapshot();
    let (ready, wait) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        wait.recv().expect("editor changed geometry");
        retained.records().iter().map(|row| (row.fid, row.wkt_geom.clone())).collect::<Vec<_>>()
    });
    assert!(buffer.change_geometry(-1, AFTER.to_owned()));
    ready.send(()).expect("release reader");
    assert_eq!(worker.join().expect("reader"), [(-1, BEFORE.to_owned())], "worker lost the source revision");
    assert_eq!(buffer.features[0].wkt_geom, AFTER);
}

#[test]
fn feature_snapshot_revision_tracks_direct_edits_without_confusing_old_worker_results() {
    let mut buffer = buffer();
    let first = buffer.snapshot();
    assert!(first.same_revision(&buffer.snapshot()), "unchanged buffer lost its revision identity");
    buffer.features[0].wkt_geom = AFTER.to_owned();
    let second = buffer.snapshot();
    assert!(!first.same_revision(&second), "edited records kept the old result identity");
    assert_eq!(first.records()[0].wkt_geom, BEFORE);
    assert_eq!(second.records()[0].wkt_geom, AFTER);
}

#[test]
fn feature_snapshot_keeps_original_ids_across_promotion_delete_undo_and_redo() {
    let mut buffer = buffer();
    let original = buffer.snapshot();
    buffer.assign_positive_ids_on_commit();
    assert_eq!(buffer.features[0].fid, 1);
    assert!(buffer.delete_feature(1));
    let deleted = buffer.snapshot();
    assert!(buffer.undo());
    let restored = buffer.snapshot();
    assert_eq!(restored.records().iter().map(|row| (row.fid, row.wkt_geom.as_str())).collect::<Vec<_>>(), [(1, BEFORE)]);
    assert!(buffer.redo());
    assert!(buffer.features.is_empty());
    assert!(deleted.records().is_empty());
    assert_eq!(original.records().iter().map(|row| row.fid).collect::<Vec<_>>(), [-1]);
}

#[test]
fn feature_snapshot_reads_legacy_array_wire_and_preserves_edit_history_wire() {
    let json = r#"{"key":null,"visible":true,"features":[{"fid":7,"wkt_geom":"POINT (200010 450030)","attributes":{"code":"original"}}],"next_temp_id":-1,"undo_stack":[[]],"redo_stack":[]}"#;
    let mut buffer: LayerEditBuffer = serde_json::from_str(json).expect("existing Vec wire format");
    let retained = buffer.snapshot();
    assert_eq!(retained.records().iter().map(|row| (row.fid, row.wkt_geom.as_str())).collect::<Vec<_>>(), [(7, BEFORE)]);
    assert!(buffer.undo() && buffer.redo());
    let encoded = serde_json::to_value(&buffer).expect("same wire shape");
    assert_eq!(encoded, serde_json::from_str::<serde_json::Value>(json).expect("literal old wire"));
}

#[test]
#[ignore = "manual acquisition-and-drop timing; no timing threshold"]
fn feature_snapshot_measure_acquisition_against_deep_copy() {
    use std::hint::black_box;
    use std::time::Instant;
    for count in [1_000, 100_000] {
        let rows = (0..count)
            .map(|fid| super::FeatureRecord { fid, wkt_geom: BEFORE.to_owned(), attributes: BTreeMap::from([("code".to_owned(), "synthetic benchmark".to_owned())]) })
            .collect::<Vec<_>>();
        let mut buffer = LayerEditBuffer::new(DomainLayerKey::ControlPoints);
        buffer.features = rows.into();
        assert_eq!(buffer.snapshot().records(), buffer.features.as_slice());
        let mut deep = Vec::new();
        let mut shared = Vec::new();
        for _ in 0..21 {
            let start = Instant::now();
            for _ in 0..5 {
                drop(black_box(black_box(&buffer).features.to_vec()));
            }
            deep.push(start.elapsed().as_nanos() / 5);
            let start = Instant::now();
            for _ in 0..10_000 {
                drop(black_box(black_box(&buffer).snapshot()));
            }
            shared.push(start.elapsed().as_nanos() / 10_000);
        }
        deep.sort_unstable();
        shared.sort_unstable();
        println!(
            "rows={count}; test-profile opt-level=1 acquisition+drop ns/op; 21 samples; deep min/median/max={}/{}/{}; shared min/median/max={}/{}/{}",
            deep[0], deep[10], deep[20], shared[0], shared[10], shared[20]
        );
    }
}
