//! `vworld_client` 시험 1/2 — WMTS·WMS 주소와 연속지적도 포털.
//!
//! 원래 `vworld_client.rs` 안 `mod tests` 를 주제별로 나눴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn vworld_cadastral_wms_url_never_sends_domain_and_detects_xml_error_on_http_200() {
    let url = VworldClient::cadastral_wms_url(
        "TEST_KEY",
        [14135000.0, 4518000.0, 14136000.0, 4519000.0],
        "EPSG:3857",
    );
    assert!(url.contains("crs=EPSG:3857"));
    assert!(url.contains("KEY=TEST_KEY"));
    assert!(!url.to_lowercase().contains("domain="));
    let xml_body = b"<?xml version=\"1.0\"?><ServiceExceptionReport><ServiceException code=\"INCORRECT_KEY\">Invalid Key</ServiceException></ServiceExceptionReport>";
    assert!(VworldClient::is_xml_error_tile(xml_body));
    assert_eq!(
        VworldClient::classify_service_exception(xml_body),
        Some(VworldError::InvalidKey)
    );
}

#[test]
fn test_vworld_cadastral_layer_uri_matches_cpp_make_vworld_wms_uri() {
    // C++ `BasemapOps.cpp:1344-1349` — layers 는 본번,부번 순서, styles 는 빈 값
    let uri = VworldClient::cadastral_layer_uri(
        "TEST_KEY",
        "lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun",
        "",
        "EPSG:3857",
    );
    assert!(uri.starts_with("IgnoreGetMapUrl=1&IgnoreGetFeatureInfoUrl=1&contextualWMSLegend=0"));
    assert!(uri.contains("&crs=EPSG:3857&"));
    assert!(uri.contains("&dpiMode=7&"));
    assert!(uri.contains("&format=image/png&"));
    assert!(uri.contains("&transparent=true&"));
    // C++ 과 같은 타일 규격
    assert!(uri.contains("&tilePixelRatio=2&stepWidth=512&stepHeight=512&"));
    // 인증은 Referer, DOMAIN 은 절대 보내지 않는다(CLAUDE.md 절대 규칙)
    assert!(uri.contains("&http-header:referer=https://localhost&"));
    assert!(!uri.to_uppercase().contains("DOMAIN="));
    // layers 순서·빈 styles
    assert!(uri.contains("&layers=lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun&styles&"));
    assert!(uri.contains("&url=https%3A%2F%2Fapi.vworld.kr%2Freq%2Fwms%3FKEY%3DTEST_KEY"));
}

#[test]
fn test_vworld_cadastral_layer_uri_crs_candidates_are_only_4326_3857_900913() {
    // C++ `BasemapOps.cpp:1327-1336`
    assert_eq!(
        VworldClient::cadastral_wms_crs_candidates(),
        ["EPSG:4326", "EPSG:3857", "EPSG:900913"]
    );
    for forbidden in ["EPSG:5186", "EPSG:5187", "EPSG:5179"] {
        let uri = VworldClient::cadastral_layer_uri("K", "lp_pa_cbnd_bonbun", "", forbidden);
        assert!(
            uri.contains("&crs=EPSG:3857&"),
            "{forbidden} 은 3857 로 떨어져야 한다: {uri}"
        );
        assert!(!uri.contains(forbidden));
    }
    // 후보 안의 값은 그대로 쓴다.
    let uri = VworldClient::cadastral_layer_uri("K", "lp_pa_cbnd_bubun", "", "EPSG:4326");
    assert!(uri.contains("&crs=EPSG:4326&"));
}

