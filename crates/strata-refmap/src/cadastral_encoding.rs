//! How the text of a county shapefile is read when it comes without a `.cpg`
//! (`target/cpp-ref/src/core/CadastralEncoding.cpp`).

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// The file beside a shapefile with this extension, in small or in capital letters (`:8-14`).
pub(super) fn sibling(shp: &Path, extension: &str) -> Option<PathBuf> {
    [extension.to_owned(), extension.to_uppercase()].into_iter().map(|ext| shp.with_extension(ext)).find(|path| path.exists())
}

/// The length of a whole UTF-8 character of several bytes that begins at `at`; 0 when there is none (`:17-28`).
fn utf8_sequence(bytes: &[u8], at: usize) -> usize {
    let lead = bytes[at];
    let length = match lead {
        0xC2..=0xDF => 2,
        _ if lead & 0xF0 == 0xE0 => 3,
        0xF0..=0xF4 => 4,
        _ => 0,
    };
    if length == 0 || at + length > bytes.len() || !bytes[at + 1..at + length].iter().all(|byte| byte & 0xC0 == 0x80) {
        return 0;
    }
    length
}

/// The encoding of the text of a `.dbf`, from a piece of its records (`detectDbfEncoding`, `:32-49`): only
/// text that clearly is UTF-8 (four whole characters of several bytes, none broken) is read as UTF-8; the
/// Korean public data is CP949 otherwise.
#[must_use]
pub fn dbf_encoding(sample: &[u8]) -> &'static str {
    let (mut whole, mut at) = (0, 0);
    while at < sample.len() {
        if sample[at] < 0x80 {
            at += 1;
            continue;
        }
        let length = utf8_sequence(sample, at);
        if length == 0 {
            // A character cut off at the very end of the sample says nothing against UTF-8.
            if sample.len() - at < 4 {
                break;
            }
            return "CP949";
        }
        whole += 1;
        at += length;
    }
    if whole >= 4 {
        "UTF-8"
    } else {
        "CP949"
    }
}

/// The encoding to read a county shapefile with when it has no `.cpg` (`fallbackFor`, `:51-65`); `None`
/// when the file says it itself, or is no shapefile.
#[must_use]
pub fn fallback_encoding(shp: &Path) -> Option<&'static str> {
    if !shp.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("shp")) || sibling(shp, "cpg").is_some() {
        return None;
    }
    let mut file = std::fs::File::open(sibling(shp, "dbf")?).ok()?;
    // The header (record count, lengths) is skipped: only the text of the records says anything.
    let mut header = Vec::new();
    file.by_ref().take(32).read_to_end(&mut header).ok()?;
    if header.len() < 12 {
        return Some("CP949");
    }
    let records_at = u64::from(header[8]) | (u64::from(header[9]) << 8);
    if records_at > 0 && file.seek(SeekFrom::Start(records_at)).is_err() {
        return Some("CP949");
    }
    let mut sample = Vec::new();
    file.take(32_768).read_to_end(&mut sample).ok()?;
    Some(dbf_encoding(&sample))
}
