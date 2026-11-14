//! Tests for `survey_user_sheet` (`ORDER.yaml` R2-3b).
//! C++ truth: `target/cpp-ref/src/core/SurveyStorage.cpp:345-370`,
//! `target/cpp-ref/src/core/SurveySession.cpp:37-41`,
//! `target/cpp-ref/src/app/MainWindowSession.cpp:291-325`,
//! `target/cpp-ref/src/core/ExportService.cpp:114-115` and `:194-214`.
//! Ported scenarios: `target/cpp-ref/tests/test_workflow.cpp:7935-8010`,
//! `target/cpp-ref/tests/test_storage_safety.cpp:794-820`,
//! `target/cpp-ref/tests/test_save_open.cpp:4408-4410`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

use crate::{survey_user_sheet, SheetCheck};

fn qgs_xml(layouts_inner: &str) -> String {
    format!(
        "<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>\n\
         <qgis version=\"3.40.0\" projectname=\"survey\">\n\
         <Layouts>{layouts_inner}</Layouts>\n\
         </qgis>\n"
    )
}

fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default();
    for (name, data) in entries {
        zip.start_file(*name, opts).expect("start zip entry");
        zip.write_all(data).expect("write zip entry");
    }
    zip.finish().expect("finish zip").into_inner()
}

fn workspace_zip(layouts_inner: &str) -> Vec<u8> {
    let xml = qgs_xml(layouts_inner);
    build_zip(&[("survey.qgs", xml.as_bytes()), ("survey_styles.db", b"styles")])
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

fn create_qgis_projects_db(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("create sqlite db");
    conn.execute_batch("CREATE TABLE qgis_projects(name TEXT PRIMARY KEY, metadata BLOB, content BLOB);")
        .expect("create qgis_projects");
    conn
}

fn insert_text_project(path: &Path, row_name: &str, content: &str) {
    let conn = create_qgis_projects_db(path);
    conn.execute(
        "INSERT INTO qgis_projects(name, metadata, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![row_name, "{\"user\":\"tester\"}", content],
    )
    .expect("insert project row");
}

fn insert_blob_project(path: &Path, content: &[u8]) {
    let conn = create_qgis_projects_db(path);
    conn.execute(
        "INSERT INTO qgis_projects(name, metadata, content) VALUES ('survey', X'', ?1)",
        [content],
    )
    .expect("insert blob project row");
}

fn assert_english_err(res: Result<SheetCheck, String>) {
    let reason = res.expect_err("expected Err");
    assert!(!reason.trim().is_empty(), "empty error reason");
    assert!(reason.is_ascii(), "expected English ASCII reason, got: {reason}");
}

#[test]
fn embedded_user_sheet_returns_has_user_sheet_and_keeps_bytes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("survey.gpkg");
    let hex = hex_encode(&workspace_zip(
        "<Layout name=\"user_sheet\" units=\"mm\"><PageCollection/></Layout>",
    ));
    assert!(hex.starts_with("504b0304"));
    insert_text_project(&gpkg, "survey", &hex);

    let before = sha256_file(&gpkg);
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::HasUserSheet));
    assert_eq!(sha256_file(&gpkg), before);

    // Self-closing <Layout name="user_sheet" .../> is also recognized.
    let gpkg_empty_tag = dir.path().join("empty_tag.gpkg");
    let hex_empty = hex_encode(&workspace_zip("<Layout name=\"user_sheet\" units=\"mm\"/>"));
    insert_text_project(&gpkg_empty_tag, "survey", &hex_empty);
    assert_eq!(survey_user_sheet(&gpkg_empty_tag), Ok(SheetCheck::HasUserSheet));
}

