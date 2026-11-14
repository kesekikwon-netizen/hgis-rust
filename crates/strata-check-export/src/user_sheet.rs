//! Detects whether a survey's saved workspace holds the `user_sheet` layout.
//!
//! Reads workspaces in the order C++ Strata opens them
//! (`target/cpp-ref/src/app/MainWindowSession.cpp:291-325`):
//! 1. Embedded project `geopackage:<file>?projectName=survey` in `qgis_projects`
//!    (`target/cpp-ref/src/core/SurveyStorage.cpp:345-370`).
//! 2. Companion `<folder>/<stem>.qgz`
//!    (`target/cpp-ref/src/core/SurveySession.cpp:37-41`).
//!
//! C++ adds the sheet PDF and sheet README lines only when `user_sheet` exists
//! (`target/cpp-ref/src/core/ExportService.cpp:114-115` and `:194-214`).

use std::io::{Cursor, Read};
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

/// Result of checking a survey's workspace for the `user_sheet` drawing layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetCheck {
    NoWorkspace,
    NoUserSheet,
    HasUserSheet,
}

/// Checks whether `survey` has a saved workspace containing `<Layout name="user_sheet">`
/// under `<Layouts>` in its `.qgs` XML.
///
/// # Errors
/// Returns a short English reason when the survey cannot be read or when a workspace
/// exists (embedded or companion `.qgz`) and is unreadable or malformed.
pub fn survey_user_sheet(survey: &Path) -> Result<SheetCheck, String> {
    if let Some(embedded) = read_embedded_workspace(survey)? {
        return Ok(embedded);
    }
    let companion = survey.with_extension("qgz");
    match std::fs::read(&companion) {
        Ok(bytes) => check_workspace_zip(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SheetCheck::NoWorkspace),
        Err(e) => Err(format!("cannot read companion .qgz ({:?})", e.kind())),
    }
}

/// `target/cpp-ref/src/core/SurveyStorage.cpp:345-370` — checks `qgis_projects` row `'survey'`
/// via a read-only SQLite connection so `survey` bytes never change.
fn read_embedded_workspace(survey: &Path) -> Result<Option<SheetCheck>, String> {
    let conn = Connection::open_with_flags(
        survey,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("cannot open survey file: {}", sqlite_reason(&e)))?;

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

fn check_workspace_zip(bytes: &[u8]) -> Result<SheetCheck, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("corrupt workspace zip: {e}"))?;
    let mut qgs_bytes: Option<Vec<u8>> = None;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("corrupt workspace zip entry: {e}"))?;
        if !entry.is_dir() && entry.name().to_ascii_lowercase().ends_with(".qgs") {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("corrupt .qgs entry in workspace zip: {e}"))?;
            qgs_bytes = Some(buf);
            break;
        }
    }
    let xml = qgs_bytes.ok_or_else(|| "workspace zip has no .qgs entry".to_owned())?;
    if qgs_has_user_sheet(&xml)? {
        Ok(SheetCheck::HasUserSheet)
    } else {
        Ok(SheetCheck::NoUserSheet)
    }
}

fn qgs_has_user_sheet(xml_bytes: &[u8]) -> Result<bool, String> {
    let xml_bytes = xml_bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(xml_bytes);
    let xml = std::str::from_utf8(xml_bytes)
        .map_err(|e| format!("malformed .qgs XML encoding: {e}"))?;
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut roots = 0usize;
    let mut found = false;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("malformed .qgs XML: {e}"))?;
        match event {
            Event::Start(e) => {
                if stack.is_empty() {
                    roots += 1;
                    if roots > 1 {
                        return Err("malformed .qgs XML: multiple root elements".to_owned());
                    }
                }
                let in_layouts = stack.iter().any(|p| p.as_slice() == b"Layouts");
                if element_is_user_sheet(&e, in_layouts)? {
                    found = true;
                }
                stack.push(e.name().as_ref().to_vec());
            }
            Event::Empty(e) => {
                if stack.is_empty() {
                    roots += 1;
                    if roots > 1 {
                        return Err("malformed .qgs XML: multiple root elements".to_owned());
                    }
                }
                let in_layouts = stack.iter().any(|p| p.as_slice() == b"Layouts");
                if element_is_user_sheet(&e, in_layouts)? {
                    found = true;
                }
            }
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err("malformed .qgs XML: unexpected closing tag".to_owned());
                }
            }
            Event::Text(t) => {
                if stack.is_empty() && !t.as_ref().iter().all(u8::is_ascii_whitespace) {
                    return Err("malformed .qgs XML: text outside root element".to_owned());
                }
            }
            Event::CData(_) => {
                if stack.is_empty() {
                    return Err("malformed .qgs XML: content outside root element".to_owned());
                }
            }
            Event::Decl(_) | Event::DocType(_) => {
                if !stack.is_empty() {
                    return Err("malformed .qgs XML: declaration inside element".to_owned());
                }
            }
            Event::PI(_) | Event::Comment(_) => {}
            Event::Eof => {
                if !stack.is_empty() {
                    return Err("malformed .qgs XML: unclosed element".to_owned());
                }
                if roots != 1 {
                    return Err("malformed .qgs XML: missing root element".to_owned());
                }
                break;
            }
        }
    }
    Ok(found)
}

fn element_is_user_sheet(e: &BytesStart<'_>, in_layouts: bool) -> Result<bool, String> {
    let check_name = in_layouts && e.name().as_ref() == b"Layout";
    let mut matched = false;
    for attr in e.attributes() {
        let attr = attr.map_err(|err| format!("malformed .qgs XML attribute: {err}"))?;
        let val = attr
            .unescape_value()
            .map_err(|err| format!("malformed .qgs XML attribute value: {err}"))?;
        if check_name && attr.key.as_ref() == b"name" && val == "user_sheet" {
            matched = true;
        }
    }
    Ok(matched)
}

#[cfg(test)]
#[path = "user_sheet_tests.rs"]
mod tests;
