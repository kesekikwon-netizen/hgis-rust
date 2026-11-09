//! `topographic_catalog` 시험 1/2 — 폴더 스캔·확장자 거르기·PRJ 동반 확인·DXF.
//!
//! 원래 `topographic_catalog.rs` 안 `mod tests` 를 주제별로 나눴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::path::Path;
use super::test_support::*;

#[test]
fn test_r14_scan_folder_filters_extensions_and_checks_prj_companion() {
    let folder = TempFolder::new("r14_filter");
    folder.write("A0010000.shp", &shp_bytes(Extent {
        min_x: 218_450.0,
        min_y: 359_800.0,
        max_x: 221_250.0,
        max_y: 362_400.0,
    }));
    folder.write("A0010000.prj", PRJ_5186.as_bytes());
    // .prj 가 없는 shp — 좌표계를 확인할 수 없다.
    folder.write("B0010000.shp", &shp_bytes(Extent {
        min_x: 1.0,
        min_y: 2.0,
        max_x: 3.0,
        max_y: 4.0,
    }));
    // 읽지 않는 확장자들.
    folder.write("메모.txt", b"not a layer");
    folder.write("도면.dwg", b"AC1032");
    folder.write("묶음.zip", b"PK\x03\x04");
    // 하위 폴더도 본다(C++ `QDirIterator::Subdirectories`).
    folder.write("하위/G0010000.ngi", b"\x00\x00\x00\x00");

    let result = scan_folder(&folder.path);
    assert_eq!(result.error, None);
    assert_eq!(result.files_read, 3, "shp 둘 + ngi 하나만 센다");
    assert_eq!(result.records.len(), 3);
    assert_eq!(result.cache_hits, 0, "색인 캐시를 두지 않는다");

    let road = result
        .records
        .iter()
        .find(|record| record.layer_name == "A0010000")
        .expect("A0010000.shp 레코드");
    // 코드북 대조(`TopographicCatalog.cpp:99`) — A0010000 은 도로다.
    assert_eq!(road.category, Category::Road);
    assert_eq!(road.display_name, "도로경계(미분류)");
    assert_eq!(road.crs_auth_id, "EPSG:5186", ".prj 를 읽어 좌표계를 채운다");
    assert!(!road.missing_prj);
    assert_eq!(
        road.extent_label_ko(),
        "X 218450.0 ~ 221250.0\nY 359800.0 ~ 362400.0"
    );
    assert!(road.review_reason.is_none());
    assert!(road.initially_selected(), "도로는 처음부터 체크된다");

    let building = result
        .records
        .iter()
        .find(|record| record.layer_name == "B0010000")
        .expect("B0010000.shp 레코드");
    assert_eq!(building.category, Category::Building);
    assert!(building.missing_prj, ".prj 없는 shp 는 동반 확인에 걸린다");
    assert!(building.crs_auth_id.is_empty());
    assert!(
        result
            .warnings
            .iter()
            .any(|line| line == &format!("좌표계 확인 필요: {} · B0010000", building.source)),
        "C++ :325 경고 문구"
    );

    let boundary = result
        .records
        .iter()
        .find(|record| record.layer_name == "G0010000")
        .expect("하위 폴더의 ngi 레코드");
    assert_eq!(boundary.category, Category::Boundary);
    assert_eq!(boundary.extent_label_ko(), "범위 없음");
    assert!(
        result
            .warnings
            .iter()
            .any(|line| line == &format!("도형 범위 없음: {} · G0010000", boundary.source)),
        "C++ :326 경고 문구"
    );
}

// C++ `KaTopographicImportDialog.cpp:465-470` — .prj 없는 DXF 는 [검토 필요], 자동 체크 없음.
#[test]
fn test_r14_dxf_without_prj_is_held_for_review_not_auto_checked() {
    let folder = TempFolder::new("r14_dxf_review");
    folder.write("등고선없음/35905086_행정경계.dxf", &dxf_bytes("G0010000"));
    folder.write("등고선있음/35905085_등고선.dxf", &dxf_bytes("F0010000"));
    folder.write(
        "등고선있음/35905085_등고선.prj",
        PRJ_5187_NAMED.as_bytes(),
    );

    let result = scan_folder(&folder.path);
    assert_eq!(result.error, None);
    assert_eq!(result.files_read, 2);
    assert_eq!(result.records.len(), 2, "DXF 한 파일이 CAD 레이어 하나다");

    let without_prj = result
        .records
        .iter()
        .find(|record| record.source.contains("등고선없음"))
        .expect("검토 필요 레코드");
    assert_eq!(without_prj.category, Category::Boundary);
    assert!(without_prj.crs_auth_id.is_empty());
    assert_eq!(
        without_prj.review_reason.as_deref(),
        Some(REVIEW_REASON_UNKNOWN_CRS)
    );
    assert!(
        !without_prj.initially_selected(),
        "C++ :469-470 은 검토 필요 항목을 체크하지 않는다"
    );
    assert!(
        without_prj.display_label_ko().ends_with(" [검토 필요]"),
        "C++ :466-467 은 이름 뒤에 [검토 필요]를 붙인다"
    );
    assert_eq!(
        without_prj.extent_label_ko(),
        "X 1000.0 ~ 1500.0\nY 2000.0 ~ 2500.0"
    );

    let with_prj = result
        .records
        .iter()
        .find(|record| record.source.contains("등고선있음"))
        .expect("좌표계를 읽은 레코드");
    assert_eq!(with_prj.category, Category::Contour);
    assert_eq!(with_prj.crs_auth_id, "EPSG:5187", "이름만 있는 ESRI 글도 읽는다");
    assert!(with_prj.review_reason.is_none());
    assert!(with_prj.initially_selected(), "등고선은 처음부터 체크된다");
    assert_eq!(with_prj.display_label_ko(), "등고선(미분류)");
    assert_eq!(with_prj.cad_layer, "F0010000");
}

// C++ `TopographicCatalog.cpp:260-263` — 폴더를 읽을 수 없을 때 문구.
#[test]
fn test_r14_unreadable_folder_reports_the_cpp_message() {
    let folder = TempFolder::new("r14_missing");
    let missing = folder.path.join("없는폴더");
    let result = scan_folder(&missing);
    assert_eq!(
        result.error.as_deref(),
        Some(
            format!(
                "수치지형도 폴더를 읽을 수 없습니다: {}",
                missing.to_string_lossy()
            )
            .as_str()
        )
    );
    assert!(result.records.is_empty());
    assert_eq!(result.files_read, 0);

    let empty = scan_folder(Path::new(""));
    assert!(empty.error.is_some(), "빈 경로도 같은 오류다");
}

// C++ `TopographicCatalog.cpp:89-103` — 이름이 카테고리 한글명이거나, 코드북 코드를 품었을 때.
