//! Tests for `survey_user_sheet` corrections (`ORDER.yaml` R2-3b2).
//! C++ truth: `target/cpp-ref/src/app/MainWindowSession.cpp:284-286` and `:323-325`,
//! `target/cpp-ref/src/core/ExportService.cpp:114-115` and `:194-203`,
//! `target/cpp-ref/src/core/LayoutService.cpp:1172-1194`,
//! `target/cpp-ref/src/core/SubmitReadme.cpp:68-91`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Write};
use std::path::Path;

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

use crate::{survey_user_sheet, SheetCheck};

const USER_SHEET_WITH_MAP: &str = concat!(
    "<Layout name=\"user_sheet\" printResolution=\"300\" units=\"mm\" worldFileMap=\"\">",
    "<customproperties><Option type=\"Map\"/></customproperties>",
    "<PageCollection>",
    "<LayoutItem type=\"65638\" uuid=\"{page-1}\"><customproperties/></LayoutItem>",
    "</PageCollection>",
    "<LayoutItem id=\"ka_map\" type=\"65639\" uuid=\"{map-1}\">",
    "<customproperties/>",
    "<Extent xmin=\"200000\" ymin=\"500000\" xmax=\"200100\" ymax=\"500100\"/>",
    "<LayerSet/>",
    "</LayoutItem>",
    "</Layout>"
);

const USER_SHEET_PAGE_AND_LABEL: &str = concat!(
    "<Layout name=\"user_sheet\" printResolution=\"300\" units=\"mm\" worldFileMap=\"\">",
    "<customproperties><Option type=\"Map\"/></customproperties>",
    "<PageCollection>",
    "<LayoutItem type=\"65638\" uuid=\"{page-1}\"><customproperties/></LayoutItem>",
    "</PageCollection>",
    "<LayoutItem id=\"title_label\" type=\"65641\" uuid=\"{lbl-1}\">",
    "<customproperties/>",
    "</LayoutItem>",
    "</Layout>"
);

const SECTION_SHEET_WITH_MAP: &str = concat!(
    "<Layout name=\"section_sheet\" printResolution=\"300\" units=\"mm\" worldFileMap=\"\">",
    "<PageCollection><LayoutItem type=\"65638\"/></PageCollection>",
    "<LayoutItem id=\"ka_section_map\" type=\"65639\">",
    "<Extent xmin=\"0\" ymin=\"0\" xmax=\"10\" ymax=\"10\"/>",
    "<LayerSet/>",
    "</LayoutItem>",
    "</Layout>"
);

fn qgs_xml(layouts_inner: &str) -> String {
    format!(
        "<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>\n\
         <qgis version=\"3.40.0\" projectname=\"survey\">\n\
         <Layouts>{layouts_inner}</Layouts>\n\
         </qgis>\n"
    )
}

fn workspace_zip(layouts_inner: &str) -> Vec<u8> {
    let xml = qgs_xml(layouts_inner);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default();
    zip.start_file("survey.qgs", opts).expect("start survey.qgs");
    zip.write_all(xml.as_bytes()).expect("write survey.qgs");
    zip.finish().expect("finish zip").into_inner()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_file(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).expect("read file"))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sorted_dir_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn create_dummy_gpkg(path: &Path) {
    let conn = Connection::open(path).expect("open dummy gpkg");
    conn.execute_batch("CREATE TABLE dummy(id INTEGER PRIMARY KEY);")
        .expect("create dummy");
}

fn assert_english_err(res: Result<SheetCheck, String>) {
    let reason = res.expect_err("expected Err");
    assert!(!reason.trim().is_empty(), "empty error reason");
    assert!(reason.is_ascii(), "expected English ASCII reason, got: {reason}");
}

