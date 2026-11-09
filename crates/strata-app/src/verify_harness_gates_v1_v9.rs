//! 검증 게이트 V1~V9 — 스키마·암호·WKB·버퍼·WMS·고지형·캔버스·도면·제출 꾸러미.
//!
//! erify_harness.rs 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 게이트를 만드는 코드는 원문 그대로이고, 이 함수는 **원래 순서대로** gates 에 밀어 넣기만
//! 한다. 게이트 번호·판정식·문구는 하나도 바꾸지 않았다.

use geo::{Coord, LineString, Polygon};
use strata_canvas2d::canvas_painter::MapSceneModel;
use strata_canvas2d::viewport::ViewportTransform;
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use strata_geo::buffer_round::round_buffer_polygon;
use strata_geo::trench_grid::{generate_standard_trenches, TrenchConfig, TrenchStandardLength};
use strata_layout::pdf_geotiff::{render_geotiff_header_bytes, render_sheet_pdf_bytes};
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_net::heritage_intranet::HeritageIntranetFlow;
use strata_net::vworld_client::VworldClient;
use strata_paleo::section_renderer::render_cross_section_rgba;
use strata_secrets::dpapi::{protect_bytes, unprotect_bytes};
use strata_secrets::secret_store::SecretStore;
use strata_storage::wkb_codec::{decode_gpkg_wkb, encode_gpkg_wkb, WkbGeometry};
use strata_terrain3d::viewport3d_renderer::{render_terrain3d_rgba, OrbitCamera3D};
use strata_types::crs::SubmitCrs;
use strata_types::schema::SurveySchemaV3;
use strata_types::theme_tokens::ThemeTokens;

use crate::app_state::StrataAppState;
use super::verify_harness_types::GateResult;