#[test]
fn embedded_without_user_sheet_returns_no_user_sheet() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg_none = dir.path().join("no_layouts.gpkg");
    insert_text_project(&gpkg_none, "survey", &hex_encode(&workspace_zip("")));
    assert_eq!(survey_user_sheet(&gpkg_none), Ok(SheetCheck::NoUserSheet));

    // Ported from target/cpp-ref/tests/test_workflow.cpp:7969-8009: section_sheet without user_sheet.
    let gpkg_sec = dir.path().join("section_only.gpkg");
    let sec_hex = hex_encode(&workspace_zip(
        "<Layout name=\"section_sheet\" units=\"mm\"><LayoutItem id=\"ka_section_map\"/></Layout>",
    ));
    insert_text_project(&gpkg_sec, "survey", &sec_hex);
    assert_eq!(survey_user_sheet(&gpkg_sec), Ok(SheetCheck::NoUserSheet));

    // <Layout name="user_sheet"> outside <Layouts> is not a project layout sheet.
    let gpkg_outside = dir.path().join("outside_layouts.gpkg");
    let outside_xml = "<qgis version=\"3.40.0\"><Layout name=\"user_sheet\"/><Layouts/></qgis>";
    let outside_zip = build_zip(&[("survey.qgs", outside_xml.as_bytes())]);
    insert_text_project(&gpkg_outside, "survey", &hex_encode(&outside_zip));
    assert_eq!(survey_user_sheet(&gpkg_outside), Ok(SheetCheck::NoUserSheet));
}

#[test]
fn no_workspace_when_table_or_row_and_companion_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let no_table = dir.path().join("no_table.gpkg");
    {
        let conn = Connection::open(&no_table).expect("open");
        conn.execute_batch("CREATE TABLE other_table(id INTEGER PRIMARY KEY);")
            .expect("create other_table");
    }
    assert_eq!(survey_user_sheet(&no_table), Ok(SheetCheck::NoWorkspace));

    let empty_table = dir.path().join("empty_table.gpkg");
    drop(create_qgis_projects_db(&empty_table));
    assert_eq!(survey_user_sheet(&empty_table), Ok(SheetCheck::NoWorkspace));

    let other_row = dir.path().join("other_row.gpkg");
    let hex = hex_encode(&workspace_zip("<Layout name=\"user_sheet\"/>"));
    insert_text_project(&other_row, "other", &hex);
    assert_eq!(survey_user_sheet(&other_row), Ok(SheetCheck::NoWorkspace));
}

#[test]
fn demo_survey_copy_has_no_workspace_and_bytes_unchanged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let copy = dir.path().join("demo.gpkg");
    std::fs::copy(&src, &copy).expect("copy demo.gpkg");

    let before = sha256_file(&copy);
    assert_eq!(survey_user_sheet(&copy), Ok(SheetCheck::NoWorkspace));
    assert_eq!(sha256_file(&copy), before);

    let entries: Vec<_> = std::fs::read_dir(dir.path()).expect("read_dir").collect();
    assert_eq!(entries.len(), 1, "read-only open must not leave sidecar files");
}

#[test]
fn companion_qgz_checked_when_embedded_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("site.2026.gpkg");
    {
        let conn = Connection::open(&gpkg).expect("open");
        conn.execute_batch("CREATE TABLE dummy(id INTEGER);").expect("create dummy");
    }
    // SurveySession.cpp:37-41: completeBaseName() + ".qgz".
    let qgz = dir.path().join("site.2026.qgz");
    std::fs::write(&qgz, workspace_zip("<Layout name=\"user_sheet\" units=\"mm\"/>"))
        .expect("write companion with sheet");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::HasUserSheet));

    std::fs::write(&qgz, workspace_zip("")).expect("write companion without sheet");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::NoUserSheet));
}

