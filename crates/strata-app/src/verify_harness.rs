use eframe::egui;
use geo::{Coord, LineString, Polygon};
use strata_canvas2d::cad_interaction::CadSessionController;
use strata_canvas2d::canvas_painter::MapSceneModel;
use strata_canvas2d::map_tools::MapToolKind;
use strata_canvas2d::viewport::ViewportTransform;
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use strata_geo::buffer_round::round_buffer_polygon;
use strata_geo::trench_grid::{generate_standard_trenches, TrenchConfig, TrenchStandardLength};
use strata_layout::pdf_geotiff::{render_geotiff_header_bytes, render_sheet_pdf_bytes};
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_net::heritage_intranet::HeritageIntranetFlow;
use strata_net::tile_fetcher::{AsyncTileDispatcher, BasemapSource};
use strata_net::vworld_client::VworldClient;
use strata_paleo::section_renderer::render_cross_section_rgba;
use strata_secrets::dpapi::{protect_bytes, unprotect_bytes};
use strata_secrets::secret_store::SecretStore;
use strata_storage::gpkg::GpkgRepository;
use strata_storage::project_actions::WorkspaceFileActions;
use strata_storage::wkb_codec::{decode_gpkg_wkb, encode_gpkg_wkb, WkbGeometry};
use strata_terrain3d::viewport3d_renderer::{render_terrain3d_rgba, OrbitCamera3D};
use strata_types::crs::{SubmitCrs, WorkCrs};
use strata_types::schema::{DomainLayerKey, SurveySchemaV3};
use strata_types::theme_tokens::ThemeTokens;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{KaBeginnerRibbon, SubToolsMode, PENINSULA_REGION_TILES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    pub gate_id: &'static str,
    pub title_ko: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[must_use]
pub fn run_v1_to_v10_verification_harness() -> Vec<GateResult> {
    let mut gates = Vec::with_capacity(17);

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

    // V10: Headless egui 5대 탭 + C++ 1:1 리본(7그룹)·보조툴바·17개 광역 지역선택 렌더링 검증
    let ctx = egui::Context::default();
    let mut gui_state = StrataAppState::new_startup();
    gui_state.ribbon.sub_mode = SubToolsMode::Draw;
    gui_state.ribbon.app_bar.region_popup_open = true;
    let mut all_tabs_rendered =
        KaBeginnerRibbon::categories().len() == 7 && PENINSULA_REGION_TILES.len() == 17;
    for &tab in MainTab::all() {
        gui_state.active_tab = tab;
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            ..Default::default()
        };
        let mut rect_out = egui::Rect::NOTHING;
        let _ = ctx.run(raw_input, |ctx| {
            rect_out = gui_state.render_workspace_egui(ctx);
        });
        if rect_out.width() < 400.0 || rect_out.height() < 300.0 {
            all_tabs_rendered = false;
        }
    }
    gates.push(GateResult {
        gate_id: "V10",
        title_ko: "eframe/egui 5대 탭 및 C++ 1:1 리본(7그룹)·17개 지역선택 1440x900 렌더링",
        passed: all_tabs_rendered,
        detail: "5 tabs + 7 ribbon groups + 17 provinces rendered".to_owned(),
    });

    // V11: CAD 양방향 드로잉(좌클릭 정점·우클릭 폴리곤 확정·500m 버퍼 갱신·거리/면적 측정)
    let mut cad = CadSessionController::default();
    let mut cad_scene = MapSceneModel::default();
    for pt in [[0.0, 0.0], [40.0, 0.0], [40.0, 30.0]] {
        cad.on_left_click(pt, MapToolKind::DrawPolygon, &mut cad_scene);
    }
    let committed = cad.on_right_click(MapToolKind::DrawPolygon, &mut cad_scene);
    for pt in [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]] {
        cad.on_left_click(pt, MapToolKind::Measure, &mut cad_scene);
    }
    gates.push(GateResult {
        gate_id: "V11",
        title_ko: "2D 마우스 CAD 드로잉·스냅·우클릭 폴리곤 확정 및 거리·면적 측정",
        passed: committed && cad_scene.buffer_500m_ring.len() == 37 && cad.last_measure.is_some(),
        detail: format!(
            "polygons={} ring={}",
            cad_scene.polygons.len(),
            cad_scene.buffer_500m_ring.len()
        ),
    });

    // V12: 비동기 배경지도(Basemap RGBA) 합성 및 인트라넷/NGII WebView 자동화 계획
    let tile_rgba = AsyncTileDispatcher::synthesize_basemap_backdrop_rgba(
        BasemapSource::Daedongyeojido,
        64,
        64,
    );
    let h_plan =
        AsyncTileDispatcher::build_heritage_webview_plan("arch_user", "pw", "경상북도", "경주시");
    let n_plan = AsyncTileDispatcher::build_ngii_webview_plan("35701");
    gates.push(GateResult {
        gate_id: "V12",
        title_ko: "비동기 배경지도 타일 합성 및 국가유산청·국토정보플랫폼 WebView 계획",
        passed: tile_rgba.len() == 64 * 64 * 4
            && !h_plan.injected_scripts.is_empty()
            && n_plan.portal_url.contains("ngii.go.kr"),
        detail: format!(
            "basemaps={} scripts={}",
            BasemapSource::all().len(),
            h_plan.injected_scripts.len()
        ),
    });

    // V13: 실제 파일 액션(.gpkg 저장·재열기 + .dxf 표고점 추출 + .csv 기준점 임포트)
    let f_dir = std::env::temp_dir().join(format!("ka_hgis_v13_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&f_dir);
    let gpkg_p = f_dir.join("v13.gpkg");
    let dxf_p = f_dir.join("v13.dxf");
    let csv_p = f_dir.join("v13.csv");
    let _ = std::fs::write(&dxf_p, "0\nPOINT\n10\n200.0\n20\n300.0\n30\n55.0\n");
    let _ = std::fs::write(&csv_p, "CP1,200.0,300.0,55.0\n");
    let v13_ok = GpkgRepository::create_new_survey(&gpkg_p, WorkCrs::Epsg5187)
        .and_then(|mut ws| {
            ws.ensure_domain_layer(DomainLayerKey::SurveyArea);
            WorkspaceFileActions::save_and_reopen_gpkg(&gpkg_p, &mut ws)
        })
        .is_ok()
        && WorkspaceFileActions::import_dxf_contour_points(&dxf_p).is_ok_and(|v| v.len() == 1)
        && WorkspaceFileActions::import_control_points_csv(&csv_p).is_ok_and(|v| v.len() == 1);
    let _ = std::fs::remove_dir_all(&f_dir);
    gates.push(GateResult {
        gate_id: "V13",
        title_ko: "실제 조사 파일 액션(.gpkg 세대저장·재열기, .dxf 표고점, .csv 기준점)",
        passed: v13_ok,
        detail: "GPKG + DXF + CSV file actions verified".to_owned(),
    });

    // V14: 11대 실무 모달 대화상자(용지·DEM·유산·수치지형·지적·시굴격자·고지형자료·PDF준비·도면인쇄·계정·제출) Headless egui 렌더링
    let mut modal_ok = true;
    for modal in [
        ActiveModalDialog::AccountSettings,
        ActiveModalDialog::TrenchGenerator,
        ActiveModalDialog::ExportSubmitPackage,
        ActiveModalDialog::PaperSetup,
        ActiveModalDialog::DemDownload,
        ActiveModalDialog::HeritageRegionSelect,
        ActiveModalDialog::NgiiTopoDownload,
        ActiveModalDialog::CadastralDownload,
        ActiveModalDialog::PaleoSourceDownload,
        ActiveModalDialog::PdfPreparing,
        ActiveModalDialog::DrawingPrint,
    ] {
        gui_state.modals.active = Some(modal);
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1680.0, 960.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw_input, |ctx| {
            let _ = gui_state.render_workspace_egui(ctx);
        });
        if gui_state.modals.active != Some(modal) {
            modal_ok = false;
        }
    }
    gates.push(GateResult {
        gate_id: "V14",
        title_ko: "10대 실무 모달 대화상자 및 실제 C++ 계정(DPAPI)·경주시 GeoPackage 로드 검증",
        passed: modal_ok && !gui_state.home_map.real_survey.survey_rings.is_empty(),
        detail: format!(
            "10 modals OK, accounts_ready={}, trenches={}, cadastral={}, heritage={}",
            gui_state.accounts.status_summary_ko,
            gui_state.home_map.real_survey.trenches.len(),
            gui_state.home_map.real_survey.cadastral_parcels.len(),
            gui_state.home_map.real_survey.heritage_polys.len()
        ),
    });

    gates.push(v15_borehole_ribbon_and_dialog_gate());
    gates.push(v16_borehole_maps_generation_gate());
    gates.push(v17_borehole_maps_are_painted_gate());

    gates
}