#[test]
fn wal_mode_survey_leaves_folder_and_bytes_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("wal_closed.gpkg");
    let hex = hex_encode(&workspace_zip(USER_SHEET_WITH_MAP));
    {
        let conn = Connection::open(&gpkg).expect("open wal gpkg");
        conn.pragma_update(None, "journal_mode", "WAL").expect("set WAL");
        conn.execute_batch(
            "CREATE TABLE qgis_projects(name TEXT PRIMARY KEY, metadata BLOB, content BLOB);",
        )
        .expect("create qgis_projects");
        conn.execute(
            "INSERT INTO qgis_projects(name, metadata, content) VALUES ('survey', '{}', ?1)",
            [&hex],
        )
        .expect("insert survey workspace");
    }
    let header = std::fs::read(&gpkg).expect("read gpkg");
    assert_eq!(&header[18..20], &[2, 2], "fixture must have WAL header bytes 02 02");

    let before_names = sorted_dir_names(dir.path());
    assert_eq!(before_names, vec!["wal_closed.gpkg"]);
    let before_sha = sha256_file(&gpkg);

    let mut perms = std::fs::metadata(&gpkg).expect("meta").permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&gpkg, perms).expect("set readonly");
    let got = survey_user_sheet(&gpkg);

    assert_eq!(
        sorted_dir_names(dir.path()),
        before_names,
        "WAL survey folder must gain no -shm or -wal files"
    );
    assert_eq!(sha256_file(&gpkg), before_sha, "survey bytes must not change");
    assert_eq!(got, Ok(SheetCheck::UserSheetWithMap));
}

#[test]
fn wal_mode_survey_with_present_wal_file_reads_wal_and_leaves_folder_unchanged() {
    let staging = tempfile::tempdir().expect("staging tempdir");
    let staged_gpkg = staging.path().join("staged.gpkg");
    let staged_wal = staging.path().join("staged.gpkg-wal");
    let hex = hex_encode(&workspace_zip(USER_SHEET_WITH_MAP));

    let conn = Connection::open(&staged_gpkg).expect("open staged gpkg");
    conn.pragma_update(None, "journal_mode", "WAL").expect("set WAL");
    conn.pragma_update(None, "wal_autocheckpoint", 0).expect("disable autocheckpoint");
    conn.execute_batch(
        "CREATE TABLE qgis_projects(name TEXT PRIMARY KEY, metadata BLOB, content BLOB);",
    )
    .expect("create qgis_projects");
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint empty table");
    conn.execute(
        "INSERT INTO qgis_projects(name, metadata, content) VALUES ('survey', '{}', ?1)",
        [&hex],
    )
    .expect("insert row into WAL only");

    let live_names = sorted_dir_names(staging.path());
    assert_eq!(live_names, vec!["staged.gpkg", "staged.gpkg-shm", "staged.gpkg-wal"]);
    assert_eq!(survey_user_sheet(&staged_gpkg), Ok(SheetCheck::UserSheetWithMap));
    assert_eq!(sorted_dir_names(staging.path()), live_names);

    let dir = tempfile::tempdir().expect("target tempdir");
    let gpkg = dir.path().join("with_wal.gpkg");
    let wal = dir.path().join("with_wal.gpkg-wal");
    std::fs::copy(&staged_gpkg, &gpkg).expect("copy main db");
    std::fs::copy(&staged_wal, &wal).expect("copy wal file");
    drop(conn);
    drop(staging);

    // Without the -wal file, the main database file alone has no 'survey' row yet.
    let alone_dir = tempfile::tempdir().expect("alone tempdir");
    let alone_gpkg = alone_dir.path().join("alone.gpkg");
    std::fs::copy(&gpkg, &alone_gpkg).expect("copy main db alone");
    assert_eq!(survey_user_sheet(&alone_gpkg), Ok(SheetCheck::NoWorkspace));

    let before_names = sorted_dir_names(dir.path());
    assert_eq!(before_names, vec!["with_wal.gpkg", "with_wal.gpkg-wal"]);
    let before_db_sha = sha256_file(&gpkg);
    let before_wal_sha = sha256_file(&wal);

    let got = survey_user_sheet(&gpkg);
    assert_eq!(
        sorted_dir_names(dir.path()),
        before_names,
        "folder with present -wal must not gain -shm or lose -wal"
    );
    assert_eq!(sha256_file(&gpkg), before_db_sha, "survey bytes must not change");
    assert_eq!(sha256_file(&wal), before_wal_sha, "-wal bytes must not change");
    assert_eq!(got, Ok(SheetCheck::UserSheetWithMap));
}

