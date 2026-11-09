//! `topographic_catalog` 시험 2/2 — 코드북 추정·요약 문구·PRJ 좌표계 읽기.
//!
//! 원래 `topographic_catalog.rs` 안 `mod tests` 를 주제별로 나눴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use super::test_support::*;


#[test]
fn test_r14_infer_code_matches_the_cpp_codebook_rules() {
    assert_eq!(infer_code("등고선"), (Category::Contour, "등고선".to_owned()));
    assert_eq!(infer_code(" 표고점 ").0, Category::ElevationPoint);

    let (category, name) = infer_code("N3L_F0010000_등고선도엽");
    assert_eq!(category, Category::Contour);
    assert_eq!(name, "등고선(미분류)");

    // 카테고리가 서로 다른 코드가 섞이면 판정을 포기한다(C++ :99).
    assert_eq!(
        infer_code("F0010000_A0010000"),
        (Category::Unknown, String::new())
    );
    // 같은 카테고리 코드 둘은 뒤엣것을 쓴다(C++ 은 마지막 일치를 남긴다).
    assert_eq!(infer_code("A0010000_A0013111").0, Category::Road);
    // 코드북에 없는 코드는 미분류.
    assert_eq!(
        infer_code("Z9999999"),
        (Category::Unknown, String::new())
    );
    // 글자에 붙은 8자 토큰은 코드가 아니다(C++ 전후방 경계 규칙).
    assert_eq!(infer_code("xF00100001").0, Category::Unknown);
    // 파일 이름처럼 카테고리 이름을 품기만 해도 코드가 아니면 미분류다.
    assert_eq!(infer_code("35905085_등고선").0, Category::Unknown);
}

// C++ `KaTopographicImportDialog.cpp:494-495, 529-537` — 상태줄과 추가 거부 문구.
#[test]
fn test_r14_summary_and_blocking_messages_match_the_cpp_dialog() {
    let folder = TempFolder::new("r14_messages");
    folder.write("F0010000.shp", &shp_bytes(Extent {
        min_x: 10.0,
        min_y: 20.0,
        max_x: 30.0,
        max_y: 40.0,
    }));
    folder.write("F0010000.prj", PRJ_5186.as_bytes());
    let result = scan_folder(&folder.path);
    assert_eq!(
        result.summary_ko(),
        "레이어 1개 · 파일 1개 확인 · 캐시 0개. 추가할 항목에 체크하세요.\n"
    );

    let record = &result.records[0];
    assert_eq!(blocking_message_ko(&[record]), None);

    let extentless = Record {
        extent: None,
        ..record.clone()
    };
    assert_eq!(
        blocking_message_ko(&[&extentless]),
        Some(IMPORT_NEEDS_EXTENT),
        "범위를 먼저 본다"
    );

    let crsless = Record {
        crs_auth_id: String::new(),
        ..record.clone()
    };
    assert_eq!(blocking_message_ko(&[&crsless]), Some(IMPORT_NEEDS_CRS));
    assert_eq!(blocking_message_ko(&[]), None, "체크가 없으면 막지 않는다");
}

// `.prj` 좌표계 읽기 — 권위 번호가 있으면 그것, 없으면 원점 이름(C++ `CadCrsHints.cpp:31-45`).
#[test]
fn test_r14_prj_wkt_reads_authority_number_then_origin_name() {
    assert_eq!(
        crs_auth_id_from_wkt(PRJ_5186).as_deref(),
        Some("EPSG:5186")
    );
    assert_eq!(
        crs_auth_id_from_wkt(PRJ_5187_NAMED).as_deref(),
        Some("EPSG:5187"),
        "이름에 EAST 가 있으면 동부원점(GRS80)"
    );
    assert_eq!(
        crs_auth_id_from_wkt("PROJCS[\"Korea_2000_Korea_Central_Belt\",\
             GEOGCS[\"Korea 2000\",DATUM[\"D\",SPHEROID[\"Bessel_1841\",6377397.155,299.1528]]]]")
            .as_deref(),
        Some("EPSG:5174"),
        "베셀 원점은 보정 베셀 번호를 쓴다"
    );
    assert_eq!(crs_auth_id_from_wkt("LOCAL_CS[\"없는 좌표계\"]"), None);
}

// C++ `TopographicCatalog.cpp:50-52` — 대소문자가 달라도 같은 이름의 곁 파일을 찾는다.
#[test]
fn test_r14_companion_prj_is_case_insensitive() {
    let folder = TempFolder::new("r14_prj_case");
    folder.write("등고선.SHP", &shp_bytes(Extent {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 1.0,
        max_y: 1.0,
    }));
    folder.write("등고선.PrJ", PRJ_5186.as_bytes());
    let result = scan_folder(&folder.path);
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].crs_auth_id, "EPSG:5186");
    assert!(!result.records[0].missing_prj);
    assert_eq!(result.records[0].crs_epsg(), Some(5186));
}
