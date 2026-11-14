//! Access-denied ACL, hot journal, URI escaping, temp cleanup, and disk-full tests (`ORDER.yaml` ST1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

use super::sqlite_read_tests::{
    append_wal_only_feature, assert_all_survey_readers, build_base_survey, snapshot_dir,
};
use crate::edit_buffer::LayerEditBuffer;
use crate::gpkg_repository::{GpkgRepository, SurveyWorkspace};
use crate::gpkg_sql_functions::register_gpkg_sql_functions;
use crate::sqlite_read::{
    classify_stage_io_error, open_survey_read_only, open_survey_read_only_with_temp_budget,
    sqlite_immutable_uri, SqliteReadError, TEMP_SPACE_FULL_KO,
};
use crate::survey_gpkg_read::{read_survey_tables, validate_survey_for_open};

#[cfg(windows)]
struct DenyFolderWriteGuard {
    dir: PathBuf,
}

#[cfg(windows)]
impl DenyFolderWriteGuard {
    /// Denies file creation, write, append, attribute write, and deletion on `dir` without admin
    /// privileges using well-known SID `*S-1-1-0` (`Everyone`), keeping `SYNCHRONIZE` and read.
    fn apply(dir: &Path) -> Self {
        let status = std::process::Command::new("icacls")
            .arg(dir)
            .args(["/deny", "*S-1-1-0:(OI)(CI)(WD,AD,WEA,WA,DC,DE)"])
            .output()
            .expect("run icacls /deny");
        assert!(status.status.success(), "icacls /deny failed: {status:?}");
        Self {
            dir: dir.to_path_buf(),
        }
    }
}

#[cfg(windows)]
impl Drop for DenyFolderWriteGuard {
    fn drop(&mut self) {
        let _ = std::process::Command::new("icacls")
            .arg(&self.dir)
            .args(["/remove:d", "*S-1-1-0"])
            .output();
    }
}

#[test]
#[cfg(windows)]
fn write_denied_folder_opens_wal_and_delete_surveys_for_all_readers() {
    let build_dir = tempfile::tempdir().expect("build_dir");
    let src_wal = build_dir.path().join("src_wal.gpkg");
    build_base_survey(&src_wal);
    let writer = Connection::open(&src_wal).expect("open writer");
    append_wal_only_feature(&writer);

    let locked_dir = tempfile::tempdir().expect("locked_dir");
    let del_gpkg = locked_dir.path().join("delete.gpkg");
    build_base_survey(&del_gpkg);

    let wal_clean = locked_dir.path().join("wal_clean.gpkg");
    build_base_survey(&wal_clean);
    Connection::open(&wal_clean)
        .expect("open wal_clean")
        .pragma_update(None, "journal_mode", "WAL")
        .expect("set WAL");

    let wal_with_rows = locked_dir.path().join("wal_rows.gpkg");
    std::fs::copy(&src_wal, &wal_with_rows).expect("copy wal main");
    std::fs::copy(
        format!("{}-wal", src_wal.display()),
        format!("{}-wal", wal_with_rows.display()),
    )
    .expect("copy wal companion");
    drop(writer);
    drop(build_dir);

    let repo_wal = locked_dir.path().join("repo_wal.gpkg");
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
    GpkgRepository::write_workspace_file(&repo_wal, &ws).expect("write repo");
    Connection::open(&repo_wal)
        .expect("open repo")
        .pragma_update(None, "journal_mode", "WAL")
        .expect("set repo WAL");

    let _acl = DenyFolderWriteGuard::apply(locked_dir.path());
    let probe_err = std::fs::File::create(locked_dir.path().join("must_fail.tmp"))
        .expect_err("ACL must block file creation in locked_dir");
    assert_eq!(probe_err.kind(), std::io::ErrorKind::PermissionDenied);

    let before = snapshot_dir(locked_dir.path());
    assert_all_survey_readers(locked_dir.path(), &del_gpkg, &["주거지"]);
    assert_all_survey_readers(locked_dir.path(), &wal_clean, &["주거지"]);
    assert_all_survey_readers(locked_dir.path(), &wal_with_rows, &["주거지", "수혈"]);

    let got_repo = GpkgRepository::open_read_only(&repo_wal).expect("open repo in locked folder");
    assert!(got_repo.layers.contains_key("survey_area"));
    assert_eq!(snapshot_dir(locked_dir.path()), before);
}

