//! `ngii_topographic_ui` 시험 — 도엽 번호 후보와 좌표 변환.
//!
//! 원래 `ngii_topographic_ui.rs` 안 `mod tests` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 는 이 파일의 부모를 가리키므로
//! `crate::ngii_topographic_ui::*` 로 명시한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::ngii_topographic_ui::*;

// C++ `MainWindowDownloads.cpp:179-182` — 후보가 비면 `359011`
#[test]
fn test_nearby_sheet_numbers_returns_cpp_fallback_when_out_of_range() {
    assert_eq!(nearby_sheet_numbers([f64::NAN, 0.0]), vec!["359011".to_owned()]);
    // 한국 밖(경도 0)은 1:25,000 격자 후보가 없다 → C++ 과 같은 기본값
    assert_eq!(nearby_sheet_numbers([0.0, 0.0]), vec!["359011".to_owned()]);
}

#[test]
fn test_nearby_sheet_numbers_finds_gyeongju_sheet() {
    // 경주 월성 일대(129.0°E, 35.83°N) 근처 도엽
    let numbers = nearby_sheet_numbers([129.005, 35.834]);
    assert_eq!(numbers.len(), 1, "한 점은 도엽 한 장: {numbers:?}");
    let number = &numbers[0];
    assert_eq!(number.len(), 6, "{number}");
    let sheet = strata_net::ngii_topographic::TopographicSheets::resolve(number).expect(number);
    assert!((sheet.west..=sheet.east + 1e-9).contains(&129.005), "{number}");
    assert!((sheet.south..=sheet.north + 1e-9).contains(&35.834), "{number}");
}

// C++ `MainWindowDownloads.cpp:183-186` — 포털 질의는 EPSG:5179
#[test]
fn test_portal_center_converts_work_crs_to_5179() {
    let out = portal_center([220_000.0, 360_000.0], strata_types::crs::WorkCrs::Epsg5187)
        .expect("변환");
    assert!((800_000.0..1_200_000.0).contains(&out[0]), "{out:?}");
    assert!((1_400_000.0..2_100_000.0).contains(&out[1]), "{out:?}");
    assert!(portal_center(
        [f64::INFINITY, 0.0],
        strata_types::crs::WorkCrs::Epsg5186
    )
    .is_none());
}

#[test]
fn test_center_wgs84_for_gyeongju_is_plausible() {
    let wgs = center_wgs84([220_000.0, 360_000.0], strata_types::crs::WorkCrs::Epsg5187)
        .expect("변환");
    assert!((128.8..129.3).contains(&wgs[0]), "{wgs:?}");
    assert!((35.5..36.1).contains(&wgs[1]), "{wgs:?}");
}
