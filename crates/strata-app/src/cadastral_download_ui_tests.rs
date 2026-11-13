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
        let folder = tempfile::tempdir().expect("temp dir");
        let survey = folder.path().join("경주_월성북편.gpkg");
        std::fs::write(&survey, b"").expect("survey file");
        let dir = cadastral_raw_dir(Some(&survey)).expect("조사 파일이 있으면 경로가 나온다");
        assert!(dir.ends_with(r"지적도\원본"), "{}", dir.display());
        assert!(dir.starts_with(folder.path()), "{}", dir.display());
        assert_eq!(cadastral_raw_dir(None), None);
        // F1c-4 — ReferenceStorage.cpp:39-43: a survey path whose file is missing has no folder.
        assert_eq!(cadastral_raw_dir(Some(std::path::Path::new("Z:/없는/조사.gpkg"))), None);
    }

    #[test]
    fn test_cadastral_no_key_message_is_cpp_text() {
        assert_eq!(
            VworldClient::NO_KEY_MESSAGE,
            "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣으세요."
        );
        assert_eq!(
            VworldClient::NO_ACCOUNT_MESSAGE,
            "5km 범위의 시·군·구를 찾으려면 더보기 → API 키 입력에서 VWorld 키를 설정하세요."
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

/// F1 — 지적도 받기 범위는 열어 둔 조사구역 도형들의 바깥 상자(MainWindowCadastral.cpp:93-115, 5km 는 그 둘레),
/// 경주 기본점은 없다. 조사구역이 없으면 C++ 처럼 받지 않고 그 까닭을 보인다.
#[test]
fn cadastral_scope_is_the_survey_area() {
    use crate::cadastral_download_ui::cadastral_download_ui_state::{start_download, CadastralState, NO_SURVEY_AREA_MESSAGE};
    use crate::survey_area_setup::survey_area_extent;
    let mut app = crate::app_state::StrataAppState::new_startup();
    assert_eq!(survey_area_extent(&app), None, "no survey open");
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = dir.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &gpkg).expect("copy");
    app.open_survey_gpkg(&gpkg);
    let areas: Vec<[f64; 4]> = app
        .survey_tables
        .iter()
        .filter(|t| crate::survey_area_setup::is_survey_area_key(&t.table))
        .filter_map(|t| crate::survey_paint_layers::table_extent_in(t, app.work_crs))
        .collect();
    assert!(!areas.is_empty(), "the demo has a survey area");
    let want = areas.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, e| [b[0].min(e[0]), b[1].min(e[1]), b[2].max(e[2]), b[3].max(e[3])]);
    assert_eq!(survey_area_extent(&app), Some(want));
    for t in app.survey_tables.iter_mut().filter(|t| crate::survey_area_setup::is_survey_area_key(&t.table)) {
        t.layer.features.clear();
    }
    let mut state = CadastralState::idle();
    start_download(&app, &mut state);
    assert_eq!((state.status.as_str(), state.running), (NO_SURVEY_AREA_MESSAGE, false));
}

/// F1c-4 — MainWindowCadastral.cpp:85-87: a survey path whose file is missing stops with the cpp text and
/// writes nothing; after 다른 이름으로 저장 the folder follows the new survey path (:121).
#[test]
fn cadastral_needs_an_existing_survey_file_and_follows_save_as() {
    use crate::cadastral_download_ui::cadastral_download_ui_state::{start_download, CadastralState};
    let a = tempfile::tempdir().expect("temp dir");
    let b = tempfile::tempdir().expect("temp dir");
    let mut app = crate::app_state::StrataAppState::new_startup();
    app.survey_path = Some(a.path().join("저장 안 됨.gpkg"));
    let mut state = CadastralState::idle();
    start_download(&app, &mut state);
    assert_eq!((state.status.as_str(), state.running), (VworldClient::NO_SURVEY_MESSAGE, false));
    assert!(std::fs::read_dir(a.path()).expect("list").next().is_none(), "nothing written");
    std::fs::write(a.path().join("조사.gpkg"), b"").expect("survey a");
    let moved = b.path().join("조사.gpkg");
    std::fs::write(&moved, b"").expect("survey b");
    app.survey_path = Some(moved);
    let raw = cadastral_raw_dir(app.survey_path.as_deref()).expect("folder");
    assert!(raw.starts_with(b.path()), "{}", raw.display());
}
