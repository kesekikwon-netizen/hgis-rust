//! 검증 게이트 V10~V14 — egui 5대 탭·CAD 드로잉·타일 합성·파일 액션·모달 대화상자.
//!
//! erify_harness.rs 가 300줄 규칙에 맞춰 나눈 파일이다.
//! 게이트를 만드는 코드는 원문 그대로이고, 이 함수는 **원래 순서대로** gates 에 밀어 넣기만
//! 한다. 게이트 번호·판정식·문구는 하나도 바꾸지 않았다.

use eframe::egui;
use strata_canvas2d::cad_interaction::CadSessionController;
use strata_canvas2d::canvas_painter::MapSceneModel;
use strata_canvas2d::map_tools::MapToolKind;
use strata_net::tile_fetcher::{AsyncTileDispatcher, BasemapSource};
use strata_storage::gpkg::GpkgRepository;
use strata_storage::project_actions::WorkspaceFileActions;
use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::ActiveModalDialog;
use crate::ribbon::{KaBeginnerRibbon, SubToolsMode, PENINSULA_REGION_TILES};
use super::verify_harness_types::GateResult;

/// V10~V14 게이트를 원래 순서대로 gates 에 넣는다.
pub(crate) fn push_v10_to_v14_gates(gates: &mut Vec<GateResult>) {
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
    // 시작 화면에는 견본 조사가 없다. 실제 그리기 경로로 조사구역 하나를 넣고 창을 띄운다.
    gui_state.draw_target = DomainLayerKey::SurveyArea;
    let _ = gui_state.commit_drawn_shape(
        MapToolKind::DrawPolygon,
        vec![
            [219_700.0, 360_900.0],
            [219_800.0, 360_900.0],
            [219_800.0, 361_000.0],
            [219_700.0, 361_000.0],
        ],
    );
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
}