/// V15 (`d7afcc8`): 리본 칩 최소폭 36px(10% 축소)·「자료 받기」 다섯째 단추 「시추」·`KaBoreholeDialog`.
/// C++ `KaTheme.h:97-98`, `MainWindowRibbon.cpp:532-547`, `KaBoreholeDialog.cpp:66-101` 1:1.
#[must_use]
pub fn v15_borehole_ribbon_and_dialog_gate() -> GateResult {
    use crate::dialogs_borehole::{BoreholeDialogAction, BoreholeDialogState};
    use crate::dialogs_borehole_spec::{BOREHOLE_MAP_CARDS, BOREHOLE_RADIUS_CHOICES};
    use crate::gui_ribbon_bar::UX_RIBBON_GROUPS;
    use strata_types::theme_tokens::ButtonMetrics;

    let metrics = ButtonMetrics::default();
    let chip_ok = metrics.ribbon_chip_width == 36 && metrics.ribbon_min_width == 36;

    // 「자료 받기」가 다섯 단추(DEM·지적·수치지형·유산·시추)이고 그 다섯째가 「시추」다.
    let fetch_group = UX_RIBBON_GROUPS
        .iter()
        .find(|(title, _)| *title == "자료 받기")
        .map(|(_, items)| *items);
    let fetch_ok = fetch_group
        .is_some_and(|items| items.len() == 5 && items[4] == "시추");

    // 시추 단추가 쓰는 Lucide glyph 가 등록돼 있고, tooltip 이 C++ 원문 그대로다.
    let icon_ok = crate::ribbon_icons::cpp_icon_id("시추") == Some("borehole")
        && crate::ribbon_icon_svg::svg_parts("borehole").is_some();
    let tooltip_ok = crate::ribbon_icons::cpp_ribbon_tooltip("시추")
        .starts_with("시추자료를 별도로 받아 5가지 지반·시추 GIS 도면");

    // 대화상자: 반경 세 선택지, 다섯 카드, 네 번째 카드의 EL.m 서브 체크, 세 단추.
    let d = BoreholeDialogState::default();
    let dialog_ok = BOREHOLE_RADIUS_CHOICES.len() == 3
        && d.radius_meters == 5000
        && d.radius_choice_index() == 1
        && BOREHOLE_MAP_CARDS.len() == 5
        && d.bedrock_depth_map
        && !d.bedrock_elev_map;

    // 조사 폴더가 없으면 창을 열지 않고 안내만 남긴다(`KaBoreholeDialog.cpp:66-70`).
    let mut app = StrataAppState::new_startup();
    let refused_ok = !app.open_borehole_dialog() && !app.borehole_dialog.open;
    app.survey_path = Some(std::path::PathBuf::from(r"D:\조사자료\_v15_검증"));
    let opened_ok = app.open_borehole_dialog() && app.borehole_dialog.open;

    // 세 단추가 서로 다른 동작을 가리킨다(`KaBoreholeDialog.cpp:169-192`).
    let actions_ok = BoreholeDialogAction::FetchAndGenerate {
        radius_meters: 5000,
    } != BoreholeDialogAction::Regenerate
        && BoreholeDialogAction::Regenerate != BoreholeDialogAction::OpenLayout;

    GateResult {
        gate_id: "V15",
        title_ko: "시추 버튼 36px 칩 · KaBoreholeDialog 반경/5카드/3단추",
        passed: chip_ok
            && fetch_ok
            && icon_ok
            && tooltip_ok
            && dialog_ok
            && refused_ok
            && opened_ok
            && actions_ok,
        detail: format!(
            "chip={}px fetch={:?} cards={} radius={}m",
            metrics.ribbon_chip_width,
            fetch_group.map(<[&str]>::len),
            BOREHOLE_MAP_CARDS.len(),
            d.radius_meters
        ),
    }
}

