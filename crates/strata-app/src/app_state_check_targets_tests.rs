//! 검수 「위치 보기」 — demo 임시 사본만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_check_export::checklist_engine::CheckResult;
use strata_check_export::check_rules::CheckTarget;

use crate::app_state::{MainTab, StrataAppState};

fn demo_app(dir: &std::path::Path) -> StrataAppState {
    let gpkg = dir.join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &gpkg).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&gpkg);
    app
}

fn result_for(layer_id: &str, fid: i64) -> CheckResult {
    CheckResult {
        id: "R1".to_owned(),
        message_ko: "시험".to_owned(),
        targets: vec![CheckTarget { layer_id: layer_id.to_owned(), layer_name: "유구(면)".to_owned(), feature_id: fid, label: String::new() }],
        severity: "error".to_owned(),
        passed: false,
        fix_ko: String::new(),
        basis_ko: String::new(),
        action: String::new(),
        detail_ko: String::new(),
    }
}

/// KaSubmitFlow.cpp:191-240 — 그 도형의 바깥 상자(작업 좌표계)를 max(너비, 높이, 20 m) × 0.3 만큼 넓혀 보이고 골라 둔다.
#[test]
fn goes_to_the_target_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = demo_app(dir.path());
    let t = app.survey_tables.iter().find(|t| t.table == "feature_poly").expect("feature_poly");
    let fid = t.layer.features[0].fid;
    let mut one = t.clone();
    one.layer.features.retain(|f| f.fid == fid);
    let b = crate::survey_paint_layers::table_extent_in(&one, app.work_crs).expect("extent");
    app.go_to_check_targets(&result_for("feature_poly", fid));
    assert_eq!(app.active_tab, MainTab::Map);
    let c = app.home_map.viewport.center_xy;
    assert!((c[0] - (b[0] + b[2]) * 0.5).abs() < 1e-6 && (c[1] - (b[1] + b[3]) * 0.5).abs() < 1e-6, "{c:?} {b:?}");
    assert_eq!(app.home_map.selected_shape, Some(("feature_poly".to_owned(), fid)));
    assert!(app.home_map.draw_guide_message.as_deref().unwrap().contains("해당 도형 1개로 이동해 골라 두었습니다"));
}

/// 대상 레이어가 지도에 없으면 어디로도 옮기지 않고 알린다(예전엔 경주 기본점으로 갔다).
#[test]
fn missing_target_moves_nowhere() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = demo_app(dir.path());
    let before = app.home_map.viewport.center_xy;
    app.go_to_check_targets(&result_for("없는_표", 1));
    assert_eq!(app.home_map.viewport.center_xy, before);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("해당 도형을 지도에서 찾지 못했습니다. 「다시 검수」로 확인하세요."));
}
