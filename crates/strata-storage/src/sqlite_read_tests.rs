//! Tests for sidecar-free survey SQLite reading across all read entry points (`ORDER.yaml` ST1).
//! C++ reference: `target/cpp-ref/src/core/SurveyStorage.cpp:371-389` (`validateForOpen`) and
//! `:476-489` (`copySurvey` / `VACUUM INTO`); `target/cpp-ref/src/core/SurveyFacts.cpp:205-230`
//! (`countLayers`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

use crate::edit_buffer::{FeatureRecord, LayerEditBuffer};
use crate::gpkg_repository::{GpkgRepository, SurveyWorkspace};
use crate::gpkg_sql_functions::register_gpkg_sql_functions;
use crate::sqlite_read::open_survey_read_only;
use crate::survey_facts_probe::count_layers;
use crate::survey_gpkg_create::create_cpp_survey_file;
use crate::survey_gpkg_read::{read_survey_tables, validate_survey_for_open};
use crate::survey_gpkg_save::{copy_survey_snapshot, save_survey};

const AREA_WKT: &str = "POLYGON ((200000 500000,200100 500000,200100 500100,200000 500000))";
const POLY_WKT: &str = "POLYGON ((200010 500010,200040 500010,200040 500040,200010 500010))";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileSnap {
    pub name: String,
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub sha256: String,
}

