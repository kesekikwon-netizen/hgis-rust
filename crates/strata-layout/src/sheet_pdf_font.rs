//! A subset TrueType of the characters a sheet PDF draws (`LayoutService.cpp:394`).
//! Malgun Gothic when the machine has it, else Noto Sans KR, else IBM Plex Sans KR.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(crate) struct EmbeddedFace {
    pub family: String,
    pub ttf: Vec<u8>,
    pub cid: BTreeMap<char, u16>,
    pub width: BTreeMap<u16, u16>,
}

pub(crate) fn embed_face(chars: impl Iterator<Item = char>) -> Option<EmbeddedFace> {
    let (family, bytes) = load_face()?;
    let needed: Vec<char> = chars.collect();
    let Subset { ttf, cid, width } = subset(&bytes, &needed)?;
    Some(EmbeddedFace { family, ttf, cid, width })
}

fn load_face() -> Option<(String, Vec<u8>)> {
    let plex = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fonts/IBMPlexSansKR-Regular.ttf");
    let noto = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/fonts/NotoSansKR-Regular.ttf");
    for (family, path) in [
        ("Malgun Gothic", std::path::PathBuf::from(r"C:\Windows\Fonts\malgun.ttf")),
        ("Noto Sans KR", noto),
        ("IBM Plex Sans KR", plex),
    ] {
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() > 12 {
                return Some((family.to_owned(), bytes));
            }
        }
    }
    None
}

struct Subset {
    ttf: Vec<u8>,
    cid: BTreeMap<char, u16>,
    width: BTreeMap<u16, u16>,
}

fn subset(font: &[u8], chars: &[char]) -> Option<Subset> {
    let face = ttf_parser::Face::parse(font, 0).ok()?;
    let raw = face.raw_face();
    let glyf = raw.table(tag(b"glyf"))?;
    let loca = raw.table(tag(b"loca"))?;
    let head = raw.table(tag(b"head"))?;
    let hhea = raw.table(tag(b"hhea"))?;
    let hmtx = raw.table(tag(b"hmtx"))?;
    let maxp = raw.table(tag(b"maxp"))?;
    if head.len() < 54 || hhea.len() < 36 {
        return None;
    }
    let long = u16be(head, 50)? == 1;
    let h_metrics = u16be(hhea, 34)? as usize;
    let mut source: Vec<u16> = vec![0];
    let mut seen = BTreeSet::from([0_u16]);
    let mut want: Vec<(char, u16)> = Vec::new();
    for ch in chars {
        let gid = face.glyph_index(*ch).map(|id| id.0).unwrap_or(0);
        want.push((*ch, gid));
        collect(gid, glyf, loca, long, &mut source, &mut seen, 0);
    }
    let mut order: BTreeMap<u16, u16> = BTreeMap::new();
    for (new, old) in source.iter().enumerate() {
        order.insert(*old, u16::try_from(new).ok()?);
    }
    let mut cid = BTreeMap::new();
    let mut width = BTreeMap::new();
    let units = u16be(head, 18)?.max(1);
    for (ch, old) in want {
        let new = *order.get(&old).unwrap_or(&0);
        cid.insert(ch, new);
        let advance = hmtx_advance(hmtx, h_metrics, old as usize);
        let w = u32::from(advance) * 1000 / u32::from(units);
        width.insert(new, u16::try_from(w).unwrap_or(1000));
    }
    let mut glyphs = Vec::new();
    for old in &source {
        let (start, end) = span(loca, long, *old as usize).ok()?;
        let mut blob = glyf.get(start..end).unwrap_or_default().to_vec();
        rewrite(&mut blob, &order);
        if blob.len() % 2 == 1 {
            blob.push(0);
        }
        glyphs.push(blob);
    }
    let ttf = pack(head, hhea, hmtx, maxp, h_metrics, &glyphs, &source)?;
    Some(Subset { ttf, cid, width })
}

fn collect(gid: u16, glyf: &[u8], loca: &[u8], long: bool, source: &mut Vec<u16>, seen: &mut BTreeSet<u16>, depth: u8) {
    if depth > 8 || !seen.insert(gid) {
        return;
    }
    source.push(gid);
    let Ok((start, end)) = span(loca, long, gid as usize) else { return };
    let Some(blob) = glyf.get(start..end) else { return };
    for part in components(blob) {
        collect(part, glyf, loca, long, source, seen, depth + 1);
    }
}

fn components(glyph: &[u8]) -> Vec<u16> {
    if glyph.len() < 10 || i16::from_be_bytes([glyph[0], glyph[1]]) >= 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = 10;
    loop {
        if i + 4 > glyph.len() {
            break;
        }
        let flags = u16::from_be_bytes([glyph[i], glyph[i + 1]]);
        out.push(u16::from_be_bytes([glyph[i + 2], glyph[i + 3]]));
        i += 4 + if flags & 1 != 0 { 4 } else { 2 };
        if flags & 8 != 0 {
            i += 2;
        } else if flags & 64 != 0 {
            i += 4;
        } else if flags & 128 != 0 {
            i += 8;
        }
        if flags & 32 == 0 {
            break;
        }
    }
    out
}

fn rewrite(glyph: &mut [u8], order: &BTreeMap<u16, u16>) {
    if glyph.len() < 10 || i16::from_be_bytes([glyph[0], glyph[1]]) >= 0 {
        return;
    }
    let mut i = 10;
    loop {
        if i + 4 > glyph.len() {
            return;
        }
        let flags = u16::from_be_bytes([glyph[i], glyph[i + 1]]);
        let old = u16::from_be_bytes([glyph[i + 2], glyph[i + 3]]);
        let new = order.get(&old).copied().unwrap_or(0);
        glyph[i + 2] = (new >> 8) as u8;
        glyph[i + 3] = (new & 0xff) as u8;
        i += 4 + if flags & 1 != 0 { 4 } else { 2 };
        if flags & 8 != 0 {
            i += 2;
        } else if flags & 64 != 0 {
            i += 4;
        } else if flags & 128 != 0 {
            i += 8;
        }
        if flags & 32 == 0 {
            return;
        }
    }
}

