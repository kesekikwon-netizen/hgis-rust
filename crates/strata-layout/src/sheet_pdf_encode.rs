//! ToUnicode CMap and the PDF file around a sheet's content stream.
//!
//! Text is Identity-H with a character map, so a reader copies the same letters
//! C++ asks Qt to keep as text (`PdfExportSettings.cpp:9-26`, `PreferText`).

use std::collections::BTreeMap;

use super::{Run, VectorSheetPdf};
use crate::sheet_pdf_export::mm_to_pdf_points;

pub(super) fn unicode_map(runs: &[Run], assigned: &BTreeMap<char, u16>) -> (BTreeMap<char, u16>, String) {
    let mut cids = BTreeMap::new();
    let mut spare = assigned.values().copied().max().unwrap_or(0);
    for run in runs {
        for ch in run.text.chars() {
            if let Some(id) = assigned.get(&ch) {
                cids.insert(ch, *id);
            } else {
                spare = spare.saturating_add(1);
                cids.entry(ch).or_insert(spare);
            }
        }
    }
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /MalgunGothic def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let mut pairs: Vec<(u16, char)> = cids.iter().map(|(ch, id)| (*id, *ch)).collect();
    pairs.sort_by_key(|(id, _)| *id);
    for chunk in pairs.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (id, ch) in chunk {
            let mut unit = [0_u16; 2];
            let encoded = ch.encode_utf16(&mut unit);
            let hex: String = encoded.iter().map(|u| format!("{u:04X}")).collect();
            cmap.push_str(&format!("<{id:04X}> <{hex}>\n"));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    (cids, cmap)
}

/// Characters a reader copies from the page text, in paint order.
#[must_use]
pub fn extracted_text(pdf: &[u8]) -> String {
    let mut codes: BTreeMap<u16, String> = BTreeMap::new();
    let mut from = 0;
    while let Some(at) = find_from(pdf, b"beginbfchar\n", from) {
        let mut line = at + b"beginbfchar\n".len();
        while line < pdf.len() && !pdf[line..].starts_with(b"endbfchar") {
            let next = pdf[line..]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(pdf.len(), |n| line + n);
            if let Some((cid, uni)) = bfchar(&pdf[line..next]) {
                codes.insert(cid, uni);
            }
            line = (next + 1).min(pdf.len());
        }
        from = line;
    }
    let mut out = String::new();
    let mut i = 0;
    while i + 4 < pdf.len() {
        if pdf[i] == b'<' && pdf.get(i + 1).is_some_and(u8::is_ascii_hexdigit) {
            if let Some(end) = pdf[i + 1..]
                .iter()
                .position(|b| *b == b'>')
                .map(|n| i + 1 + n)
            {
                if pdf.get(end + 1..end + 4) == Some(b" Tj") {
                    append_tj(&pdf[i + 1..end], &codes, &mut out);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn append_tj(hex: &[u8], codes: &BTreeMap<u16, String>, out: &mut String) {
    let Ok(hex) = std::str::from_utf8(hex) else {
        return;
    };
    for cid in hex.as_bytes().chunks(4) {
        let Ok(text) = std::str::from_utf8(cid) else {
            continue;
        };
        let Ok(id) = u16::from_str_radix(text, 16) else {
            continue;
        };
        if let Some(ch) = codes.get(&id) {
            out.push_str(ch);
        }
    }
}

fn bfchar(line: &[u8]) -> Option<(u16, String)> {
    let text = std::str::from_utf8(line).ok()?;
    let cid_hex = text.split('<').nth(1)?.split('>').next()?;
    let uni_hex = text.split('<').nth(2)?.split('>').next()?;
    let cid = u16::from_str_radix(cid_hex, 16).ok()?;
    let bytes = hex_bytes(uni_hex)?;
    let units: Vec<u16> = bytes
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    Some((cid, String::from_utf16_lossy(&units)))
}

fn hex_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

pub(super) fn stream_obj(body: &str) -> Vec<u8> {
    format!("<< /Length {} >>\nstream\n{body}endstream", body.len()).into_bytes()
}

pub(super) fn page_dict(doc: &VectorSheetPdf<'_>, image: bool, image_obj: usize) -> Vec<u8> {
    let xobject = if image { format!(" /XObject << /Im0 {image_obj} 0 R >>") } else { String::new() };
    format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 5 0 R >>{xobject} >> /Contents 4 0 R >>",
        mm_to_pdf_points(doc.sheet.width_mm),
        mm_to_pdf_points(doc.sheet.height_mm)
    )
    .into_bytes()
}

pub(super) fn image_obj_bytes(jpeg: &[u8], px: [u32; 2]) -> Vec<u8> {
    let mut out = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        px[0], px[1], jpeg.len()
    )
    .into_bytes();
    out.extend_from_slice(jpeg);
    out.extend_from_slice(b"\nendstream");
    out
}

pub(super) fn width_array(widths: &BTreeMap<u16, u16>) -> String {
    let mut out = String::from("/W [");
    for (cid, width) in widths {
        out.push_str(&format!("{cid} [{width}] "));
    }
    out.push(']');
    out
}

pub(super) fn stream_bytes(body: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", body.len()).into_bytes();
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendstream");
    out
}

pub(super) fn pdf_file(objects: &[Vec<u8>], info: usize) -> Vec<u8> {
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut starts = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        starts.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let table = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for start in starts {
        out.extend_from_slice(format!("{start:010} 00000 n \n").as_bytes());
    }
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R /Info {info} 0 R >>\nstartxref\n{table}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|at| at + from)
}
