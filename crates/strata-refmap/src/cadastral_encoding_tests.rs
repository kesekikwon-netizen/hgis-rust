//! The text of a county shapefile that comes without a `.cpg` (`test_cadastral.cpp:461-522`, F123).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::tests::{parcels, pnu, square, wide, written};
use super::*;

/// `CadastralEncoding::detectDbfEncoding` (`CadastralEncoding.cpp:32-49`): CP949 unless the text clearly
/// is UTF-8 (four whole multi-byte characters and none that is broken).
#[test]
fn the_text_of_a_dbf_is_korean_unless_it_clearly_is_utf8() {
    assert_eq!(dbf_encoding(b"\xBB\xEA 23-1    "), "CP949", "「산」 in CP949");
    assert_eq!(dbf_encoding("산 23-1   산 24   답 3   대 5".as_bytes()), "UTF-8");
    assert_eq!(dbf_encoding(b"1-2 3-4 5"), "CP949");
    assert_eq!(dbf_encoding("산 23-1".as_bytes()), "CP949", "one character is too little to be sure");
    // A character cut off at the very end of the sample says nothing against UTF-8 (`:41-43`).
    let mut cut = "산 23-1   산 24   답 3   대 5 임".as_bytes().to_vec();
    cut.pop();
    assert_eq!(dbf_encoding(&cut), "UTF-8");
}

/// `shapefileWithoutCpgKeepsKoreanJibun` (`:463-522`): a county file whose `.dbf` holds CP949 and which has
/// no `.cpg` keeps its lot numbers readable; a `.cpg` is taken as it is; another kind of file has no rule.
#[test]
fn a_shapefile_without_cpg_keeps_its_korean_lot_numbers() {
    let (input, output) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let shp = parcels(input.path(), "LSMD_CONT_LDREG_fixture", &[(7, "XX 23-1", square(200_000.0, 550_000.0, 10.0, 10.0))], true, true);
    // Turn it into a file as the portal sends it: CP949 bytes in the `.dbf` and no `.cpg` beside it.
    let dbf = shp.with_extension("dbf");
    let mut bytes = std::fs::read(&dbf).unwrap();
    let at = bytes.windows(7).position(|window| window == b"XX 23-1").unwrap();
    bytes[at..at + 2].copy_from_slice(b"\xBB\xEA");
    std::fs::write(&dbf, bytes).unwrap();
    std::fs::remove_file(shp.with_extension("cpg")).unwrap();
    assert_eq!(fallback_encoding(&shp), Some("CP949"));

    let ready = prepare(std::slice::from_ref(&shp), Some(&wide()), 5186, output.path(), &|| false, &mut |_, _| {}).unwrap();
    assert_eq!(written(&ready.file), [(pnu(7), "산 23-1".to_owned(), written(&ready.file)[0].2.clone())]);

    std::fs::write(shp.with_extension("cpg"), "CP949").unwrap();
    assert_eq!(fallback_encoding(&shp), None, "a .cpg is taken as it is");
    assert_eq!(fallback_encoding(&input.path().join("other.gpkg")), None);
}