fn pack(head: &[u8], hhea: &[u8], hmtx: &[u8], maxp: &[u8], h_metrics: usize, glyphs: &[Vec<u8>], source: &[u16]) -> Option<Vec<u8>> {
    let n = glyphs.len();
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    for blob in glyphs {
        put_u32(&mut loca, u32::try_from(glyf.len()).ok()?);
        glyf.extend_from_slice(blob);
    }
    put_u32(&mut loca, u32::try_from(glyf.len()).ok()?);
    let mut hmtx_out = Vec::new();
    for old in source {
        let advance = hmtx_advance(hmtx, h_metrics, *old as usize);
        let lsb = hmtx_lsb(hmtx, h_metrics, *old as usize);
        hmtx_out.extend_from_slice(&advance.to_be_bytes());
        hmtx_out.extend_from_slice(&lsb.to_be_bytes());
    }
    let mut head = head.to_vec();
    head[8..12].fill(0);
    head[50] = 0;
    head[51] = 1;
    let mut hhea = hhea.to_vec();
    let count = u16::try_from(n).ok()?;
    hhea[34] = (count >> 8) as u8;
    hhea[35] = (count & 0xff) as u8;
    let mut maxp = maxp.to_vec();
    if maxp.len() >= 6 {
        maxp[4] = (count >> 8) as u8;
        maxp[5] = (count & 0xff) as u8;
    }
    let cmap = cmap4();
    let post = vec![0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let tables = [("cmap", cmap), ("glyf", glyf), ("head", head), ("hhea", hhea), ("hmtx", hmtx_out), ("loca", loca), ("maxp", maxp), ("post", post)];
    let mut body = Vec::new();
    let count_t = u16::try_from(tables.len()).ok()?;
    let entry = count_t.ilog2();
    let search = 1_u16 << (entry + 4);
    body.extend_from_slice(&0x00010000_u32.to_be_bytes());
    body.extend_from_slice(&count_t.to_be_bytes());
    body.extend_from_slice(&search.to_be_bytes());
    body.extend_from_slice(&(entry as u16).to_be_bytes());
    body.extend_from_slice(&(count_t * 16 - search).to_be_bytes());
    let dir = 12 + tables.len() * 16;
    let mut offset = dir;
    let mut records = Vec::new();
    let mut blobs = Vec::new();
    for (name, data) in tables {
        let start = align4(offset);
        let len = data.len();
        records.push((name, checksum(&data), start, len));
        blobs.push((start, data));
        offset = start + align4(len);
    }
    for (name, sum, start, len) in records {
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(&sum.to_be_bytes());
        body.extend_from_slice(&u32::try_from(start).ok()?.to_be_bytes());
        body.extend_from_slice(&u32::try_from(len).ok()?.to_be_bytes());
    }
    for (start, data) in blobs {
        body.resize(start, 0);
        body.extend_from_slice(&data);
    }
    body.resize(align4(body.len()), 0);
    Some(body)
}

fn cmap4() -> Vec<u8> {
    let mut c = Vec::new();
    for v in [0_u16, 4, 4, 0] {
        c.extend_from_slice(&v.to_be_bytes());
    }
    c
}

fn hmtx_advance(hmtx: &[u8], metrics: usize, index: usize) -> u16 {
    let i = index.min(metrics.saturating_sub(1));
    hmtx.get(i * 4..i * 4 + 2).map(|b| u16::from_be_bytes([b[0], b[1]])).unwrap_or(0)
}

fn hmtx_lsb(hmtx: &[u8], metrics: usize, index: usize) -> i16 {
    if index < metrics {
        return hmtx.get(index * 4 + 2..index * 4 + 4).map(|b| i16::from_be_bytes([b[0], b[1]])).unwrap_or(0);
    }
    let at = metrics * 4 + (index - metrics) * 2;
    hmtx.get(at..at + 2).map(|b| i16::from_be_bytes([b[0], b[1]])).unwrap_or(0)
}

fn span(loca: &[u8], long: bool, index: usize) -> Result<(usize, usize), ()> {
    if long {
        let a = u32be(loca, index * 4).ok_or(())? as usize;
        let b = u32be(loca, index * 4 + 4).ok_or(())? as usize;
        Ok((a, b.max(a)))
    } else {
        let a = u16be(loca, index * 2).ok_or(())? as usize * 2;
        let b = u16be(loca, index * 2 + 2).ok_or(())? as usize * 2;
        Ok((a, b.max(a)))
    }
}

fn tag(bytes: &[u8; 4]) -> ttf_parser::Tag {
    ttf_parser::Tag::from_bytes(bytes)
}
fn u16be(data: &[u8], at: usize) -> Option<u16> {
    data.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}

fn u32be(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}
fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn align4(n: usize) -> usize { n.div_ceil(4) * 4 }

#[cfg(test)]
#[path = "sheet_pdf_font_tests.rs"]
mod tests;

fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0_u32;
    let mut i = 0;
    while i < data.len() {
        let mut word = [0_u8; 4];
        let n = (data.len() - i).min(4);
        word[..n].copy_from_slice(&data[i..i + n]);
        sum = sum.wrapping_add(u32::from_be_bytes(word));
        i += 4;
    }
    sum
}
