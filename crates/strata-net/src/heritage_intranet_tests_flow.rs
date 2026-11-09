//! `heritage_intranet` 시험 2/2 — 문구·단계·사전 점검·재시도.
//!
//! 원래 `heritage_intranet.rs` 안 `mod tests` 를 주제별로 나눴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::path::Path;

#[test]
fn heritage_stage_names_and_ui_indices_match_the_cpp_order() {
    // HeritageIntranetFlow.cpp:94-110 — 단계 이름을 그대로 쓴다.
    let all = [
        (HeritageStage::Idle, "대기"),
        (HeritageStage::Login, "로그인"),
        (HeritageStage::DismissTutorial, "튜토리얼 닫기"),
        (HeritageStage::OpenDownloadPage, "국가유산자료 다운로드 열기"),
        (HeritageStage::AgreeTerms, "서약서 동의"),
        (HeritageStage::SelectDataset, "자료 종류 선택"),
        (HeritageStage::SelectRegion, "시·군 선택"),
        (HeritageStage::Search, "검색"),
        (HeritageStage::CollectResults, "결과 모으기"),
        (HeritageStage::Download, "내려받기"),
        (HeritageStage::Done, "완료"),
        (HeritageStage::Failed, "실패"),
    ];
    for (index, (stage, name)) in all.into_iter().enumerate() {
        assert_eq!(stage.stage_name_ko(), name);
        assert_eq!(stage.ui_index(), index);
    }
}

#[test]
fn heritage_tab_codes_match_heritage_style_and_never_guess_designated() {
    // HeritageStyle.cpp:27-32 의 표 + HeritageIntranetFlow.cpp:247-248 의 지정유산 → "S"
    assert_eq!(heritage_tab_code("지정유산"), "S");
    assert_eq!(heritage_tab_code("현상변경허용기준"), "P");
    assert_eq!(heritage_tab_code("매장유산유존지역"), "B");
    assert_eq!(heritage_tab_code("문화유적분포지도"), "U");
    assert_eq!(heritage_tab_code("지표조사구역"), "R");
    assert_eq!(heritage_tab_code("발굴조사구역"), "E");
    assert_eq!(heritage_tab_code("없는자료"), "");
}

#[test]
fn heritage_preflight_blocks_like_the_cpp_before_any_request() {
    let dir = Path::new("C:/survey/test");
    // KaHeritageBrowser.cpp:598 — 계정 없음
    assert_eq!(
        heritage_preflight(false, Some(dir), "경상북도", "경주시"),
        Err(messages::NO_ACCOUNT.to_owned())
    );
    // KaHeritageBrowser.cpp:585-588 — 시·군 미정
    assert_eq!(
        heritage_preflight(true, Some(dir), "", "경주시"),
        Err("받을 시·군이 정해지지 않았습니다.".to_owned())
    );
    // MainWindow.cpp:3113-3116 — 조사폴더 없음
    assert_eq!(
        heritage_preflight(true, None, "경상북도", "경주시"),
        Err("조사를 먼저 열거나 저장하세요.\n받은 자료는 그 조사폴더 안에만 둡니다.".to_owned())
    );
    assert!(heritage_preflight(true, Some(dir), "경상북도", "경주시").is_ok());
}

#[test]
fn heritage_download_root_keeps_files_inside_the_survey_folder() {
    // MainWindow.cpp:3191-3192 — 주변유적/<시도 시군>/원본
    let root = heritage_download_root(Path::new("C:/survey/경주"), " 경상북도 ", "경주시");
    let text = root.to_string_lossy().replace('\\', "/");
    assert!(text.ends_with("주변유적/경상북도 경주시/원본"), "{text}");
    assert!(text.starts_with("C:/survey/경주"), "{text}");
}

#[test]
fn heritage_retry_classification_matches_the_cpp_download_retry() {
    // KaHeritageBrowser.cpp:1357-1373 — 이 실패들은 다시 받는다.
    assert!(is_retryable_download_error(
        "파일을 끝까지 받지 못했습니다: 네트워크 오류"
    ));
    assert!(is_retryable_download_error(
        "다운로드 완료 신호를 받았지만 저장된 파일이 없거나 비어 있습니다."
    ));
    assert!(is_retryable_download_error("예상·수신·저장 파일 크기가 일치하지 않습니다."));
    // 계정·설정 문제는 다시 받아도 같으므로 곧바로 실패다.
    assert!(!is_retryable_download_error(messages::NO_ACCOUNT));
    assert!(!is_retryable_download_error(messages::NO_SESSION));
}

#[test]
fn heritage_http_search_without_session_stops_with_the_cpp_message() {
    // 네트워크를 타기 전에 막히는 경로만 확인한다(계정 없음).
    let err = heritage_http_search(
        Some(Path::new("C:/survey/test")),
        false,
        "경상북도",
        "경주시",
        None,
    )
    .expect_err("계정이 없으면 요청하지 않고 멈춰야 한다");
    assert_eq!(err, messages::NO_ACCOUNT);
    // 조사폴더가 없으면 역시 요청하지 않는다.
    let err = heritage_http_search(None, true, "경상북도", "경주시", None)
        .expect_err("조사폴더가 없으면 요청하지 않고 멈춰야 한다");
    assert!(err.contains("조사를 먼저 열거나 저장하세요"));
}

#[test]
fn heritage_outcome_detail_reads_like_the_browser_status() {
    let outcome = HeritagePipelineOutcome {
        matched_sido: "경상북도".to_owned(),
        matched_city: "경주시".to_owned(),
        search_count: 90,
        files: vec![HeritageFileResult {
            label: "지정유산".to_owned(),
            url: String::new(),
            saved_path: Some(std::path::PathBuf::from("C:/x/지정유산.zip")),
            bytes: 1024,
            error: None,
        }],
        ..HeritagePipelineOutcome::default()
    };
    assert_eq!(
        outcome.detail_ko(),
        "경상북도 경주시 — 검색결과 90건 · 파일 1개 받음"
    );
}