#[test]
fn user_sheet_with_map_item_65639_returns_user_sheet_with_map() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("with_map.gpkg");
    {
        let conn = Connection::open(&gpkg).expect("open");
        conn.execute_batch(
            "CREATE TABLE qgis_projects(name TEXT PRIMARY KEY, metadata BLOB, content BLOB);",
        )
        .expect("create");
        conn.execute(
            "INSERT INTO qgis_projects(name, metadata, content) VALUES ('survey', '{}', ?1)",
            [hex_encode(&workspace_zip(USER_SHEET_WITH_MAP))],
        )
        .expect("insert");
    }
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithMap));

    // Self-closing <LayoutItem ... type="65639"/> inside user_sheet also counts.
    let empty_item_gpkg = dir.path().join("empty_map_item.gpkg");
    create_dummy_gpkg(&empty_item_gpkg);
    std::fs::write(
        dir.path().join("empty_map_item.qgz"),
        workspace_zip("<Layout name=\"user_sheet\"><LayoutItem id=\"m\" type=\"65639\"/></Layout>"),
    )
    .expect("write qgz");
    assert_eq!(survey_user_sheet(&empty_item_gpkg), Ok(SheetCheck::UserSheetWithMap));
}

#[test]
fn user_sheet_without_map_item_or_map_only_in_other_layout_returns_without_map() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("no_map.gpkg");
    create_dummy_gpkg(&gpkg);
    let qgz = dir.path().join("no_map.qgz");

    std::fs::write(&qgz, workspace_zip(USER_SHEET_PAGE_AND_LABEL)).expect("write page+label");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithoutMap));

    let other_first = format!("{SECTION_SHEET_WITH_MAP}{USER_SHEET_PAGE_AND_LABEL}");
    std::fs::write(&qgz, workspace_zip(&other_first)).expect("write section first");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithoutMap));

    let user_first = format!("{USER_SHEET_PAGE_AND_LABEL}{SECTION_SHEET_WITH_MAP}");
    std::fs::write(&qgz, workspace_zip(&user_first)).expect("write user first");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithoutMap));
}

#[test]
fn companion_qgs_read_when_embedded_and_qgz_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("site.2026.gpkg");
    create_dummy_gpkg(&gpkg);
    let qgs = dir.path().join("site.2026.qgs");

    std::fs::write(&qgs, qgs_xml(USER_SHEET_WITH_MAP)).expect("write qgs with map");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithMap));

    std::fs::write(&qgs, qgs_xml(USER_SHEET_PAGE_AND_LABEL)).expect("write qgs without map");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithoutMap));

    std::fs::write(&qgs, qgs_xml("")).expect("write qgs without sheet");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::NoUserSheet));

    std::fs::write(&qgs, "<qgis><Layouts><Layout name=\"user_sheet\"></Layouts></qgis>")
        .expect("write malformed qgs");
    assert_english_err(survey_user_sheet(&gpkg));
}

#[test]
fn companion_qgz_decides_over_qgs_and_unreadable_qgz_returns_err() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("both_comp.gpkg");
    create_dummy_gpkg(&gpkg);
    let qgz = dir.path().join("both_comp.qgz");
    let qgs = dir.path().join("both_comp.qgs");

    std::fs::write(&qgz, workspace_zip("")).expect("write qgz without sheet");
    std::fs::write(&qgs, qgs_xml(USER_SHEET_WITH_MAP)).expect("write qgs with map");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::NoUserSheet));

    std::fs::write(&qgz, workspace_zip(USER_SHEET_WITH_MAP)).expect("write qgz with map");
    std::fs::write(&qgs, qgs_xml("")).expect("write qgs without sheet");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::UserSheetWithMap));

    std::fs::write(&qgz, b"corrupt qgz bytes").expect("write corrupt qgz");
    std::fs::write(&qgs, qgs_xml(USER_SHEET_WITH_MAP)).expect("write valid qgs");
    assert_english_err(survey_user_sheet(&gpkg));

    std::fs::remove_file(&qgz).expect("remove corrupt qgz");
    std::fs::create_dir(&qgz).expect("create directory at .qgz path");
    assert_english_err(survey_user_sheet(&gpkg));
}