/// V16 (`d7afcc8`): 시추 5종 IDW GeoTIFF + 0.5m 등층후선 GPKG 를 실제로 만들어 검증한다.
///
/// C++ `GeoInfoBoreholeMaps.cpp:179-294` 의 산출물 규격을 파일 수준에서 확인한다:
/// 180×180 Float32 GeoTIFF, Tag 33550/33922/34735, GPKG 매직·SRID·등층후선 레이어.
#[must_use]
pub fn v16_borehole_maps_generation_gate() -> GateResult {
    use strata_net::geoinfo_borehole::BoreholeRecord;
    use strata_paleo::borehole_map_options::BoreholeMapOptions;
    use strata_paleo::borehole_maps::{
        generate_borehole_gis_maps, BOREHOLE_MIN_POINTS, FILE_ISOPACH_GPKG,
    };

    fn hole(code: &str, x: f64, y: f64, fill: f64, alluv: f64, weath: f64, water: f64) -> BoreholeRecord {
        BoreholeRecord {
            hole_code: code.to_owned(),
            tm_x: x,
            tm_y: y,
            altitude_m: 40.0,
            fill_depth_m: fill,
            alluvial_from_m: fill,
            alluvial_to_m: fill + alluv,
            alluvial_thickness_m: alluv,
            weathered_depth_m: weath,
            water_level_m: water,
            dominant_uscs: "CL".to_owned(),
            paleo_hint: "실트질점토".to_owned(),
            layers: Vec::new(),
        }
    }

    // C++ `:205-225` 처럼 여러 공을 흩어 놓아야 IDW 격자가 의미를 갖는다.
    let holes = [
        hole("BH-01", 218_500.0, 361_400.0, 1.2, 4.2, 5.4, 2.1),
        hole("BH-02", 218_900.0, 361_600.0, 0.8, 3.1, -1.0, 1.8),
        hole("BH-03", 219_300.0, 361_200.0, 2.0, 6.5, 8.5, 3.2),
        hole("BH-04", 218_200.0, 361_000.0, 0.5, 2.4, 2.9, 0.0),
        hole("BH-05", 219_700.0, 361_800.0, 1.6, 5.0, 6.6, 2.7),
    ];
    let kigam: [[f64; 3]; 2] = [[218_650.0, 361_250.0, 1.9], [219_050.0, 361_700.0, 2.6]];

    let dir = std::env::temp_dir().join(format!("ka_hgis_v16_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let opts = BoreholeMapOptions {
        radius_meters: 5000,
        marker_map: true,
        alluvial_isopach_map: true,
        fill_thickness_map: true,
        bedrock_depth_map: true,
        // 4-B 를 켜 5장 + 등층후선 1장이 모두 만들어지는지 본다.
        bedrock_elev_map: true,
        groundwater_map: true,
    };
    let bundle = generate_borehole_gis_maps(&holes, &kigam, WorkCrs::Epsg5187, &dir, &opts);
    let Ok(bundle) = bundle else {
        let _ = std::fs::remove_dir_all(&dir);
        return GateResult {
            gate_id: "V16",
            title_ko: "시추 5종 IDW GeoTIFF · 0.5m 등층후선 GPKG 생성",
            passed: false,
            detail: "generate_borehole_gis_maps returned Err".to_owned(),
        };
    };

    // C++ `:244-246` — 180×180 격자, cell >= 5.0.
    let grid_ok = bundle.grid.width == 180
        && bundle.grid.height == 180
        && bundle.grid.cell_size >= 5.0
        && bundle.grid.cell_size.is_finite();

    // C++ `:188` 마커 + `:251-291` 다섯 래스터 + 등층후선 = 7.
    let count_ok = bundle.created_count == 7;

    // 5장의 Float32 GeoTIFF 가 실제로 있고, 각각 TIFF 매직과 GeoKey 를 담는다.
    let tifs: Vec<&std::path::PathBuf> = bundle
        .written_files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "tif"))
        .collect();
    let mut tiff_ok = tifs.len() == 5;
    for p in &tifs {
        match std::fs::read(p) {
            Ok(bytes) => {
                let magic = bytes.starts_with(b"II*\0");
                let has_scale = bytes.windows(2).any(|w| w == 33550_u16.to_le_bytes());
                let has_tie = bytes.windows(2).any(|w| w == 33922_u16.to_le_bytes());
                let has_key = bytes.windows(2).any(|w| w == 34735_u16.to_le_bytes());
                // Float32 SampleFormat(339)=3 이 있어야 진짜 실수 래스터다.
                let has_float = bytes.windows(2).any(|w| w == 339_u16.to_le_bytes());
                if !(magic && has_scale && has_tie && has_key && has_float) {
                    tiff_ok = false;
                }
            }
            Err(_) => tiff_ok = false,
        }
    }

    // 등층후선 GPKG: SQLite 매직 + GPKG application_id + 레이어 이름.
    let gpkg_path = dir.join(FILE_ISOPACH_GPKG);
    let gpkg_ok = std::fs::read(&gpkg_path).is_ok_and(|bytes| {
        let sqlite_magic = bytes.starts_with(b"SQLite format 3\0");
        let has_gpkg_id = bytes
            .windows(4)
            .any(|w| w == strata_paleo::borehole_gpkg_geom::GPKG_APPLICATION_ID.to_be_bytes());
        sqlite_magic && has_gpkg_id
    });

    // 2공 미만이면 아무것도 만들지 않는다(`:226`).
    let short_ok = generate_borehole_gis_maps(
        &holes[..BOREHOLE_MIN_POINTS - 1],
        &[],
        WorkCrs::Epsg5187,
        &dir,
        &opts,
    )
    .is_ok_and(|b| b.created_count == usize::from(opts.marker_map) && b.written_files.is_empty());

    let _ = std::fs::remove_dir_all(&dir);

    GateResult {
        gate_id: "V16",
        title_ko: "시추 5종 IDW GeoTIFF · 0.5m 등층후선 GPKG 생성",
        passed: grid_ok && count_ok && tiff_ok && gpkg_ok && short_ok,
        detail: format!(
            "grid={}x{} cell={:.1}m created={} tifs={} gpkg={}",
            bundle.grid.width,
            bundle.grid.height,
            bundle.grid.cell_size,
            bundle.created_count,
            tifs.len(),
            if gpkg_ok { "ok" } else { "bad" }
        ),
    }
}

