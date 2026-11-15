//! The one-picture PDF, read back the way a reader finds its way: from the end of the file to the table, from
//! the table to every object.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::picture_page_pdf;

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|window| window == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|window| window == needle)
}

/// Where every object starts, read from the table the last lines of the file point to.
fn object_starts(pdf: &[u8]) -> Vec<usize> {
    let pointer = rfind(pdf, b"startxref\n").unwrap() + "startxref\n".len();
    let end = pointer + find(&pdf[pointer..], b"\n").unwrap();
    let table: usize = std::str::from_utf8(&pdf[pointer..end]).unwrap().parse().unwrap();
    assert!(pdf[table..].starts_with(b"xref\n0 7\n0000000000 65535 f \n"), "the table is where the file says it is");
    let rows = &pdf[table + "xref\n0 7\n".len()..];
    (1..7).map(|row| std::str::from_utf8(&rows[row * 20..row * 20 + 10]).unwrap().parse().unwrap()).collect()
}

/// A stand-in for a JPEG with bytes a text would break on (a line end, a zero, the word that ends a stream).
const PICTURE: &[u8] = b"\xFF\xD8\xFF\xE0\n\r\0endstream\x00\xFF\xD9";

#[test]
fn the_table_points_at_every_object_and_the_page_is_the_paper() {
    let pdf = picture_page_pdf([210.0, 297.0], "도면", PICTURE, [2480, 3508]);
    assert!(pdf.starts_with(b"%PDF-1.4\n"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    for (index, start) in object_starts(&pdf).into_iter().enumerate() {
        let head = format!("{} 0 obj\n", index + 1);
        assert!(pdf[start..].starts_with(head.as_bytes()), "object {} starts at {start}", index + 1);
    }
    // A4 upright in points of 1/72 inch: 210 mm = 595.276, 297 mm = 841.890.
    assert!(find(&pdf, b"/MediaBox [0 0 595.276 841.890]").is_some());
    // The picture is stretched over the whole page.
    assert!(find(&pdf, b"q 595.276 0 0 841.890 0 0 cm /Im0 Do Q").is_some());
    assert!(find(&pdf, b"/Root 1 0 R").is_some() && find(&pdf, b"/Info 6 0 R").is_some());
}

#[test]
fn the_picture_is_inside_byte_for_byte_with_its_length_and_size() {
    let pdf = picture_page_pdf([297.0, 420.0], "도면", PICTURE, [3508, 4961]);
    let picture = object_starts(&pdf)[4];
    let head = format!("5 0 obj\n<< /Type /XObject /Subtype /Image /Width 3508 /Height 4961 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", PICTURE.len());
    assert!(pdf[picture..].starts_with(head.as_bytes()));
    let data = picture + head.len();
    assert_eq!(&pdf[data..data + PICTURE.len()], PICTURE);
    assert!(pdf[data + PICTURE.len()..].starts_with(b"\nendstream\nendobj\n"));
}

#[test]
fn a_korean_title_is_written_so_that_a_reader_shows_it() {
    let pdf = picture_page_pdf([210.0, 297.0], "조사도면 1", PICTURE, [1, 1]);
    let at = find(&pdf, b"/Title <").unwrap() + "/Title <".len();
    let end = at + find(&pdf[at..], b">").unwrap();
    let units: Vec<u16> = pdf[at..end].chunks(4).map(|hex| u16::from_str_radix(std::str::from_utf8(hex).unwrap(), 16).unwrap()).collect();
    assert_eq!(units[0], 0xFEFF, "the mark of UTF-16BE comes first");
    assert_eq!(String::from_utf16(&units[1..]).unwrap(), "조사도면 1");
}
