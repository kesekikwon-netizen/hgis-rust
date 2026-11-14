//! `.qgz` archive extraction and `.qgs` XML inspection for `user_sheet` (`ORDER.yaml` R2-3b2).
//! C++ truth: `target/cpp-ref/src/core/LayoutService.cpp:1172-1194` and
//! `target/cpp-ref/src/core/SubmitReadme.cpp:68-90`.

use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::SheetCheck;

pub(super) fn check_workspace_zip(bytes: &[u8]) -> Result<SheetCheck, String> {
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
    check_qgs_xml(&xml)
}

pub(super) fn check_qgs_xml(xml_bytes: &[u8]) -> Result<SheetCheck, String> {
    let xml_bytes = xml_bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(xml_bytes);
    let xml = std::str::from_utf8(xml_bytes)
        .map_err(|e| format!("malformed .qgs XML encoding: {e}"))?;
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut roots = 0usize;
    let mut user_sheet_depth: Option<usize> = None;
    let mut found_user_sheet = false;
    let mut found_map_item = false;

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
                let parent_is_layouts = stack.last().is_some_and(|p| p.as_slice() == b"Layouts");
                match inspect_element(&e, parent_is_layouts, user_sheet_depth.is_some())? {
                    ElementHit::UserSheetLayout => {
                        found_user_sheet = true;
                        if user_sheet_depth.is_none() {
                            user_sheet_depth = Some(stack.len());
                        }
                    }
                    ElementHit::MapItem => found_map_item = true,
                    ElementHit::None => {}
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
                let parent_is_layouts = stack.last().is_some_and(|p| p.as_slice() == b"Layouts");
                match inspect_element(&e, parent_is_layouts, user_sheet_depth.is_some())? {
                    ElementHit::UserSheetLayout => found_user_sheet = true,
                    ElementHit::MapItem => found_map_item = true,
                    ElementHit::None => {}
                }
            }
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err("malformed .qgs XML: unexpected closing tag".to_owned());
                }
                if user_sheet_depth == Some(stack.len()) {
                    user_sheet_depth = None;
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

    if found_map_item {
        Ok(SheetCheck::UserSheetWithMap)
    } else if found_user_sheet {
        Ok(SheetCheck::UserSheetWithoutMap)
    } else {
        Ok(SheetCheck::NoUserSheet)
    }
}

enum ElementHit {
    None,
    UserSheetLayout,
    MapItem,
}

/// Inspects `<Layout name="user_sheet">` under `<Layouts>` and `<LayoutItem type="65639">`
/// (map item; `65638` is a page) anywhere inside `user_sheet`.
fn inspect_element(
    e: &BytesStart<'_>,
    parent_is_layouts: bool,
    in_user_sheet: bool,
) -> Result<ElementHit, String> {
    let is_layout = parent_is_layouts && e.name().as_ref() == b"Layout";
    let is_item = in_user_sheet && e.name().as_ref() == b"LayoutItem";
    let mut hit = ElementHit::None;
    for attr in e.attributes() {
        let attr = attr.map_err(|err| format!("malformed .qgs XML attribute: {err}"))?;
        let val = attr
            .unescape_value()
            .map_err(|err| format!("malformed .qgs XML attribute value: {err}"))?;
        if is_layout && attr.key.as_ref() == b"name" && val == "user_sheet" {
            hit = ElementHit::UserSheetLayout;
        } else if is_item && attr.key.as_ref() == b"type" && val == "65639" {
            hit = ElementHit::MapItem;
        }
    }
    Ok(hit)
}