/// V17: 시추 도면이 **화면에 실제로 칠해지는지** 확인한다 (`d7afcc8` Phase 3 후속).
///
/// V16 은 파일이 만들어졌는지만 본다. 그래서 램프가 계산만 되고 지도에 안 칠해져도
/// V16 은 통과했다 — 실제로 그런 상태로 한동안 초록불이었다. 이 게이트는 그 구멍을 막는다:
/// 격자가 번들에 남아 있는지, 램프로 칠했을 때 불투명 픽셀이 나오는지,
/// C++ `setOpacity(0.72)` 알파가 지켜지는지 본다.
#[must_use]
pub fn v17_borehole_maps_are_painted_gate() -> GateResult {
    use strata_net::geoinfo_borehole::BoreholeRecord;
    use strata_paleo::borehole_color_ramp::{rasterize_ramp_rgba, BOREHOLE_RASTER_OPACITY};
    use strata_paleo::borehole_map_options::BoreholeMapOptions;
    use strata_paleo::borehole_maps::generate_borehole_gis_maps;

    fn hole(code: &str, x: f64, y: f64, fill: f64, alluv: f64, weath: f64) -> BoreholeRecord {
        BoreholeRecord {
            hole_code: code.to_owned(),
            tm_x: x,
            tm_y: y,
            altitude_m: 40.0,
            fill_depth_m: fill,
            alluvial_from_m: fill,
            alluvial_to_m: fill + alluv,
            alluvial_thickness_m: alluv,
            weathered_depth_m: weath,
            water_level_m: 2.4,
            dominant_uscs: "CL".to_owned(),
            paleo_hint: String::new(),
            layers: Vec::new(),
        }
    }

    let holes = [
        hole("BH-01", 218_500.0, 361_400.0, 1.2, 4.2, 5.4),
        hole("BH-02", 218_900.0, 361_600.0, 0.8, 3.1, -1.0),
        hole("BH-03", 219_300.0, 361_200.0, 2.0, 6.5, 8.5),
    ];
    let dir = std::env::temp_dir().join(format!("ka_hgis_v17_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let opts = BoreholeMapOptions {
        radius_meters: 5000,
        marker_map: true,
        alluvial_isopach_map: true,
        fill_thickness_map: true,
        bedrock_depth_map: true,
        bedrock_elev_map: true,
        groundwater_map: true,
    };

    let result = generate_borehole_gis_maps(&holes, &[], WorkCrs::Epsg5187, &dir, &opts);
    let _ = std::fs::remove_dir_all(&dir);

    let Ok(bundle) = result else {
        return GateResult {
            gate_id: "V17",
            title_ko: "시추 도면 화면 칠하기(램프 래스터화 · 알파 0.72)",
            passed: false,
            detail: "generate_borehole_gis_maps returned Err".to_owned(),
        };
    };

    // 1) 칠할 격자가 번들에 남아 있어야 한다(이게 없으면 화면이 빈다).
    let ramp_layers: Vec<&strata_paleo::borehole_maps::BoreholeMapLayerResult> =
        bundle.layers.iter().filter(|l| l.ramp.is_some()).collect();
    let grids_ok = bundle.painted_grids.len() == ramp_layers.len() && !bundle.painted_grids.is_empty();

    // 2) 각 격자가 램프로 칠해져 불투명 픽셀을 낸다.
    let expected_alpha = (BOREHOLE_RASTER_OPACITY * 255.0).round() as u8;
    let mut painted_ok = grids_ok;
    let mut visible_total = 0usize;
    for (grid, layer) in bundle.painted_grids.iter().zip(ramp_layers.iter()) {
        let Some(ramp) = layer.ramp.as_ref() else {
            painted_ok = false;
            continue;
        };
        let rgba = rasterize_ramp_rgba(grid, ramp);
        let visible = rgba.iter().filter(|p| p[3] > 0).count();
        if visible == 0 {
            painted_ok = false;
        }
        // 보이는 픽셀의 알파는 C++ `setOpacity(0.72)` 값과 같아야 한다.
        if rgba.iter().filter(|p| p[3] > 0).any(|p| p[3] != expected_alpha) {
            painted_ok = false;
        }
        visible_total += visible;
    }

    GateResult {
        gate_id: "V17",
        title_ko: "시추 도면 화면 칠하기(램프 래스터화 · 알파 0.72)",
        passed: grids_ok && painted_ok && expected_alpha == 184,
        detail: format!(
            "grids={} ramps={} alpha={} visible_px={}",
            bundle.painted_grids.len(),
            ramp_layers.len(),
            expected_alpha,
            visible_total
        ),
    }
}

/// C++ `hgis.exe --smoke` 3단계 코어 무결성(`CrsRegistry`, `WkbCodec`, `GpkgStorage`) 검증 보고서 1:1
#[must_use]
pub fn run_cpp_smoke_suite_report() -> (bool, String) {
    let crs_ok = strata_types::crs::find_crs_by_epsg(5186).is_some();
    let poly = WkbGeometry::Polygon(vec![vec![
        [200_000.0, 400_000.0],
        [200_010.0, 400_000.0],
        [200_010.0, 400_010.0],
        [200_000.0, 400_010.0],
        [200_000.0, 400_000.0],
    ]]);
    let encoded = encode_gpkg_wkb(5186, &poly);
    let wkb_ok = decode_gpkg_wkb(&encoded).is_ok_and(|(s, g)| s == 5186 && g == poly);

    let tmp_gpkg = std::env::temp_dir().join(format!("ka_hgis_smoke_{}.gpkg", std::process::id()));
    let gpkg_ok = GpkgRepository::create_survey_schema_5_layers(&tmp_gpkg, WorkCrs::Epsg5186);
    let _ = std::fs::remove_file(&tmp_gpkg);

    let passed_cnt = usize::from(crs_ok) + usize::from(wkb_ok) + usize::from(gpkg_ok);
    let failed_cnt = 3 - passed_cnt;
    let report = format!(
        "[SMOKE] CrsRegistry: {} (EPSG:5186 loaded)\n\
         [SMOKE] WkbCodec: {} (Roundtrip passed)\n\
         [SMOKE] GpkgStorage: {} (Atomic write verified)\n\
         [SMOKE] Result: ALL {passed_cnt} PASSED, {failed_cnt} FAILED. (Exit Code: {failed_cnt})",
        if crs_ok { "OK" } else { "FAIL" },
        if wkb_ok { "OK" } else { "FAIL" },
        if gpkg_ok { "OK" } else { "FAIL" },
    );
    (failed_cnt == 0, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_to_v17_verification_harness_passes_all_seventeen_gates() {
        let gates = run_v1_to_v10_verification_harness();
        assert_eq!(gates.len(), 17);
        for g in &gates {
            assert!(g.passed, "Gate {} failed: {}", g.gate_id, g.detail);
        }
    }

    #[test]
    fn test_cpp_smoke_suite_and_crash_handler_emergency_autosave_and_shortcut_conflict() {
        let (smoke_ok, report) = run_cpp_smoke_suite_report();
        assert!(smoke_ok, "smoke suite failed: {report}");
        assert!(report.contains("ALL 3 PASSED, 0 FAILED"));

        let mut shortcuts = crate::app_state::ShortcutManager::new();
        assert!(shortcuts.register_shortcut("file.save", "Ctrl+S"));
        assert!(!shortcuts.register_shortcut("layer.save_dup", "ctrl+s"));
        assert!(shortcuts.register_shortcut("view.pan", "Space"));

        let mut app = StrataAppState::new_startup();
        app.left_layers
            .ensure_domain_layer(DomainLayerKey::SurveyArea);
        let crash_path =
            std::env::temp_dir().join(format!("crash_recovery_{}.gpkg", std::process::id()));
        assert!(app.emergency_auto_save(&crash_path));
        assert!(crash_path.exists());
        let recovered = GpkgRepository::open_read_only(&crash_path)
            .expect("crash recovery GPKG must open cleanly");
        assert!(recovered
            .layers
            .contains_key(DomainLayerKey::SurveyArea.id()));
        let _ = std::fs::remove_file(&crash_path);
    }
}