#[test]
fn test_vworld_cadastral_gdal_wms_xml_has_no_domain_and_keeps_cpp_extent() {
    // C++ `BasemapOps.cpp:1236-1269`
    let xml = VworldClient::cadastral_gdal_wms_xml("TEST_KEY", "lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun");
    assert!(xml.starts_with("<GDAL_WMS><Service name=\"WMS\">"));
    assert!(xml.contains("<Version>1.3.0</Version>"));
    assert!(xml.contains("<ServerUrl>https://api.vworld.kr/req/wms?key=TEST_KEY&amp;</ServerUrl>"));
    assert!(xml.contains("<Layers>lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun</Layers>"));
    assert!(xml.contains("<CRS>EPSG:3857</CRS>"));
    assert!(xml.contains("<Referer>https://localhost</Referer>"));
    // domain= 은 해가 되기만 한다 — 넣지 않는다.
    assert!(!xml.to_lowercase().contains("domain="));
    // C++ 과 같은 DataWindow 상수
    assert!(xml.contains("<UpperLeftX>13500000</UpperLeftX>"));
    assert!(xml.contains("<LowerRightY>3800000</LowerRightY>"));
    assert!(xml.contains("<SizeX>2600000</SizeX>"));
    assert!(xml.contains("<SizeY>2000000</SizeY>"));
    assert!(xml.contains("<BlockSizeX>512</BlockSizeX>"));
}

#[test]
fn test_vworld_cadastral_messages_match_cpp_when_key_missing() {
    // C++ `BasemapOps.cpp:1015-1016` requireVworldKey
    assert_eq!(
        VworldClient::NO_KEY_MESSAGE,
        "VWorld API 키가 없습니다. 리본의 더보기 → API 키 입력에서 키를 넣으세요."
    );
    // C++ `MainWindowCadastral.cpp:86,114,122-123`
    assert_eq!(
        VworldClient::NO_SURVEY_MESSAGE,
        "먼저 조사를 열거나 저장하고 조사구역을 그려 주세요."
    );
    assert_eq!(
        VworldClient::NO_SCOPE_MESSAGE,
        "조사구역을 먼저 그려 주세요. 경계에서 주변 5km의 지적도를 받습니다."
    );
    assert_eq!(VworldClient::CADASTRAL_DIALOG_TITLE, "지적도 다운로드");
    assert_eq!(
        VworldClient::CADASTRAL_DIALOG_SUBTITLE,
        "VWorld · 조사구역 경계에서 주변 5km의 지적도를 준비합니다."
    );
    // C++ `CadastralPortal.cpp:99` ZIP 판정 문구
    assert!(CADASTRAL_NOT_ZIP_MESSAGE.contains("ZIP 대신 로그인·오류 화면"));
}

#[test]
fn test_vworld_cadastral_urls_match_cpp_portal() {
    // C++ `CadastralPortal.cpp:322-323`
    assert_eq!(
        VworldClient::cadastral_zip_url(" 987654 "),
        "https://www.vworld.kr/dtmk/downloadResourceFile.do?ds_id=30563&fileNo=987654"
    );
    // C++ `CadastralPortal.cpp:295-297`
    assert_eq!(
        VworldClient::cadastral_dataset_page_url(3, 100),
        "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563&datPageIndex=3&datPageSize=100"
    );
    // 페이지/크기 하한
    assert!(VworldClient::cadastral_dataset_page_url(0, 0).contains("datPageIndex=1&datPageSize=1"));
}

#[test]
fn test_vworld_parse_cadastral_resources_reads_zip_rows() {
    // C++ `CadastralPortal.cpp:157-172` — 이름·링크·크기·갱신일
    let html = r##"
    <ul>
      <li><a href="#" onclick="listFnc.download('30563','111222','512');">LSMD_CONT_LDREG_경북_경주시.zip</a>
          <span>갱신일 <em>2026-01-05</em></span></li>
      <li><a href="#" onclick="listFnc.download('30563','333444','2048');">LSMD_CONT_LDREG_경북_포항시.zip</a></li>
      <li><a href="#">LSMD_CONT_LDREG_경북_안동시.zip</a></li>
    </ul>"##;
    let rows = parse_cadastral_resources(html);
    assert_eq!(rows.len(), 2, "링크 없는 줄은 버린다: {rows:?}");
    assert_eq!(rows[0].file_no, "111222");
    assert_eq!(rows[0].name, "LSMD_CONT_LDREG_경북_경주시.zip");
    assert_eq!(rows[0].size_kb, 512);
    assert_eq!(rows[0].revision, "2026-01-05");
    assert_eq!(rows[1].file_no, "333444");
    assert_eq!(rows[1].revision, "");
}