#[test]
fn embedded_project_decides_when_companion_also_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gpkg = dir.path().join("both.gpkg");
    let qgz = dir.path().join("both.qgz");

    insert_text_project(&gpkg, "survey", &hex_encode(&workspace_zip("")));
    std::fs::write(&qgz, workspace_zip("<Layout name=\"user_sheet\"/>"))
        .expect("write companion with sheet");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::NoUserSheet));

    {
        let conn = Connection::open(&gpkg).expect("open");
        conn.execute(
            "UPDATE qgis_projects SET content = ?1 WHERE name = 'survey'",
            [hex_encode(&workspace_zip("<Layout name=\"user_sheet\"/>"))],
        )
        .expect("update embedded with sheet");
    }
    std::fs::write(&qgz, workspace_zip("")).expect("write companion without sheet");
    assert_eq!(survey_user_sheet(&gpkg), Ok(SheetCheck::HasUserSheet));

    // Embedded is corrupt while companion is valid -> embedded still decides (Err).
    {
        let conn = Connection::open(&gpkg).expect("open");
        conn.execute("UPDATE qgis_projects SET content = 'not-hex' WHERE name = 'survey'", [])
            .expect("corrupt embedded");
    }
    assert_english_err(survey_user_sheet(&gpkg));
}

#[test]
fn blob_content_reads_hex_text_first_then_raw_zip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let zip_with = workspace_zip("<Layout name=\"user_sheet\"/>");
    let zip_without = workspace_zip("");

    let gpkg_hex_blob = dir.path().join("hex_blob.gpkg");
    insert_blob_project(&gpkg_hex_blob, hex_encode(&zip_with).to_ascii_uppercase().as_bytes());
    assert_eq!(survey_user_sheet(&gpkg_hex_blob), Ok(SheetCheck::HasUserSheet));

    let gpkg_hex_none = dir.path().join("hex_none.gpkg");
    insert_blob_project(&gpkg_hex_none, hex_encode(&zip_without).as_bytes());
    assert_eq!(survey_user_sheet(&gpkg_hex_none), Ok(SheetCheck::NoUserSheet));

    let gpkg_raw_with = dir.path().join("raw_with.gpkg");
    insert_blob_project(&gpkg_raw_with, &zip_with);
    assert_eq!(survey_user_sheet(&gpkg_raw_with), Ok(SheetCheck::HasUserSheet));

    let gpkg_raw_without = dir.path().join("raw_without.gpkg");
    insert_blob_project(&gpkg_raw_without, &zip_without);
    assert_eq!(survey_user_sheet(&gpkg_raw_without), Ok(SheetCheck::NoUserSheet));
}

#[test]
fn corrupt_hex_zip_missing_qgs_and_malformed_xml_return_err() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_english_err(survey_user_sheet(&dir.path().join("missing.gpkg")));

    let gpkg = dir.path().join("bad.gpkg");
    insert_text_project(&gpkg, "survey", "504b03zz");
    assert_english_err(survey_user_sheet(&gpkg));

    let set_text = |val: &str| {
        let conn = Connection::open(&gpkg).expect("open");
        conn.execute("UPDATE qgis_projects SET content = ?1 WHERE name = 'survey'", [val])
            .expect("update text");
    };
    for bad in ["", "504b0", "deadbeef"] {
        set_text(bad);
        assert_english_err(survey_user_sheet(&gpkg));
    }

    let no_qgs = build_zip(&[("survey_styles.db", b"only styles")]);
    set_text(&hex_encode(&no_qgs));
    assert_english_err(survey_user_sheet(&gpkg));

    for bad_xml in [
        "",
        "plain text without tags",
        "<qgis><Layouts><Layout name=\"user_sheet\"></Layouts></qgis>",
        "<qgis><Layouts><Layout name=\"user_sheet\">",
        "<qgis broken=\"attr><Layouts/></qgis>",
    ] {
        let z = build_zip(&[("survey.qgs", bad_xml.as_bytes())]);
        set_text(&hex_encode(&z));
        assert_english_err(survey_user_sheet(&gpkg));
    }

    let comp_gpkg = dir.path().join("comp_bad.gpkg");
    drop(create_qgis_projects_db(&comp_gpkg));
    std::fs::write(dir.path().join("comp_bad.qgz"), b"not a zip").expect("write bad qgz");
    assert_english_err(survey_user_sheet(&comp_gpkg));
}
