//! `read_shapefile` 한글 인코딩 시험 — CP949 폴백, `.cpg` UTF-8, `.cpg` 우선.
//!
//! `shp_reader_tests` 에서 분리했다(300줄 규칙).
//! C++ 은 `LayerOps::prepareShapefileEncoding`(`D:\hgis\src\core\LayerOps.cpp`)이
//! `.cpg` → 자동 판정 순으로 인코딩을 정하고, 없으면 CP949 를 기본값으로 둔다.
//! 여기서는 그 세 갈래가 Rust 쪽에서도 같은 결과를 내는지 본다.

use super::test_support::{cp949_table, point_layer, utf8_table, TempShpDir};
use super::read_shapefile;

const KOREAN: &str = "경주시 성동동 유적";

#[test]
fn decodes_korean_attributes_from_cp949_without_cpg() -> Result<(), String> {
    // 한국 공공 SHP 기본형: `.cpg` 없이 CP949 바이트. C++ 도 이걸 폴백 기본값으로 둔다.
    let dir = TempShpDir::new("cp949");
    let base = dir.base("유적");
    point_layer(
        &base,
        &[[219_800.0, 361_500.0], [219_900.0, 361_600.0]],
        &[KOREAN, "발굴조사 1호 유구"],
        cp949_table,
    )?;
    assert!(
        super::find_sibling(&dir.base("유적"), "cpg").is_none(),
        "시험 전제: .cpg 가 없어야 한다"
    );

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(
        layer.features[0].attributes.get("NAME").map(String::as_str),
        Some(KOREAN)
    );
    assert_eq!(
        layer.features[1].attributes.get("NAME").map(String::as_str),
        Some("발굴조사 1호 유구")
    );
    // 깨짐 판정: 대체문자(U+FFFD)가 없어야 한다.
    for f in &layer.features {
        for v in f.attributes.values() {
            assert!(!v.contains('\u{FFFD}'), "깨진 글자: {v}");
        }
    }
    Ok(())
}

#[test]
fn honours_cpg_utf8_even_though_bytes_would_pass_as_cp949() -> Result<(), String> {
    // 같은 한글을 UTF-8 로 쓰고 `.cpg` 에 UTF-8 을 적는다.
    let dir = TempShpDir::new("cpg-utf8");
    let base = dir.base("유적");
    point_layer(&base, &[[219_800.0, 361_500.0]], &[KOREAN], utf8_table)?;
    std::fs::write(base.with_extension("cpg"), "UTF-8\n").map_err(|e| e.to_string())?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(
        layer.features[0].attributes.get("NAME").map(String::as_str),
        Some(KOREAN)
    );
    Ok(())
}

#[test]
fn cpg_cp949_label_wins_over_utf8_looking_bytes() -> Result<(), String> {
    // `.cpg` 가 CP949 라고 하면 그 말을 따른다(자동 판정보다 우선).
    let dir = TempShpDir::new("cpg-cp949");
    let base = dir.base("유적");
    point_layer(&base, &[[219_800.0, 361_500.0]], &[KOREAN], cp949_table)?;
    std::fs::write(base.with_extension("cpg"), "CP949").map_err(|e| e.to_string())?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(
        layer.features[0].attributes.get("NAME").map(String::as_str),
        Some(KOREAN)
    );
    Ok(())
}

#[test]
fn cpg_absent_utf8_body_is_detected_as_utf8() -> Result<(), String> {
    // `.cpg` 가 없어도 본문이 온전한 UTF-8 멀티바이트 4개 이상이면 UTF-8 로 읽는다
    // (C++ `prepareShapefileEncoding` 2단계의 `utf8MultiByteCount >= 4` 판정).
    let dir = TempShpDir::new("utf8-no-cpg");
    let base = dir.base("유적");
    point_layer(&base, &[[219_800.0, 361_500.0]], &[KOREAN], utf8_table)?;

    let layer = read_shapefile(&base).map_err(|e| e.to_string())?;

    assert_eq!(
        layer.features[0].attributes.get("NAME").map(String::as_str),
        Some(KOREAN)
    );
    Ok(())
}