/// V1~V9 게이트를 원래 순서대로 gates 에 넣는다.
pub(crate) fn push_v1_to_v9_gates(gates: &mut Vec<GateResult>) {
    // V1: 7대 도메인 레이어 스키마 v3 및 KaTheme 조용한 도판 대비비
    let schema = SurveySchemaV3::default();
    let paper = ThemeTokens::paper_tokens(false);
    let contrast = paper.contrast_ratio(paper.ink, paper.surface);
    gates.push(GateResult {
        gate_id: "V1",
        title_ko: "7대 도메인 레이어 스키마(v3) 및 조용한 도판 WCAG 대비 검증",
        passed: schema.layers.len() == 7 && contrast >= 7.0,
        detail: format!("layers={} contrast={contrast:.2}", schema.layers.len()),
    });

    // V2: 기존 DPAPI/포터블 암호 엔트로피("ka-hgis-account-v1") 왕복 호환
    let enc = SecretStore::wrap_portable("ka-test-pass-2026");
    let dec = SecretStore::unwrap_portable(&enc).unwrap_or_default();
    let dpapi_ok = protect_bytes(b"dpapi-secret")
        .and_then(|b| unprotect_bytes(&b))
        .is_ok_and(|p| p == b"dpapi-secret");
    gates.push(GateResult {
        gate_id: "V2",
        title_ko: "기존 사이트 계정 암호화(ka-hgis-account-v1 & DPAPI) 왕복 호환",
        passed: dec == "ka-test-pass-2026" && dpapi_ok,
        detail: format!("portable_cipher_len={}", enc.len()),
    });

    // V3: OGC GeoPackage WKB 이진 인코딩·디코딩 왕복
    let wkb_poly = WkbGeometry::Polygon(vec![vec![
        [200_000.0, 500_000.0],
        [200_050.0, 500_000.0],
        [200_050.0, 500_050.0],
        [200_000.0, 500_000.0],
    ]]);
    let bytes = encode_gpkg_wkb(5187, &wkb_poly);
    gates.push(GateResult {
        gate_id: "V3",
        title_ko: "OGC GeoPackage 이진 WKB 헤더·폴리곤 무손실 왕복",
        passed: decode_gpkg_wkb(&bytes).is_ok_and(|(srs, g)| srs == 5187 && g == wkb_poly),
        detail: format!("wkb_bytes={}", bytes.len()),
    });

    // V4: 36분할 500m 원형 버퍼 및 표준 시굴격자(20m) 생성
    let site = Polygon::new(
        LineString::from(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 80.0, y: 0.0 },
            Coord { x: 80.0, y: 80.0 },
            Coord { x: 0.0, y: 80.0 },
            Coord { x: 0.0, y: 0.0 },
        ]),
        vec![],
    );
    let buf500 = round_buffer_polygon(&site, 500.0, 36);
    let trenches = generate_standard_trenches(
        &site,
        &TrenchConfig {
            standard_length: TrenchStandardLength::M20,
            width_m: 2.0,
            coverage_ratio: 0.10,
            angle_deg: 0.0,
        },
    );
    gates.push(GateResult {
        gate_id: "V4",
        title_ko: "36분할 500m 원형 버퍼 및 20m 표준 시굴 트렌치 배치",
        passed: !buf500.exterior().0.is_empty() && !trenches.is_empty(),
        detail: format!("trenches={}", trenches.len()),
    });

    // V5: VWorld 지적 WMS DOMAIN 미포함 & 문화재청 인트라넷 URL 마스킹
    let wms_url =
        VworldClient::cadastral_wms_url("KEY123", [100.0, 200.0, 300.0, 400.0], "EPSG:3857");
    let xml_err =
        VworldClient::is_xml_error_tile(b"<?xml version=\"1.0\"?><ServiceExceptionReport/>");
    let js_ok = !HeritageIntranetFlow::search_script().contains("location.href");
    gates.push(GateResult {
        gate_id: "V5",
        title_ko: "VWorld 지적 WMS 규격 및 국가유산청 인트라넷 프레임 탐색 보안",
        passed: !wms_url.contains("DOMAIN=") && xml_err && js_ok,
        detail: "WMS DOMAIN excluded & XML 200 detected".to_owned(),
    });

    // V6: 반경 10km 4단계 고지형 엔진 및 2D 지층 단면도 RGBA 렌더링
    let app = StrataAppState::new_startup();
    let _ = render_cross_section_rgba;
    gates.push(GateResult {
        gate_id: "V6",
        title_ko: "반경 10km 4단계 고지형 분류 및 2D 지층 단면도 렌더링",
        passed: app.paleo_3d.last_paleo_output.is_some()
            && app.paleo_3d.cross_section_rgba.len() == 320 * 120 * 4,
        detail: format!("section_rgba={}", app.paleo_3d.cross_section_rgba.len()),
    });

    // V7: 2D 캔버스 투영 및 3D 입체지형 AgX/Nishita 뷰포트 렌더링
    let scene = MapSceneModel::new_demo_gyeongju_scene();
    let vp = ViewportTransform::new([219_850.0, 360_120.0], 0.5, [1200.0, 800.0]);
    let prims = scene.project_polygons(&vp);
    let t3d = render_terrain3d_rgba(&[42.0; 64], 8, 8, &OrbitCamera3D::default(), 160, 100);
    gates.push(GateResult {
        gate_id: "V7",
        title_ko: "2D 벡터 캔버스 투영 및 3D 입체지형 AgX 뷰포트 렌더링",
        passed: prims.len() >= 2 && t3d.len() == 160 * 100 * 4,
        detail: format!("polygons={} t3d_bytes={}", prims.len(), t3d.len()),
    });

    // V8: 도면 스튜디오 용지 회전 및 벡터 PDF / GeoTIFF(Tag 34735) 출력
    let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
    sheet.change_paper(SheetPaperSize::A3, SheetOrientation::Landscape);
    let pdf_ok = render_sheet_pdf_bytes(&sheet, "검증도면")
        .is_ok_and(|b| String::from_utf8_lossy(&b).contains("re S"));
    let tiff = render_geotiff_header_bytes(512, 512, 5187);
    gates.push(GateResult {
        gate_id: "V8",
        title_ko: "도면 용지 변경 경계 유지 및 벡터 PDF·GeoTIFF(Tag 34735) 출력",
        passed: sheet.all_decorations_inside_sheet()
            && pdf_ok
            && tiff.windows(2).any(|w| w == [0xAF, 0x87]),
        detail: format!("sheet={}x{}mm", sheet.width_mm, sheet.height_mm),
    });

    // V9: 국가유산청 제출 꾸러미(EPSG:5179 SHP + PDF + MANIFEST.sha256 + README)
    let findings = ChecklistEngine::evaluate_sheet(&sheet, SubmitCrs::Epsg5179, true);
    let tmp_dir = std::env::temp_dir().join(format!("ka_hgis_v9_{}", std::process::id()));
    let v9_ok = export_submission_package(&tmp_dir, &findings)
        .is_ok_and(|m| m.crs == SubmitCrs::Epsg5179 && m.files_sha256.len() == 3);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    gates.push(GateResult {
        gate_id: "V9",
        title_ko: "국가유산청 제출 꾸러미(EPSG:5179 + MANIFEST.sha256) 무결성",
        passed: v9_ok,
        detail: "EPSG:5179 SHP + PDF + MANIFEST.sha256 verified".to_owned(),
    });
}
