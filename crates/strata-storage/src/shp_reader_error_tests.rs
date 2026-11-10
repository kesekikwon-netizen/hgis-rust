//! `read_shapefile` 오류 시험 — 빈 파일·깨진 파일에서 **패닉 없이** `Err`.
//!
//! C++ 쪽에서 잘린 SHP/ZIP 이 0xc0000005 로 죽던 자리를 Rust 에서는 `StrataError` 로
//! 돌려주는지 확인한다.

use strata_types::error::StrataError;

use super::test_support::TempShpDir;
use super::read_shapefile;

#[test]
fn missing_shp_is_invalid_input_error() {
    let dir = TempShpDir::new("missing");
    let err = read_shapefile(&dir.base("없는파일")).expect_err("없는 파일은 오류여야 한다");
    assert!(
        matches!(err, StrataError::InvalidInput(_)),
        "InvalidInput 이어야 한다: {err:?}"
    );
}

#[test]
fn empty_shp_and_dbf_are_errors_not_panics() {
    let dir = TempShpDir::new("empty");
    let base = dir.base("빈파일");
    std::fs::write(base.with_extension("shp"), b"").expect("빈 .shp 쓰기");
    std::fs::write(base.with_extension("dbf"), b"").expect("빈 .dbf 쓰기");

    let err = read_shapefile(&base).expect_err("빈 파일은 오류여야 한다");
    assert!(
        matches!(err, StrataError::Storage(_) | StrataError::InvalidInput(_)),
        "Storage/InvalidInput 이어야 한다: {err:?}"
    );
}

#[test]
fn truncated_shp_header_is_an_error_not_a_panic() {
    let dir = TempShpDir::new("truncated");
    let base = dir.base("잘린파일");
    // 파일 코드(9994)만 있고 나머지 100바이트 헤더가 없는 파일.
    std::fs::write(base.with_extension("shp"), [0x00, 0x00, 0x27, 0x0A]).expect(".shp 쓰기");
    std::fs::write(base.with_extension("dbf"), [0x03, 0x7E, 0x0B, 0x07]).expect(".dbf 쓰기");

    let err = read_shapefile(&base).expect_err("잘린 파일은 오류여야 한다");
    assert!(
        matches!(err, StrataError::Storage(_) | StrataError::InvalidInput(_)),
        "Storage/InvalidInput 이어야 한다: {err:?}"
    );
}

#[test]
fn garbage_bytes_are_an_error_not_a_panic() {
    let dir = TempShpDir::new("garbage");
    let base = dir.base("깨진파일");
    let junk: Vec<u8> = (0..512_u32).map(|i| (i * 37 % 251) as u8).collect();
    std::fs::write(base.with_extension("shp"), &junk).expect(".shp 쓰기");
    std::fs::write(base.with_extension("dbf"), &junk).expect(".dbf 쓰기");

    let err = read_shapefile(&base).expect_err("깨진 파일은 오류여야 한다");
    assert!(
        matches!(err, StrataError::Storage(_) | StrataError::InvalidInput(_)),
        "Storage/InvalidInput 이어야 한다: {err:?}"
    );
}

#[test]
fn missing_dbf_is_reported_as_invalid_input() {
    let dir = TempShpDir::new("no-dbf");
    let base = dir.base("속성없음");
    // .shp 만 있으면 속성을 읽을 길이 없다는 뜻이므로 InvalidInput.
    std::fs::write(base.with_extension("shp"), [0x00, 0x00, 0x27, 0x0A]).expect(".shp 쓰기");

    let err = read_shapefile(&base).expect_err(".dbf 가 없으면 오류여야 한다");
    match err {
        StrataError::InvalidInput(msg) => assert!(
            msg.contains("dbf"),
            ".dbf 부재를 알려야 한다: {msg}"
        ),
        other => panic!(".dbf 부재는 InvalidInput 이어야 한다: {other:?}"),
    }
}

#[test]
fn broken_cpg_label_falls_back_to_cp949_instead_of_failing() {
    let dir = TempShpDir::new("bad-cpg");
    let base = dir.base("이상cpg");
    // 알아볼 수 없는 이름이면 CP949 폴백으로 간다(오류 아님).
    super::test_support::point_layer(
        &base,
        &[[1.0, 2.0]],
        &["가"],
        super::test_support::cp949_table,
    )
    .expect("시험 자료 작성");
    std::fs::write(base.with_extension("cpg"), "SHIFT_JIS_XYZ").expect(".cpg 쓰기");

    let layer = read_shapefile(&base).expect("CP949 폴백으로 읽어야 한다");
    assert_eq!(layer.features.len(), 1);
    assert_eq!(
        layer.features[0].attributes.get("NAME").map(String::as_str),
        Some("가")
    );
}
