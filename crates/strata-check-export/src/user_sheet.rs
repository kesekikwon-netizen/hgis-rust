//! Detects whether a survey's saved workspace holds a `user_sheet` print layout and map item.
//!
//! Reads workspaces in the order C++ Strata opens them
//! (`target/cpp-ref/src/app/MainWindowSession.cpp:284-325`):
//! 1. Embedded project `geopackage:<file>?projectName=survey` in `qgis_projects`
//!    (`target/cpp-ref/src/core/SurveyStorage.cpp:345-370`).
//! 2. Companion `<folder>/<stem>.qgz` if that file exists, else `<folder>/<stem>.qgs`
//!    read as plain project XML (`target/cpp-ref/src/app/MainWindowSession.cpp:284-286`
//!    and `:323-325`; `target/cpp-ref/src/core/SurveySession.cpp:37-41`).

use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::Connection;

use strata_storage::sqlite_read::open_survey_read_only;

#[path = "user_sheet_xml.rs"]
mod xml;

use xml::{check_qgs_xml, check_workspace_zip};

/// Result of checking a survey's workspace for the `user_sheet` print layout and map items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetCheck {
    NoWorkspace,
    NoUserSheet,
    UserSheetWithoutMap,
    UserSheetWithMap,
}

/// Checks whether `survey` has a saved workspace (embedded, companion `.qgz`, or companion `.qgs`)
/// containing `<Layout name="user_sheet">` under `<Layouts>` and a map item (`type="65639"`).
///
/// Meaning of each answer for the C++ submit package:
/// - [`SheetCheck::NoWorkspace`]: no embedded `'survey'` project
///   (`target/cpp-ref/src/core/SurveyStorage.cpp:345-370`) and neither `<stem>.qgz` nor
///   `<stem>.qgs` exists (`target/cpp-ref/src/app/MainWindowSession.cpp:284-286` and `:323-325`).
///   `project` is null at `target/cpp-ref/src/core/ExportService.cpp:115` and `:195`, so the
///   submit package has no sheet PDF and no reference-layer lines in `README_submit.txt`.
/// - [`SheetCheck::NoUserSheet`]: a workspace exists, but `<Layouts>` holds no
///   `<Layout name="user_sheet">` print layout (`dynamic_cast<QgsPrintLayout*>` is null at
///   `target/cpp-ref/src/core/LayoutService.cpp:1172-1175` and
///   `target/cpp-ref/src/core/SubmitReadme.cpp:74-75`). The submit package has no sheet PDF
///   (`target/cpp-ref/src/core/ExportService.cpp:114-115` and `:198-200`) and no reference lines
///   (`target/cpp-ref/src/core/ExportService.cpp:203`).
/// - [`SheetCheck::UserSheetWithoutMap`]: `<Layout name="user_sheet">` exists under `<Layouts>`
///   and holds no `<LayoutItem>` with `type="65639"`. A layout without a map item is never a
///   composed sheet (`target/cpp-ref/src/core/LayoutService.cpp:1180-1194`) and yields zero
///   reference lines (`target/cpp-ref/src/core/SubmitReadme.cpp:76-88`), so the C++ submit
///   package still has no sheet PDF and no reference lines.
/// - [`SheetCheck::UserSheetWithMap`]: `<Layout name="user_sheet">` holds at least one map item
///   (`type="65639"`). The C++ submit package may include the sheet PDF when composed
///   (`target/cpp-ref/src/core/ExportService.cpp:114-115` and `:198-200`;
///   `target/cpp-ref/src/core/LayoutService.cpp:1168-1198`) and lists reference layers from every
///   map item of `user_sheet` whether or not the sheet is composed
///   (`target/cpp-ref/src/core/ExportService.cpp:203`;
///   `target/cpp-ref/src/core/SubmitReadme.cpp:68-90`), which the Rust app cannot produce yet.
///
/// # Errors
/// Returns a short English reason when `survey` cannot be read or when a workspace exists
/// (embedded, companion `.qgz`, or companion `.qgs`) and is unreadable or malformed.
pub fn survey_user_sheet(survey: &Path) -> Result<SheetCheck, String> {
    if let Some(embedded) = read_embedded_workspace(survey)? {
        return Ok(embedded);
    }
    let qgz = survey.with_extension("qgz");
    match std::fs::read(&qgz) {
        Ok(bytes) => return check_workspace_zip(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("cannot read companion .qgz ({:?})", e.kind())),
    }
    let qgs = survey.with_extension("qgs");
    match std::fs::read(&qgs) {
        Ok(bytes) => check_qgs_xml(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SheetCheck::NoWorkspace),
        Err(e) => Err(format!("cannot read companion .qgs ({:?})", e.kind())),
    }
}

/// `target/cpp-ref/src/core/SurveyStorage.cpp:345-370` — reads `qgis_projects` row `'survey'`
/// using `strata_storage::sqlite_read::open_survey_read_only` so reading never leaves `-shm` or
/// `-wal` sidecars beside `survey` and works when write access to `survey`'s folder is denied.
fn read_embedded_workspace(survey: &Path) -> Result<Option<SheetCheck>, String> {
    let conn = open_survey_read_only(survey).map_err(|e| e.english_reason())?;
    query_embedded_workspace(&conn)
}

fn query_embedded_workspace(conn: &Connection) -> Result<Option<SheetCheck>, String> {
    let has_table: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'qgis_projects'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("cannot query survey file: {}", sqlite_reason(&e)))?;
    if has_table == 0 {
        return Ok(None);
    }

    let mut stmt = conn
        .prepare("SELECT content FROM qgis_projects WHERE name = 'survey'")
        .map_err(|e| format!("cannot read qgis_projects table: {}", sqlite_reason(&e)))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("cannot query qgis_projects table: {}", sqlite_reason(&e)))?;
    let Some(row) = rows
        .next()
        .map_err(|e| format!("cannot read embedded workspace row: {}", sqlite_reason(&e)))?
    else {
        return Ok(None);
    };

    let content = row
        .get_ref(0)
        .map_err(|e| format!("cannot read embedded workspace content: {}", sqlite_reason(&e)))?;
    let check = match content {
        ValueRef::Text(hex) => {
            let zip_bytes = decode_hex(hex)?;
            check_workspace_zip(&zip_bytes)?
        }
        ValueRef::Blob(bytes) => match decode_hex(bytes) {
            Ok(zip_bytes) => check_workspace_zip(&zip_bytes)?,
            Err(_) => check_workspace_zip(bytes)?,
        },
        ValueRef::Null | ValueRef::Integer(_) | ValueRef::Real(_) => {
            return Err("embedded workspace content is empty or invalid".to_owned());
        }
    };
    Ok(Some(check))
}

fn sqlite_reason(err: &rusqlite::Error) -> String {
    match err {
        rusqlite::Error::SqliteFailure(ffi_err, _) => ffi_err.to_string(),
        rusqlite::Error::InvalidPath(_) => "invalid path".to_owned(),
        _ => "sqlite error".to_owned(),
    }
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn decode_hex(hex: &[u8]) -> Result<Vec<u8>, String> {
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return Err("corrupt hex in embedded workspace".to_owned());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    for pair in hex.chunks_exact(2) {
        let hi = hex_nibble(pair[0]).ok_or_else(|| "corrupt hex in embedded workspace".to_owned())?;
        let lo = hex_nibble(pair[1]).ok_or_else(|| "corrupt hex in embedded workspace".to_owned())?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "user_sheet_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "user_sheet_r2_3b2_tests.rs"]
mod r2_3b2_tests;
