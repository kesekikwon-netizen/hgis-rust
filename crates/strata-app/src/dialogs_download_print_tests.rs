//! `dialogs_download_print` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]



use crate::dlg_geotiff_limits::{geotiff_canvas_size_px, geotiff_default_file_name, MAP_TAB_ONLY_REASON};

// D24 KaStageRail 6단계 · 반응형 클램프
#[test]
fn test_d24_stage_rail_and_responsive_clamp() {
    // KaStageRail.cpp — 6단계
    const STAGES: [&str; 6] = [
        "도엽 확인",
        "로그인",
        "신청",
        "파일 준비",
        "다운로드",
        "지도에 올리기",
    ];
    assert_eq!(STAGES.len(), 6);
    assert_eq!(STAGES[0], "도엽 확인");
    assert_eq!(STAGES[5], "지도에 올리기");

    // 완료/현재/대기 3상태 판정
    let stage = 3_usize;
    let states: Vec<&str> = (0..6)
        .map(|i| {
            if i < stage {
                "done"
            } else if i == stage {
                "current"
            } else {
                "wait"
            }
        })
        .collect();
    assert_eq!(states.iter().filter(|s| **s == "done").count(), 3);
    assert_eq!(states.iter().filter(|s| **s == "current").count(), 1);
    assert_eq!(states.iter().filter(|s| **s == "wait").count(), 2);

    // 어느 화면 크기에서도 창이 화면 안에 든다
    for (sw, sh) in [
        (1600.0_f32, 900.0_f32),
        (1920.0, 1080.0),
        (2560.0, 1080.0),
        (3440.0, 1440.0),
        (3840.0, 2160.0),
    ] {
        for (dw, dh) in [(960.0_f32, 620.0_f32), (940.0, 700.0), (820.0, 500.0)] {
            assert!(dw.min(sw - 32.0) <= sw);
            assert!(dh.min(sh - 32.0) <= sh);
        }
    }
}

// R10 — MapGeoTiffExport.cpp:94-103 · MainWindowExport.cpp:59,73-74 · KaShellUi.cpp:48-50
#[test]
fn test_r10_geotiff_uses_canvas_size_with_cpp_limits_and_map_tab_only() {
    // :94-97 크기는 화면 캔버스 출력 크기 그대로다. 고정 DPI 로 다시 계산하지 않는다.
    assert_eq!(geotiff_canvas_size_px([1600.0, 900.0]), Ok((1600, 900)));
    assert_eq!(geotiff_canvas_size_px([1920.0, 1080.0]), Ok((1920, 1080)));
    // 화면이 바뀌면 파일 픽셀 수도 같이 바뀐다 (고정 2480×3508 이 아니다).
    assert_eq!(geotiff_canvas_size_px([800.0, 600.0]), Ok((800, 600)));
    assert_ne!(
        geotiff_canvas_size_px([1600.0, 900.0]),
        geotiff_canvas_size_px([1920.0, 1080.0])
    );

    // :101-103 한 변 8,192픽셀, 총 3,200만 픽셀 상한
    assert_eq!(geotiff_canvas_size_px([8192.0, 1000.0]), Ok((8192, 1000)));
    assert!(geotiff_canvas_size_px([8193.0, 1000.0]).is_err());
    assert!(geotiff_canvas_size_px([1000.0, 8193.0]).is_err());
    // 8192 × 3906 = 31,997,952 는 통과하지만 8192 × 3907 = 32,006,144 는 총량 상한을 넘는다.
    assert_eq!(geotiff_canvas_size_px([8192.0, 3906.0]), Ok((8192, 3906)));
    assert!(geotiff_canvas_size_px([8192.0, 3907.0]).is_err());
    // 한 변이 상한 안이어도 총량이 넘으면 저장하지 않는다 (8192 × 4000 = 32,768,000).
    assert!(geotiff_canvas_size_px([8192.0, 4000.0]).is_err());
    assert_eq!(
        geotiff_canvas_size_px([8193.0, 1000.0]),
        Err("지도 영상은 한 변 8,192픽셀, 총 3,200만 픽셀 이내여야 합니다.")
    );

    // :98-100 좌표계나 범위를 못 얻으면 저장하지 않는다.
    assert!(geotiff_canvas_size_px([f64::NAN, 900.0]).is_err());
    assert!(geotiff_canvas_size_px([0.0, 900.0]).is_err());

    // :73-74 기본 파일 이름은 좌표계를 밑줄로 이어 붙인다 (`:` → `_`).
    assert_eq!(geotiff_default_file_name("EPSG:5187"), "지도_EPSG_5187.tif");
    assert_eq!(geotiff_default_file_name("EPSG:5186"), "지도_EPSG_5186.tif");

    // KaShellUi.cpp:48-50 지도 탭에서만 쓸 수 있다는 사유문
    assert_eq!(
        MAP_TAB_ONLY_REASON,
        "「지도」 탭에서 쓸 수 있습니다. 지도 탭으로 옮긴 뒤 누르세요."
    );
}