#[test]
fn hot_rollback_journal_reads_recovered_state_without_modifying_folder() {
    let build_dir = tempfile::tempdir().expect("build_dir");
    let src = build_dir.path().join("hot.gpkg");
    build_base_survey(&src);

    let writer = Connection::open(&src).expect("open writer");
    register_gpkg_sql_functions(&writer).expect("register gpkg sql");
    writer
        .execute_batch(
            "PRAGMA journal_mode = DELETE;
             PRAGMA cache_size = 1;
             BEGIN EXCLUSIVE;
             UPDATE feature_poly SET kind = '미완료편집' WHERE fid = 1;",
        )
        .expect("begin uncommitted write");
    // Flush dirty pages to main db while keeping rollback journal hot.
    writer
        .execute_batch(
            "CREATE TABLE pad_pages (id INTEGER PRIMARY KEY, body BLOB);
             INSERT INTO pad_pages VALUES (1, zeroblob(32768));",
        )
        .expect("spill dirty pages");

    let src_journal = PathBuf::from(format!("{}-journal", src.display()));
    assert!(src_journal.exists(), "hot rollback journal must exist during spilled tx");

    let dir = tempfile::tempdir().expect("target_dir");
    let gpkg = dir.path().join("crashed.gpkg");
    std::fs::copy(&src, &gpkg).expect("copy dirty db");
    std::fs::copy(&src_journal, format!("{}-journal", gpkg.display())).expect("copy hot journal");
    drop(writer);
    drop(build_dir);

    assert_all_survey_readers(dir.path(), &gpkg, &["주거지"]);
}

#[test]
fn paths_with_korean_spaces_and_uri_meta_chars_open_in_wal_mode_without_sidecars() {
    let root = tempfile::tempdir().expect("tempdir");
    let sub = root.path().join("경주 조사 #1 (100% 사본)");
    std::fs::create_dir_all(&sub).expect("create sub");
    let gpkg = sub.join("현장 조사 #2 50%.gpkg");
    build_base_survey(&gpkg);
    Connection::open(&gpkg)
        .expect("open")
        .pragma_update(None, "journal_mode", "WAL")
        .expect("set WAL");

    let conn = open_survey_read_only(&gpkg).expect("open immutable URI");
    assert!(
        conn.staging_dir_path().is_none(),
        "WAL survey without -wal file must open in place without staging copy"
    );
    drop(conn);

    assert_all_survey_readers(&sub, &gpkg, &["주거지"]);
}

