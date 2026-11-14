//! ZIP files for the archive tests: ordinary deflated ones through the `zip` library, and hand-built stored
//! ones for names and attributes no well-behaved writer produces.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::Path;

pub fn le16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

pub fn le32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

pub fn put_le32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// A deflated ZIP as an ordinary tool writes it (names in UTF-8).
pub fn zip_deflated(path: &Path, entries: &[(&str, Vec<u8>)]) {
    let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
}

/// One stored entry of a hand-built ZIP.
pub struct RawEntry<'a> {
    name: &'a str,
    data: &'a [u8],
    unix_mode: Option<u32>,
}

impl<'a> RawEntry<'a> {
    pub fn file(name: &'a str, data: &'a [u8]) -> Self {
        Self { name, data, unix_mode: None }
    }

    /// Marks the entry as made on Unix with this file mode (type and permission bits).
    pub fn unix_mode(mut self, mode: u32) -> Self {
        self.unix_mode = Some(mode);
        self
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A ZIP of stored entries with exactly the given names (UTF-8 flag set), byte for byte.
pub fn zip_raw(path: &Path, entries: &[RawEntry<'_>]) {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    for entry in entries {
        let (name, size, crc, offset) = (entry.name.as_bytes(), entry.data.len() as u32, crc32(entry.data), out.len() as u32);
        let common = |out: &mut Vec<u8>| {
            out.extend_from_slice(&0x0800_u16.to_le_bytes()); // flags: names are UTF-8
            out.extend_from_slice(&0_u16.to_le_bytes()); // stored
            out.extend_from_slice(&[0, 0, 0x21, 0]); // time, date (1980-01-01)
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0_u16.to_le_bytes()); // extra
        };
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04, 20, 0]);
        common(&mut out);
        out.extend_from_slice(name);
        out.extend_from_slice(entry.data);
        let made_by: u16 = if entry.unix_mode.is_some() { (3 << 8) | 20 } else { 20 };
        central.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        central.extend_from_slice(&made_by.to_le_bytes());
        central.extend_from_slice(&20_u16.to_le_bytes());
        common(&mut central);
        central.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // comment length, disk, internal attributes
        central.extend_from_slice(&(entry.unix_mode.unwrap_or(0) << 16).to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    let (start, length, count) = (out.len() as u32, central.len() as u32, entries.len() as u16);
    out.extend_from_slice(&central);
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    std::fs::write(path, out).unwrap();
}
