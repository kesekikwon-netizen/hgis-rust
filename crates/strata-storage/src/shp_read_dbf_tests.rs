//! `shp_read_dbf` 시험 — 인코딩 판정·필드 종류·값 문자열화·소수 자릿수.

use shapefile::dbase::{FieldType, FieldValue};

use super::*;

#[test]
fn cpg_labels_are_recognised_like_the_cpp_helper() {
    // C++ `prepareShapefileEncoding` 이 보던 이름들.
    assert_eq!(encoding_from_cpg_label("UTF-8\n"), Some(ShpEncoding::Utf8));
    assert_eq!(encoding_from_cpg_label("utf8"), Some(ShpEncoding::Utf8));
    assert_eq!(encoding_from_cpg_label("\u{feff}65001"), Some(ShpEncoding::Utf8));
    assert_eq!(encoding_from_cpg_label("CP949"), Some(ShpEncoding::Cp949));
    assert_eq!(encoding_from_cpg_label("949"), Some(ShpEncoding::Cp949));
    assert_eq!(encoding_from_cpg_label("EUC-KR"), Some(ShpEncoding::Cp949));
    assert_eq!(encoding_from_cpg_label("ANSI 949"), Some(ShpEncoding::Cp949));
    assert_eq!(encoding_from_cpg_label("System"), Some(ShpEncoding::Cp949));
    assert_eq!(encoding_from_cpg_label(""), None);
    assert_eq!(encoding_from_cpg_label("SHIFT_JIS_XYZ"), None);
}

#[test]
fn utf8_detection_needs_four_multibyte_sequences_like_cpp() {
    let korean = "가나다라";
    assert!(looks_like_utf8(korean.as_bytes()), "한글 4자 이상은 UTF-8 로 본다");
    // 한글 2자(멀티바이트 2개)는 증거가 약해 CP949 로 간다(C++ 도 4개 미만은 CP949).
    assert!(!looks_like_utf8("가나".as_bytes()));
    assert!(!looks_like_utf8(b"plain ascii"));
    // CP949 바이트는 UTF-8 로 해독되지 않는다.
    let (cp949, _, _) = encoding_rs::EUC_KR.encode("경주시");
    assert!(!looks_like_utf8(cp949.as_ref()));
    // 끝에서 잘린 시퀀스는 UTF-8 증거로 치지 않는다: 온전한 4자에서 마지막 3바이트를
    // 잘라내면 "가나다" 3개만 남아 기준(4개) 미달이므로 UTF-8 로 보지 않는다.
    let full = "가나다라".as_bytes();
    assert!(!looks_like_utf8(&full[..full.len() - 3]));
}

#[test]
fn sample_skips_the_dbf_header() {
    // 헤더 32 + 필드 32 + 종결 1 = 65바이트 뒤부터 레코드가 시작한다.
    let mut dbf = vec![0u8; 32];
    dbf[8] = 65;
    dbf[9] = 0;
    dbf.extend_from_slice(&[0u8; 33]); // 필드 정의 32 + 종결 1
    dbf.extend_from_slice(b"record-data");
    let sample = dbf_record_sample(&dbf, 32_768);
    assert_eq!(sample, b"record-data".to_vec());
}

#[test]
fn decimals_are_read_from_field_definitions() {
    let mut dbf = vec![0u8; 32];
    let header_len = 32 + 2 * 32 + 1;
    dbf[8] = u8::try_from(header_len).expect("65");
    dbf[9] = 0;
    for (i, dec) in [0u8, 2u8].iter().enumerate() {
        let mut field = vec![0u8; 32];
        field[17] = *dec;
        dbf.extend_from_slice(&field);
        let _ = i;
    }
    dbf.push(0x0D);
    assert_eq!(field_decimals(&dbf), vec![0, 2]);
}

#[test]
fn field_kinds_follow_dbf_types() {
    assert_eq!(field_kind(FieldType::Character, 0), ShpFieldKind::Text);
    assert_eq!(field_kind(FieldType::Memo, 0), ShpFieldKind::Text);
    assert_eq!(field_kind(FieldType::Date, 0), ShpFieldKind::Date);
    assert_eq!(field_kind(FieldType::DateTime, 0), ShpFieldKind::Date);
    assert_eq!(field_kind(FieldType::Logical, 0), ShpFieldKind::Logical);
    assert_eq!(field_kind(FieldType::Integer, 0), ShpFieldKind::Integer);
    // 숫자 필드는 소수 자릿수 0 이면 정수로 본다(한국 SHP 의 유구번호·조사번호가 이 꼴).
    assert_eq!(field_kind(FieldType::Numeric, 0), ShpFieldKind::Integer);
    assert_eq!(field_kind(FieldType::Numeric, 2), ShpFieldKind::Real);
    assert_eq!(field_kind(FieldType::Float, 0), ShpFieldKind::Integer);
    assert_eq!(field_kind(FieldType::Float, 3), ShpFieldKind::Real);
    assert_eq!(field_kind(FieldType::Double, 0), ShpFieldKind::Real);
    assert_eq!(field_kind(FieldType::Currency, 0), ShpFieldKind::Real);
}

#[test]
fn values_become_strings_without_losing_precision_shape() {
    assert_eq!(
        value_text(&FieldValue::Character(Some("경주".to_owned())), 0),
        Some("경주".to_owned())
    );
    assert_eq!(value_text(&FieldValue::Character(None), 0), None);
    assert_eq!(value_text(&FieldValue::Numeric(Some(17.0)), 0), Some("17".to_owned()));
    assert_eq!(
        value_text(&FieldValue::Numeric(Some(42.5)), 2),
        Some("42.50".to_owned())
    );
    assert_eq!(value_text(&FieldValue::Numeric(None), 0), None);
    assert_eq!(value_text(&FieldValue::Logical(Some(false)), 0), Some("F".to_owned()));
    assert_eq!(value_text(&FieldValue::Logical(None), 0), None);
    assert_eq!(value_text(&FieldValue::Integer(-3), 0), Some("-3".to_owned()));
    let date = shapefile::dbase::Date::new(9, 11, 2026).expect("valid date");
    assert_eq!(
        value_text(&FieldValue::Date(Some(date)), 0),
        Some("2026-11-09".to_owned())
    );
}

#[test]
fn non_finite_numbers_become_empty() {
    assert_eq!(format_number(f64::NAN, 2), String::new());
    assert_eq!(format_number(f64::INFINITY, 0), String::new());
}

#[test]
fn resolve_encoding_defaults_to_cp949_without_cpg_or_utf8_evidence() {
    // 레코드 본문이 ASCII 뿐이면 UTF-8 증거가 없으므로 CP949(한국 기본값).
    let mut dbf = vec![0u8; 32];
    dbf[8] = 33;
    dbf[9] = 0;
    dbf.push(0x0D);
    dbf.extend_from_slice(b"plain");
    assert_eq!(
        resolve_encoding(std::path::Path::new("no-such.shp"), &dbf),
        ShpEncoding::Cp949
    );
}

#[test]
fn resolve_encoding_detects_utf8_body_without_cpg() {
    let mut dbf = vec![0u8; 32];
    dbf[8] = 33;
    dbf[9] = 0;
    dbf.push(0x0D);
    dbf.extend_from_slice("경주시성동동".as_bytes());
    assert_eq!(
        resolve_encoding(std::path::Path::new("no-such.shp"), &dbf),
        ShpEncoding::Utf8
    );
}