pub(super) fn snapshot_dir(dir: &Path) -> Vec<FileSnap> {
    let mut out: Vec<FileSnap> = std::fs::read_dir(dir)
        .expect("read_dir")
        .map(|entry| {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let meta = entry.metadata().expect("metadata");
            let bytes = std::fs::read(&path).expect("read file bytes");
            let sha256 = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
            FileSnap {
                name: entry.file_name().to_string_lossy().into_owned(),
                len: meta.len(),
                modified: meta.modified().ok(),
                sha256,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Builds a fresh survey GeoPackage in `path` with 1 `survey_area` and 1 `feature_poly` (`주거지`).
pub(super) fn build_base_survey(path: &Path) {
    create_cpp_survey_file(path, WorkCrs::Epsg5187).expect("create survey");
    let mut tables = read_survey_tables(path).expect("read initial tables");
    for t in &mut tables {
        if t.table == "survey_area" {
            t.layer.features.push(FeatureRecord {
                fid: -1,
                wkt_geom: AREA_WKT.to_owned(),
                attributes: BTreeMap::from([("survey_name".to_owned(), "1구역".to_owned())]),
            });
        } else if t.table == "feature_poly" {
            t.layer.features.push(FeatureRecord {
                fid: -1,
                wkt_geom: POLY_WKT.to_owned(),
                attributes: BTreeMap::from([("kind".to_owned(), "주거지".to_owned())]),
            });
        }
    }
    save_survey(path, &tables).expect("save base survey");
}

pub(super) fn append_wal_only_feature(conn: &Connection) {
    register_gpkg_sql_functions(conn).expect("register gpkg sql");
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA wal_autocheckpoint = 0;")
        .expect("enable WAL without autocheckpoint");
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint base pages");
    conn.execute(
        "INSERT INTO feature_poly (geom, kind, period)
         SELECT geom, '수혈', '삼국시대' FROM feature_poly WHERE fid = 1",
        [],
    )
    .expect("insert WAL-only feature");
}

/// Runs `open_survey_read_only`, `validate_survey_for_open`, `read_survey_tables`, `count_layers`,
/// and `copy_survey_snapshot` against `gpkg`, asserting the folder snapshot never changes and
/// `expected_kinds` are read by every entry point.
pub(super) fn assert_all_survey_readers(dir: &Path, gpkg: &Path, expected_kinds: &[&str]) {
    let before = snapshot_dir(dir);
    let expected_features = i32::try_from(expected_kinds.len()).expect("i32 count");

    {
        let conn = open_survey_read_only(gpkg).expect("open_survey_read_only");
        let n: i64 = conn
            .query_row("SELECT count(*) FROM feature_poly", [], |r| r.get(0))
            .expect("query feature_poly");
        assert_eq!(n, i64::from(expected_features));
    }
    assert_eq!(snapshot_dir(dir), before, "open_survey_read_only altered folder");

    assert_eq!(validate_survey_for_open(gpkg), Ok(()));
    assert_eq!(snapshot_dir(dir), before, "validate_survey_for_open altered folder");

    let tables = read_survey_tables(gpkg).expect("read_survey_tables");
    let poly = tables.iter().find(|t| t.table == "feature_poly").expect("feature_poly");
    let kinds: Vec<&str> = poly
        .layer
        .features
        .iter()
        .filter_map(|f| f.attributes.get("kind").map(String::as_str))
        .collect();
    assert_eq!(kinds, expected_kinds);
    assert_eq!(snapshot_dir(dir), before, "read_survey_tables altered folder");

    assert_eq!(count_layers(&gpkg.to_string_lossy()), Some((1, expected_features)));
    assert_eq!(snapshot_dir(dir), before, "count_layers altered folder");

    let snap_out_dir = tempfile::tempdir().expect("snap_out_dir");
    let dst = snap_out_dir.path().join("snapshot.gpkg");
    copy_survey_snapshot(gpkg, &dst).expect("copy_survey_snapshot");
    assert_eq!(snapshot_dir(dir), before, "copy_survey_snapshot altered folder");

    let copied = read_survey_tables(&dst).expect("read snapshot");
    let copied_poly = copied.iter().find(|t| t.table == "feature_poly").expect("copied poly");
    let copied_kinds: Vec<&str> = copied_poly
        .layer
        .features
        .iter()
        .filter_map(|f| f.attributes.get("kind").map(String::as_str))
        .collect();
    assert_eq!(copied_kinds, expected_kinds, "VACUUM INTO snapshot missed rows");
}

#[test]
fn delete_mode_leaves_folder_and_bytes_unchanged_for_all_readers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("delete_mode.gpkg");
    build_base_survey(&gpkg);
    let header = std::fs::read(&gpkg).expect("read header");
    assert_eq!(&header[18..20], &[1, 1]);
    assert_eq!(snapshot_dir(dir.path()).len(), 1);

    assert_all_survey_readers(dir.path(), &gpkg, &["주거지"]);
}

#[test]
fn wal_without_wal_file_leaves_no_sidecars_for_all_readers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("wal_clean.gpkg");
    build_base_survey(&gpkg);
    {
        let conn = Connection::open(&gpkg).expect("open to set WAL");
        conn.pragma_update(None, "journal_mode", "WAL").expect("set WAL");
    }
    let header = std::fs::read(&gpkg).expect("read header");
    assert_eq!(&header[18..20], &[2, 2]);
    let names: Vec<String> = snapshot_dir(dir.path()).into_iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["wal_clean.gpkg"]);

    assert_all_survey_readers(dir.path(), &gpkg, &["주거지"]);
}

#[test]
fn wal_with_wal_only_rows_reads_wal_and_leaves_folder_unchanged_for_all_readers() {
    let build_dir = tempfile::tempdir().expect("build_dir");
    let src = build_dir.path().join("src.gpkg");
    build_base_survey(&src);
    let writer = Connection::open(&src).expect("open writer");
    append_wal_only_feature(&writer);

    let dir = tempfile::tempdir().expect("target_dir");
    let gpkg = dir.path().join("idle_wal.gpkg");
    let wal = PathBuf::from(format!("{}-wal", gpkg.display()));
    std::fs::copy(&src, &gpkg).expect("copy main db");
    std::fs::copy(format!("{}-wal", src.display()), &wal).expect("copy -wal");
    drop(writer);
    drop(build_dir);

    // Prove the main file alone has only 1 feature_poly row (`주거지`).
    let alone_dir = tempfile::tempdir().expect("alone_dir");
    let alone = alone_dir.path().join("alone.gpkg");
    std::fs::copy(&gpkg, &alone).expect("copy main alone");
    assert_eq!(count_layers(&alone.to_string_lossy()), Some((1, 1)));

    let names: Vec<String> = snapshot_dir(dir.path()).into_iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["idle_wal.gpkg", "idle_wal.gpkg-wal"]);

    assert_all_survey_readers(dir.path(), &gpkg, &["주거지", "수혈"]);
}