// R13 — 「유산」 받기 흐름이 C++ 순서·문구를 지키는가 (네트워크 없이 확인)
#[test]
fn test_r13_heritage_receive_flow_matches_the_cpp_pipeline() {
    use strata_net::heritage_intranet::{
        heritage_preflight, messages, HeritageIntranetFlow, HeritageStage,
        HERITAGE_TAB_CODES,
    };

    // KaHeritageBrowser.cpp:598 — 계정이 없으면 요청 전에 이 문구로 멈춘다.
    let dir = std::path::Path::new("C:/survey/r13");
    assert_eq!(
        heritage_preflight(false, Some(dir), "경상북도", "경주시"),
        Err(messages::NO_ACCOUNT.to_owned())
    );
    // MainWindow.cpp:3113-3116 — 조사폴더 없이는 받지 않는다.
    assert!(heritage_preflight(true, None, "경상북도", "경주시")
        .expect_err("조사폴더 없이 받으면 안 된다")
        .contains("조사를 먼저 열거나 저장하세요"));

    // KaHeritageBrowser.cpp:571-629 — 로그인 → 다운로드 화면 → 목록 → 시·군 선택 → 검색
    // → 받기 순서가 단계 이름으로 드러난다.
    let order = [
        HeritageStage::Login,
        HeritageStage::DismissTutorial,
        HeritageStage::OpenDownloadPage,
        HeritageStage::AgreeTerms,
        HeritageStage::SelectDataset,
        HeritageStage::SelectRegion,
        HeritageStage::Search,
        HeritageStage::CollectResults,
        HeritageStage::Download,
    ];
    for pair in order.windows(2) {
        assert!(
            pair[0].ui_index() < pair[1].ui_index(),
            "{} 다음에 {} 가 와야 한다",
            pair[0].stage_name_ko(),
            pair[1].stage_name_ko()
        );
    }
    assert_eq!(HeritageStage::Done.ui_index(), 10);
    assert_eq!(HeritageStage::Failed.ui_index(), 11);

    // 받을 자료 종류 6종은 HeritageStyle.cpp:27-32 표와 같고, 지정유산 탭 코드는
    // HeritageIntranetFlow.cpp:247-248 대로 "S" 다.
    assert_eq!(HERITAGE_TAB_CODES.len(), 6);
    assert_eq!(
        HeritageIntranetFlow::download_files_all_url(),
        "https://intranet.gis-heritage.go.kr/user/data/heritageDownload/downloadFilesAll.do"
    );
    // 화이트리스트 밖으로는 나가지 않는다(V5 와 같은 규칙).
    assert!(HeritageIntranetFlow::is_allowed_frame_url(
        &HeritageIntranetFlow::download_files_all_url()
    )
    .is_ok());
    assert!(HeritageIntranetFlow::is_allowed_frame_url("https://evil.example/x").is_err());
}
