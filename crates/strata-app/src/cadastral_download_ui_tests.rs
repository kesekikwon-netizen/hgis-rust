//! `cadastral_download_ui` 시험 — 저장 폴더 규칙·C++ 문구·시군구 파싱·좌표 변환.
//!
//! 원래 `cadastral_download_ui.rs` 안 `mod tests` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 가 창구를 가리키지 않으므로 명시적으로 가져온다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_net::vworld_client::VworldClient;
use strata_types::crs::WorkCrs;

use crate::cadastral_download_ui::cadastral_download_ui_worker::with_page;
use crate::cadastral_download_ui::{
    cadastral_district_error_ko, cadastral_raw_dir, cadastral_success_message_ko,
    cadastral_zip_size_error_ko, parse_sigungu_features, work_to_portal_xy,
};

    // C++ `MainWindowCadastral.cpp:121` — 조사 폴더 안 `지적도/원본`
    #[test]
    fn test_cadastral_raw_dir_follows_cpp_folder_rule() {
        let survey = std::path::Path::new(r"C:\조사\경주_월성북편.gpkg");
        let dir = cadastral_raw_dir(Some(survey)).expect("조사 파일이 있으면 경로가 나온다");
        assert!(dir.ends_with(r"지적도\원본"), "{}", dir.display());
        assert!(dir.starts_with(r"C:\조사"), "{}", dir.display());
        assert_eq!(cadastral_raw_dir(None), None);
    }

    #[test]
    fn test_cadastral_no_key_message_is_cpp_text() {
        assert_eq!(
            VworldClient::NO_KEY_MESSAGE,
            "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣으세요."
        );
        // C++ `CadastralPortal.cpp:216` — 누리집 아이디·비밀번호가 없을 때의 문구.
        // (예전에는 여기에 `:214` 의 「키가 없을 때」 문구가 잘못 들어 있었다.)
        assert_eq!(
            VworldClient::NO_ACCOUNT_MESSAGE,
            "지적도 메뉴에서 VWorld 계정을 설정하세요."
        );
        assert_eq!(cadastral_zip_size_error_ko(), "해당 자료는 자동 수신 가능한 시·군·구 ZIP 크기를 벗어났습니다.");
    }

    // C++ `CadastralPortal.cpp:249-258` — 5자리 코드 + full_nm
    #[test]
    fn test_parse_sigungu_features_keeps_valid_districts() {
        let body = r#"{"response":{"status":"OK","result":{"featureCollection":{"features":[
            {"properties":{"sig_cd":"47110","full_nm":"경상북도 경주시"}},
            {"properties":{"sig_cd":"47110","full_nm":"경상북도 경주시"}},
            {"properties":{"sig_cd":"47111","full_nm":"경상북도 포항시"}}]}}}}"#;
        let rows = parse_sigungu_features(body.as_bytes()).expect("parse");
        assert_eq!(rows.len(), 2, "중복 코드는 한 번만: {rows:?}");
        assert_eq!(rows[0].code, "47110");
        assert_eq!(rows[1].name, "경상북도 포항시");

        // 코드가 5자리가 아니면 C++ `:255` 처럼 오류
        let bad = r#"{"response":{"status":"OK","result":{"featureCollection":{"features":[
            {"properties":{"sig_cd":"4711","full_nm":"x"}}]}}}}"#;
        assert_eq!(
            parse_sigungu_features(bad.as_bytes()).expect_err("5자리 코드가 아니면 오류"),
            cadastral_district_error_ko()
        );
    }

    #[test]
    fn test_cadastral_success_message_names_zip_and_district_counts() {
        let message = cadastral_success_message_ko(2, 2);
        assert!(message.starts_with("조사 주변 5km 지적도를 추가했습니다."));
        assert!(message.contains("시·군·구 2곳의 원본 ZIP 2개를 받았습니다."));
    }

    // C++ `MainWindowDownloads.cpp:183-186` — 공식 신청 좌표계(EPSG:5179)
    #[test]
    fn test_work_to_portal_xy_converts_5187_center_to_5179() {
        // 경주 일대 동부원점 좌표
        let out = work_to_portal_xy([220_000.0, 360_000.0], WorkCrs::Epsg5187).expect("변환");
        assert!(out[0].is_finite() && out[1].is_finite());
        // 5179 는 false easting 1,000,000 이므로 x 가 1,000,000 근처
        assert!((800_000.0..1_200_000.0).contains(&out[0]), "{out:?}");
        assert!((1_400_000.0..2_100_000.0).contains(&out[1]), "{out:?}");
        assert!(work_to_portal_xy([f64::NAN, 0.0], WorkCrs::Epsg5187).is_none());
    }

    #[test]
    fn test_with_page_replaces_the_cpp_page_parameter() {
        let url = VworldClient::districts_url("K", [127.0, 35.0, 128.0, 36.0]);
        assert!(with_page(&url, 3).contains("&page=3&"));
        assert!(!with_page(&url, 3).contains("&page=1&"));
    }