#[test]
fn wal_held_open_by_live_writer_reads_wal_and_leaves_folder_unchanged_for_all_readers() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("live_wal.gpkg");
    build_base_survey(&gpkg);
    let writer = Connection::open(&gpkg).expect("open live writer");
    append_wal_only_feature(&writer);

    let names: Vec<String> = snapshot_dir(dir.path()).into_iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["live_wal.gpkg", "live_wal.gpkg-shm", "live_wal.gpkg-wal"]);

    assert_all_survey_readers(dir.path(), &gpkg, &["주거지", "수혈"]);
    drop(writer);
}

#[test]
fn gpkg_repository_open_read_only_handles_all_four_modes_without_sidecars() {
    let base_ws = || {
        let mut ws = SurveyWorkspace {
            schema_version: 3,
            work_crs: WorkCrs::Epsg5187,
            layers: BTreeMap::new(),
            embedded_project_json: "{}".to_owned(),
        };
        ws.layers.insert(
            DomainLayerKey::SurveyArea.id().to_owned(),
            LayerEditBuffer::new(DomainLayerKey::SurveyArea),
        );
        ws
    };
    let mut wal_ws = base_ws();
    wal_ws.layers.insert(
        DomainLayerKey::FeaturePoly.id().to_owned(),
        LayerEditBuffer::new(DomainLayerKey::FeaturePoly),
    );
    let wal_json = serde_json::to_string(&wal_ws).expect("serialize wal_ws");

    // 1. DELETE mode & 2. WAL mode without -wal file.
    for (name, wal_mode) in [("repo_del.gpkg", false), ("repo_wal.gpkg", true)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(name);
        GpkgRepository::write_workspace_file(&path, &base_ws()).expect("write ws");
        if wal_mode {
            Connection::open(&path)
                .expect("open")
                .pragma_update(None, "journal_mode", "WAL")
                .expect("set WAL");
        }
        let before = snapshot_dir(dir.path());
        let got = GpkgRepository::open_read_only(&path).expect("open_read_only");
        assert!(got.layers.contains_key("survey_area"));
        assert_eq!(snapshot_dir(dir.path()), before, "{name} altered folder");
    }

    // 3. WAL mode while held open & 4. WAL mode with update only in -wal.
    let live_dir = tempfile::tempdir().expect("live_dir");
    let live_path = live_dir.path().join("repo_live.gpkg");
    GpkgRepository::write_workspace_file(&live_path, &base_ws()).expect("write live ws");
    let writer = Connection::open(&live_path).expect("open writer");
    writer
        .execute_batch("PRAGMA journal_mode = WAL; PRAGMA wal_autocheckpoint = 0;")
        .expect("WAL");
    writer
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint");
    writer
        .execute("UPDATE ka_hgis_workspace_state SET workspace_json = ?1 WHERE id = 1", [&wal_json])
        .expect("update in WAL only");

    let live_before = snapshot_dir(live_dir.path());
    let live_got = GpkgRepository::open_read_only(&live_path).expect("read live repo");
    assert!(live_got.layers.contains_key("feature_poly"), "missed WAL-only workspace update");
    assert_eq!(snapshot_dir(live_dir.path()), live_before, "live repo altered folder");

    let idle_dir = tempfile::tempdir().expect("idle_dir");
    let idle_path = idle_dir.path().join("repo_idle.gpkg");
    std::fs::copy(&live_path, &idle_path).expect("copy main");
    std::fs::copy(
        format!("{}-wal", live_path.display()),
        format!("{}-wal", idle_path.display()),
    )
    .expect("copy wal");
    drop(writer);

    let idle_before = snapshot_dir(idle_dir.path());
    let idle_got = GpkgRepository::open_read_only(&idle_path).expect("read idle repo");
    assert!(idle_got.layers.contains_key("feature_poly"), "missed idle WAL workspace update");
    assert_eq!(snapshot_dir(idle_dir.path()), idle_before, "idle repo altered folder");
}
