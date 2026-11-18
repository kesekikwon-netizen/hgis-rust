//! Binary buffers and old callers, target/cpp-ref/tests/test_buffer.cpp:166-198.
use crate::gpkg::GpkgRepository;
use crate::survey_gpkg_buffers::read_buffers;
use crate::survey_gpkg_read::read_survey_tables;
use crate::survey_gpkg_save::save_survey;
use std::collections::BTreeMap;
use strata_types::buffer::DistanceRing;
use strata_types::crs::WorkCrs;

fn sample(crs: WorkCrs) -> DistanceRing {
    // target/cpp-ref/src/core/BufferAnalysis.cpp:242-277: one line and its independent point-label layer.
    DistanceRing {
        crs,
        lines: vec![vec![[200_000.125, 450_000.25], [200_040.5, 450_040.75]]],
        labels: vec![[200_000.125, 450_000.25], [200_010.25, 450_010.5], [200_020.375, 450_020.75], [200_040.5, 450_040.75]],
    }
}

#[test]
fn buffers_binary_roundtrip_retains_both_crs_values_and_original_feature() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("buffers.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5186).expect("first-party schema");
    let mut tables = read_survey_tables(&path).expect("tables");
    tables.iter_mut().find(|t| t.table == "feature_poly").expect("original").layer.add_unsaved_feature(
        "POLYGON ((200000 450000,200040 450000,200040 450040,200000 450000))".to_owned(),
        BTreeMap::from([("note".to_owned(), "preserved source".to_owned())]),
    );
    let updates = BTreeMap::from([(500, sample(WorkCrs::Epsg5187)), (1000, sample(WorkCrs::Epsg5186))]);
    save_survey(&path, &tables, &updates).expect("atomic generation with source and derived data");
    let (ordinary, decoded) = read_buffers(&path, read_survey_tables(&path).expect("saved tables")).expect("decode both owned pairs");
    // target/cpp-ref/tests/test_buffer.cpp:191-198: serialization has no coordinate operation, so compare exact values.
    assert_eq!(decoded, updates);
    assert_eq!(ordinary.len(), 7);
    let original = ordinary.iter().find(|t| t.table == "feature_poly").expect("original table");
    assert_eq!(original.layer.features.len(), 1);
    assert_eq!(original.layer.features[0].fid, 1);
    assert_eq!(original.layer.features[0].attributes.get("note").map(String::as_str), Some("preserved source"));
    let conn = rusqlite::Connection::open(&path).expect("read actual metadata");
    let (organization, definition): (String, String) = conn
        .query_row("SELECT organization,definition FROM gpkg_spatial_ref_sys WHERE srs_id=5187", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("stored CRS definition");
    assert_eq!(organization, "EPSG");
    assert!(definition.contains("GRS 1980") && definition.contains("5187"));
}

#[test]
fn buffers_empty_updates_from_generic_save_preserve_existing_owned_geometry() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("buffers.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187).expect("schema");
    let updates = BTreeMap::from([(500, sample(WorkCrs::Epsg5187))]);
    save_survey(&path, &read_survey_tables(&path).expect("tables"), &updates).expect("save buffers");
    save_survey(&path, &read_survey_tables(&path).expect("all stored feature tables"), &BTreeMap::new()).expect("generic caller without buffer edits");
    let (_, decoded) = read_buffers(&path, read_survey_tables(&path).expect("saved tables")).expect("still present");
    assert_eq!(decoded, updates);
}

#[test]
fn buffers_failed_generation_keeps_previous_bytes_and_allows_retry() {
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("buffers.gpkg");
    GpkgRepository::create_new_survey(&path, WorkCrs::Epsg5187).expect("schema");
    let tables = read_survey_tables(&path).expect("tables");
    let valid = BTreeMap::from([(500, sample(WorkCrs::Epsg5187))]);
    save_survey(&path, &tables, &valid).expect("first generation");
    let previous = std::fs::read(&path).expect("bytes");
    let mut invalid = valid.clone();
    invalid.get_mut(&500).expect("ring").labels[0][0] = f64::INFINITY;
    let mut pending = tables.clone();
    pending.iter_mut().find(|t| t.table == "feature_poly").expect("source").layer.add_unsaved_feature(
        "POLYGON ((200000 450000,200040 450000,200040 450040,200000 450000))".to_owned(),
        BTreeMap::from([("note".to_owned(), "same-generation source edit".to_owned())]),
    );
    assert!(save_survey(&path, &pending, &invalid).is_err());
    assert_eq!(std::fs::read(&path).expect("previous bytes"), previous);
    save_survey(&path, &pending, &valid).expect("successful retry");
    let (ordinary, decoded) = read_buffers(&path, read_survey_tables(&path).expect("tables")).expect("decode");
    assert_eq!(decoded, valid);
    assert_eq!(ordinary.iter().find(|t| t.table == "feature_poly").expect("source").layer.features.len(), 1);
}