#[test]
fn temp_copies_are_removed_after_success_error_and_caller_panic() {
    let build_dir = tempfile::tempdir().expect("build_dir");
    let src = build_dir.path().join("src.gpkg");
    build_base_survey(&src);
    let writer = Connection::open(&src).expect("open writer");
    append_wal_only_feature(&writer);

    let dir = tempfile::tempdir().expect("dir");
    let gpkg = dir.path().join("staged.gpkg");
    std::fs::copy(&src, &gpkg).expect("copy main");
    std::fs::copy(format!("{}-wal", src.display()), format!("{}-wal", gpkg.display()))
        .expect("copy wal");
    drop(writer);
    drop(build_dir);

    // 1. Removed after success.
    let conn = open_survey_read_only(&gpkg).expect("open staged");
    let success_stage = conn.staging_dir_path().expect("must stage when -wal exists").to_path_buf();
    assert!(success_stage.exists());
    let n: i64 = conn
        .query_row("SELECT count(*) FROM feature_poly", [], |r| r.get(0))
        .expect("count");
    assert_eq!(n, 2);
    drop(conn);
    assert!(!success_stage.exists(), "staging folder must be gone after normal drop");

    // 2. Removed after an open error on corrupt file with companion -wal.
    let bad_dir = tempfile::tempdir().expect("bad_dir");
    let bad_gpkg = bad_dir.path().join("corrupt.gpkg");
    std::fs::write(&bad_gpkg, b"not a valid sqlite header at all").expect("write bad main");
    std::fs::write(format!("{}-wal", bad_gpkg.display()), b"bad wal").expect("write bad wal");
    let mut err_stage = None;
    let bad_conn = open_survey_read_only_with_temp_budget(&bad_gpkg, None, Some(&mut err_stage))
        .expect("sqlite opens handle lazily");
    let recorded_err_stage = err_stage.expect("staging path recorded");
    assert!(recorded_err_stage.exists());
    let query_err = bad_conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0));
    assert!(query_err.is_err());
    drop(bad_conn);
    assert!(!recorded_err_stage.exists(), "staging folder must be gone after query error");

    // 3. Removed after a panic in the caller while holding both connection and statement.
    let panic_gpkg = gpkg.clone();
    let mut panic_stage = PathBuf::new();
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let conn = open_survey_read_only(&panic_gpkg).expect("open before panic");
        panic_stage = conn.staging_dir_path().expect("staged").to_path_buf();
        let mut stmt = conn.prepare("SELECT fid FROM feature_poly").expect("prepare");
        let _rows = stmt.query([]).expect("query");
        assert!(panic_stage.exists());
        panic!("simulated caller panic");
    }));
    assert!(panicked.is_err());
    assert!(!panic_stage.exists(), "staging folder must be gone after caller panic");
}

#[test]
fn temp_drive_out_of_space_returns_korean_message_and_cleans_up() {
    let build_dir = tempfile::tempdir().expect("build_dir");
    let src = build_dir.path().join("src.gpkg");
    build_base_survey(&src);
    let writer = Connection::open(&src).expect("open writer");
    append_wal_only_feature(&writer);

    let mut stage_dir = None;
    let err = open_survey_read_only_with_temp_budget(&src, Some(64), Some(&mut stage_dir))
        .expect_err("budget smaller than survey must fail with TempSpaceFull");
    assert!(matches!(err, SqliteReadError::TempSpaceFull), "got {err:?}");
    assert_eq!(err.to_string(), TEMP_SPACE_FULL_KO);
    assert!(err.english_reason().is_ascii());
    if let Some(created) = stage_dir {
        assert!(!created.exists(), "temp dir created before space error must be removed");
    }

    for os_err in [
        std::io::Error::from(std::io::ErrorKind::StorageFull),
        std::io::Error::from_raw_os_error(39),
        std::io::Error::from_raw_os_error(112),
    ] {
        let mapped = classify_stage_io_error(os_err);
        assert!(matches!(mapped, SqliteReadError::TempSpaceFull));
        assert_eq!(mapped.to_string(), TEMP_SPACE_FULL_KO);
    }

    // Verify UNC and extended-length URI escaping for ?immutable=1.
    assert_eq!(
        sqlite_immutable_uri(Path::new(r"\\server\share\경주 #1\a?b%c.gpkg")).expect("unc uri"),
        "file:////server/share/경주%20%231/a%3Fb%25c.gpkg?immutable=1"
    );
    assert_eq!(
        sqlite_immutable_uri(Path::new(r"\\?\C:\dir\survey #1.gpkg")).expect("verbatim uri"),
        "file:///C:/dir/survey%20%231.gpkg?immutable=1"
    );

    // Verify validate_survey_for_open keeps its C++ messages on invalid/damaged files.
    let non_gpkg = build_dir.path().join("plain.txt");
    std::fs::write(&non_gpkg, b"plain").expect("write plain");
    assert_eq!(
        validate_survey_for_open(&non_gpkg),
        Err("읽을 수 있는 GeoPackage 조사 파일이 아닙니다.".to_owned())
    );
    let _ = read_survey_tables(&src);
    drop(writer);
}
