//! 「가까운 선에 붙이기」 — 열어 둔 조사 표(저장 안 한 도형 포함)의 대상을 붙이고, 되돌릴 수 있다. 임시 폴더만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::{MainTab, StrataAppState};
use crate::real_survey::RealCadastralParcel;

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app
}

fn outer_ring(app: &StrataAppState) -> Vec<[f64; 2]> {
    let t = app.survey_tables.iter().find(|t| t.table == "survey_area").expect("table");
    let parts = wkt_to_parts(&t.layer.features.last().expect("feature").wkt_geom).expect("wkt");
    let WkbGeometry::Polygon(rings) = &parts[0] else { panic!("polygon") };
    rings[0].clone()
}

/// ported from tests/test_parcel_copy_keys.cpp:snapTraceShowsNearbyVerticesOnlyWhenCloseAndFillsSkippedVertex — 지도
/// 80 m 를 640 px(점당 0.125 m)로 보고, 도면 선 위에 대략 그린(저장 안 한) 조사구역을 붙이면 6점(빠진 사이 꼭짓점
/// (200010, 450001.2) 채움 + 닫는 점). 되돌리면 그린 그대로.
#[test]
fn just_drawn_area_snaps_to_the_line_and_undoes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = new_survey(dir.path());
    app.home_map.real_survey.cadastral_parcels.push(RealCadastralParcel {
        jibun: "도면".to_owned(),
        ring: vec![[200_000.0, 450_000.0], [200_010.0, 450_001.2], [200_020.0, 450_000.0]],
        center: [200_010.0, 450_000.0],
    });
    app.home_map.viewport.meters_per_pixel = 0.125;
    app.start_edit_survey_area();
    app.accept_survey_area_setup(); // 이어 그릴 조사구역이 없으면 「조사구역 그리기 설정」 창 → 「그리기 시작」
    for p in [[200_001.2, 449_999.1], [200_019.0, 450_000.8], [200_020.8, 450_019.2], [199_999.3, 450_020.6]] {
        app.sketch_press(p);
    }
    app.sketch_finish();
    let drawn = outer_ring(&app);

    assert_eq!(app.snap_shapes_to_nearest_lines(), 1);
    let snapped = outer_ring(&app);
    assert_eq!(snapped.len(), 6, "{snapped:?}");
    assert!(snapped.iter().any(|p| (p[0] - 200_010.0).abs() < 1e-6 && (p[1] - 450_001.2).abs() < 1e-6));
    assert!(app.status_left_message().starts_with("도형을 가장 가까운 선에 붙였습니다 (꼭짓점 "));
    assert!(app.is_dirty);

    app.undo_map_action();
    assert_eq!(outer_ring(&app), drawn);
}

/// 대상이 없으면 C++ 안내 글.
#[test]
fn nothing_to_snap_says_so() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = new_survey(dir.path());
    assert_eq!(app.snap_shapes_to_nearest_lines(), 0);
    assert_eq!(app.status_left_message(), "먼저 지적도·도면 위에 도형을 그리거나 도형선택으로 도형을 고르세요.");
}
